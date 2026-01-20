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
use stash::Stash;

use std::io::Error;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), Error> {
    let mut args = std::env::args();
    let mut search_term = args.nth(1).unwrap_or_default().to_lowercase();
    for arg in args {
        search_term.push_str(&(" ".to_owned() + &arg.to_lowercase()));
    }

    let config = Arc::new(Config::new());

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

    // Read game database files in new threads and send them to "db_done_rx"
    let (db_done_tx, db_done_rx) = mpsc::channel();
    {
        let config = config.clone();
        thread::spawn(move || {
            let mut receivers = Vec::new();
            for path in config.get_databases() {
                let (db_thread_tx, db_thread_rx) = mpsc::channel();
                receivers.push(db_thread_rx);
                thread::spawn(move || {
                    let (items, affixes) = arz_parser::read_archive(&path).unwrap();
                    db_thread_tx.send((items, affixes)).unwrap();
                });
            }
            let mut tag_names = TagNames::default();
            for rcv in receivers {
                if let Ok((items, affixes)) = rcv.recv() {
                    // TODO this probably copies data around
                    tag_names.items.extend(items);
                    tag_names.affixes.extend(affixes);
                }
            }
            db_done_tx.send(tag_names).unwrap();
        });
    }

    // Read localization strings in new threads and send them to "localization_done_rx"
    let (localization_done_tx, localization_done_rx) = mpsc::channel();
    {
        let config = config.clone();
        thread::spawn(move || {
            let mut receivers = Vec::new();
            for path in config.get_localization_files() {
                let (loc_tx, loc_rx) = mpsc::channel();
                receivers.push(loc_rx);
                thread::spawn(move || {
                    let localization_data = arc_parser::read_archive(&path).unwrap();
                    loc_tx.send(localization_data).unwrap();
                });
            }
            let mut localization_data = item_search::LocalizationStrings::default();
            for rcv in receivers {
                if let Ok(map) = rcv.recv() {
                    localization_data.extend(map);
                }
            }
            localization_done_tx.send(localization_data).unwrap();
        });
    }

    // Read save files in new threads and send them to "saves_done_rx"
    let (saves_done_tx, saves_done_rx) = mpsc::channel();
    {
        let config = config.clone();
        thread::spawn(move || {
            let mut receivers = Vec::new();
            for save in config.get_save_files() {
                let (ci_tx, ci_rx) = mpsc::channel::<CharacterItems>();
                receivers.push(ci_rx);
                thread::spawn(move || match CharacterItems::read(&save) {
                    Ok(ci) => {
                        ci_tx.send(ci).unwrap();
                    }
                    Err(e) => {
                        println!("Unable to read save file {:?}: {e}", save);
                    }
                });
            }
            let mut all_char_items = Vec::new();
            for rx in receivers {
                if let Ok(ci) = rx.recv() {
                    all_char_items.push(ci);
                }
            }
            saves_done_tx.send(all_char_items).unwrap();
        });
    }

    // This causes the main thread to wait for the jobs
    let tag_names = db_done_rx.recv().unwrap();
    let localization_data = localization_done_rx.recv().unwrap();
    let all_char_items = saves_done_rx.recv().unwrap();

    let lookup = Arc::new(ItemLookup {
        search_term,
        localization_data,
        tag_names,
    });

    // receiver.recv() all of these to make sure the threads finish
    let mut search_receivers = Vec::new();

    let (softcore_stash_path, hardcore_stash_path) = config.get_stash_files();
    {
        let (tx, rx) = mpsc::channel();
        search_receivers.push(rx);
        let lookup = lookup.clone();
        thread::spawn(move || {
            if let Some(path) = softcore_stash_path {
                let softcore_stash = Stash::new(&path).unwrap();
                for (i, tab) in softcore_stash.tabs.iter().enumerate() {
                    for inventory_item in tab {
                        lookup.check_item(inventory_item, &format!("Softcore stash tab {}", i + 1));
                    }
                }
            }
            tx.send(true).unwrap();
        });
    }

    {
        let (tx, rx) = mpsc::channel();
        search_receivers.push(rx);
        let lookup = lookup.clone();
        thread::spawn(move || {
            if let Some(path) = hardcore_stash_path {
                let hardcore_stash = Stash::new(&path).unwrap();
                for (i, tab) in hardcore_stash.tabs.iter().enumerate() {
                    for inventory_item in tab {
                        lookup.check_item(inventory_item, &format!("Hardcore stash tab {}", i + 1));
                    }
                }
            }
            tx.send(true).unwrap();
        });
    }

    for char_items in all_char_items {
        let (tx, rx) = mpsc::channel();
        search_receivers.push(rx);
        let lookup = lookup.clone();
        thread::spawn(move || {
            for (i, bag) in char_items.inventory.bags.iter().enumerate() {
                for inventory_item in &bag.items {
                    lookup.check_item(inventory_item, &format!("{} bag {}", char_items.name, i + 1));
                }
            }

            for (i, tab) in char_items.stash.tabs.iter().enumerate() {
                for inventory_item in tab {
                    lookup.check_item(inventory_item, &format!("{} stash tab {}", char_items.name, i + 1));
                }
            }

            for inventory_item in char_items.inventory.equipment.iter() {
                lookup.check_item(&inventory_item.item, &format!("Equipped by {}", char_items.name));
            }

            for inventory_item in char_items.inventory.weapon_set_1.iter() {
                lookup.check_item(&inventory_item.item, &format!("Equipped by {}, weapon set 1", char_items.name));
            }

            for inventory_item in char_items.inventory.weapon_set_2.iter() {
                lookup.check_item(&inventory_item.item, &format!("Equipped by {}, weapon set 2", char_items.name));
            }

            tx.send(true).unwrap();
        });
    }

    // This makes sure all threads finish
    for rx in search_receivers {
        let _ = rx.recv().unwrap();
    }

    Ok(())
}
