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

use std::collections::HashMap;
use std::io::Error;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use crate::arz_parser::Affixes;
use crate::arz_parser::Items;
use crate::item_search::LocalizationStrings;

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

    let now = Instant::now();

    let lookup: OnceLock<ItemLookup> = OnceLock::new();

    thread::scope(|s| {
        let config = &config;
        let lookup = &lookup;
        s.spawn(move || {
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
                    let now = Instant::now();
                    let (items, affixes) = arz_parser::read_archive(&path).unwrap();
                    db_tx.send(DbData::GameData((items, affixes))).unwrap();
                    println!("game data took {:.2?}", now.elapsed());
                });
            }

            for path in config.get_localization_files() {
                msg_count += 1;
                let db_tx = db_tx.clone();
                // localization_receivers.push(loc_rx);
                s.spawn(move || {
                    let now = Instant::now();
                    let localization_data = arc_parser::read_archive(&path).unwrap();
                    db_tx.send(DbData::LocalizationData(localization_data)).unwrap();
                });
            }
            // println!("localization done");

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

            // ... and once all the data is received we assign it to the OnceLock
            lookup
                .set(ItemLookup {
                    search_term,
                    localization_data,
                    tag_names,
                })
                .unwrap();
            println!("db done");

            println!("data mapping took {:.2?}", now.elapsed());
        });

        s.spawn(move || {
            for save in config.get_save_files() {
                s.spawn(move || match CharacterItems::read(&save) {
                    Ok(ci) => {
                        lookup_char_items(ci, lookup.wait());
                    }
                    Err(e) => {
                        println!("Unable to read save file {:?}: {e}", save);
                    }
                });
            }
        });

        let (softcore_stash_path, hardcore_stash_path) = config.get_stash_files();
        s.spawn(move || {
            if let Some(path) = softcore_stash_path {
                let softcore_stash = Stash::new(&path).unwrap();
                for (i, tab) in softcore_stash.tabs.into_iter().enumerate() {
                    s.spawn(move || {
                        for inventory_item in tab {
                            lookup.wait().check_item(&inventory_item, &format!("Softcore stash tab {}", i + 1));
                        }
                    });
                }
            }
        });

        s.spawn(move || {
            if let Some(path) = hardcore_stash_path {
                let hardcore_stash = Stash::new(&path).unwrap();
                for (i, tab) in hardcore_stash.tabs.iter().enumerate() {
                    for inventory_item in tab {
                        lookup.wait().check_item(inventory_item, &format!("Hardcore stash tab {}", i + 1));
                    }
                }
            }
        });
    });

    Ok(())
}

fn lookup_char_items(items: CharacterItems, lookup: &ItemLookup) {
    let CharacterItems {
        ref name,
        inventory,
        stash,
    } = items;
    thread::scope(|s| {
        for (i, bag) in inventory.bags.iter().enumerate() {
            s.spawn(move || {
                for inventory_item in &bag.items {
                    lookup.check_item(inventory_item, &format!("{} bag {}", &name, i + 1));
                }
            });
        }

        s.spawn(move || {
            for (i, stash_tab) in stash.tabs.iter().enumerate() {
                for inventory_item in stash_tab {
                    lookup.check_item(inventory_item, &format!("{} stash tab {}", &name, i + 1));
                }
            }
        });

        for inventory_item in inventory.equipment.iter() {
            lookup.check_item(&inventory_item.item, &format!("Equipped by {}", &name));
        }

        for inventory_item in inventory.weapon_set_1.iter() {
            lookup.check_item(&inventory_item.item, &format!("Equipped by {}, weapon set 1", &name));
        }

        for inventory_item in inventory.weapon_set_2.iter() {
            lookup.check_item(&inventory_item.item, &format!("Equipped by {}, weapon set 2", &name));
        }
    });
}
