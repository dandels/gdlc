mod arc_parser;
mod arz_parser;
mod byte_reader;
mod config;
mod decrypt;
mod inventory_item;
mod item_search;
mod player;
mod stash;

use byte_reader::ByteReader;
use config::Config;
use item_search::ItemLookup;
use item_search::TagNames;
use player::CharacterItems;
use rayon::prelude::*;
use stash::Stash;

use std::collections::HashMap;
use std::io::Error;
use std::io::Write;
use std::io::stdin;
use std::io::stdout;
use std::sync::mpsc;
use std::thread;

use crate::arz_parser::Affixes;
use crate::arz_parser::Items;
use crate::inventory_item::InventoryItem;
use crate::item_search::LocalizationStrings;

fn main() -> Result<(), Error> {
    let config = Config::new();

    if config.installation_dir().is_none() {
        println!("The game installation dir needs to be configured.");
        return Ok(());
    }

    if config.save_dir().is_none() {
        println!("The save dir needs to be configured.");
        return Ok(());
    }

    if let Some(install_dir) = config.installation_dir()
        && !install_dir.exists()
    {
        println!("The configured installation directory does not exist: {:?}", install_dir);
        return Ok(());
    }

    if let Some(save_dir) = config.save_dir()
        && !save_dir.exists()
    {
        println!("The configured save directory does not exist: {:?}", save_dir);
        return Ok(());
    }

    let mut args = std::env::args();

    // read input in a loop if invoked with -i
    let first_arg = args.nth(1);
    let should_loop = first_arg == Some("-i".to_owned());

    let mut search_term = {
        if should_loop {
            String::new()
        } else {
            first_arg.unwrap_or_default()
        }
    };
    for arg in args {
        search_term.push_str(&(" ".to_owned() + &arg.to_lowercase()));
    }
    let initial_search_term = search_term.clone();
    let (mut lookup, owned_items) = thread::scope(|s| {
        let config = &config;
        let lookup_thread = s.spawn(move || {
            enum DbData {
                GameData((Items, Affixes)),
                LocalizationData(HashMap<String, String>),
            }
            let mut msg_count = 0;
            let (db_tx, db_rx) = mpsc::channel();
            for path in config.get_databases() {
                msg_count += 1;
                let db_tx = db_tx.clone();
                s.spawn(move || {
                    let (items, affixes) = arz_parser::read_archive(&path).unwrap();
                    db_tx.send(DbData::GameData((items, affixes))).unwrap();
                });
            }

            for path in config.get_localization_files() {
                msg_count += 1;
                let db_tx = db_tx.clone();
                s.spawn(move || {
                    let localization_data = arc_parser::read_archive(&path).unwrap();
                    db_tx.send(DbData::LocalizationData(localization_data)).unwrap();
                });
            }

            let mut tag_names = TagNames::default();
            let mut localization_data = LocalizationStrings::default();

            // Fill TagNames and LocalizationStrings at the rate which the data comes in...
            for _ in 0..msg_count {
                match db_rx.recv().unwrap() {
                    DbData::GameData((items, affixes)) => {
                        tag_names.items.extend(items);
                        tag_names.affixes.extend(affixes);
                    }
                    DbData::LocalizationData(map) => {
                        localization_data.extend(map);
                    }
                }
            }

            // ... and once all the data is received we return it
            ItemLookup {
                search_term: initial_search_term,
                localization_data,
                tag_names,
            }
        });
        let mut stash_item_threads = Vec::new();

        let char_items_thread = s.spawn(|| {
            config.get_save_files().into_par_iter().filter_map(|save| match CharacterItems::read(&save) {
                Ok(ci) => Some(search_pairs_for_character(ci)),
                Err(e) => {
                    println!("Unable to read save file {:?}: {e}", save);
                    None
                }
            })
        });

        let (softcore_stash_path, hardcore_stash_path) = config.get_stash_files();
        stash_item_threads.push(s.spawn(|| {
            if let Some(path) = softcore_stash_path {
                let softcore_stash = Stash::new(&path).unwrap();
                let vec: Vec<(String, Vec<InventoryItem>)> = softcore_stash
                    .tabs
                    .into_iter()
                    .enumerate()
                    .map(|(i, tabitems)| (format!("Softcore stash tab {}", i + 1), tabitems))
                    .collect();
                Some(vec)
            } else {
                None
            }
        }));

        stash_item_threads.push(s.spawn(|| {
            if let Some(path) = hardcore_stash_path {
                let hardcore_stash = Stash::new(&path).unwrap();
                let vec: Vec<(String, Vec<InventoryItem>)> = hardcore_stash
                    .tabs
                    .into_iter()
                    .enumerate()
                    .map(|(i, tabitems)| (format!("Hardcore stash tab {}", i + 1), tabitems))
                    .collect();
                Some(vec)
            } else {
                None
            }
        }));

        let mut owned_items: Vec<(String, Vec<InventoryItem>)> = stash_item_threads
            .into_iter()
            .filter_map(|t| t.join().ok()?)
            .fold(Vec::new(), |mut items, mut stash_tabs| {
                items.append(&mut stash_tabs);
                items
            });

        owned_items.par_extend(char_items_thread.join().unwrap().flatten_iter());
        let lookup = lookup_thread.join().unwrap();
        (lookup, owned_items)
    });

    if should_loop {
        const PROMPT: &str = "\nEnter search term: ";
        if !search_term.is_empty() {
            do_search(&owned_items, &lookup);
            print!("{PROMPT}");
        } else {
            // Don't print the newline the first time
            print!("{}", PROMPT.split_at(1).1);
        }

        if stdout().flush().is_err() {
            return Ok(());
        }

        let lines = stdin().lines();
        for line in lines {
            match line {
                Ok(mut line) => {
                    line.pop(); // remove \n from line
                    lookup.search_term = line;
                    do_search(&owned_items, &lookup);
                    print!("{PROMPT}");
                    if stdout().flush().is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    } else {
        do_search(&owned_items, &lookup);
    }

    Ok(())
}

fn do_search(owned_items: &Vec<(String, Vec<InventoryItem>)>, lookup: &ItemLookup) {
    owned_items.par_iter().for_each(|(item_source, items)| {
        lookup.check_items(items, item_source);
    });
}

fn search_pairs_for_character(items: CharacterItems) -> Vec<(String, Vec<InventoryItem>)> {
    let CharacterItems { name, inventory, stash } = items;

    let mut searches: Vec<(String, Vec<InventoryItem>)> = vec![
        (format!("Equipped by {}", &name), inventory.equipment.to_vec()),
        (format!("Equipped by {}, weapon set 1", &name), inventory.weapon_set_1.to_vec()),
        (format!("Equipped by {}, weapon set 2", &name), inventory.weapon_set_2.to_vec()),
    ];

    for (i, bag) in inventory.bags.into_iter().enumerate() {
        searches.push((format!("{} bag {}", &name, i + 1), bag.items));
    }

    for (i, stash_tab) in stash.tabs.into_iter().enumerate() {
        searches.push((format!("{} stash tab {}", &name, i + 1), stash_tab));
    }
    searches
}
