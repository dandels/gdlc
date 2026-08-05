use crate::inventory_item::{StorageItem, StorageType};

use super::decrypt::Decrypt;
use super::inventory_item::Item;
use std::io::Error;
use std::path::Path;

pub struct Stash {
    pub tabs: Vec<Vec<Item>>,
}

impl Stash {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut decrypt = Decrypt::new(path)?;
        let val = decrypt.read_int();
        assert_eq!(val, 2);
        let block_start = decrypt.read_block_start();
        assert_eq!(block_start, 18);
        let stash_version = decrypt.read_int();
        assert!((5..=11).contains(&stash_version), "Expected stash version to be between 5 and 11.");
        assert_eq!(decrypt.next_int(), 0);
        let _str_mod = decrypt.read_string();

        if stash_version >= 5 {
            let _has_expansion1 = decrypt.read_bool(); // does this refer to AoM?
        }

        let tabs_count = decrypt.read_int();
        let mut tabs = Vec::new();

        #[cfg(feature = "debug-bytes")]
        println!("stash ver {stash_version}");
        for _ in 0..tabs_count {
            tabs.push(read_stash_tab(&mut decrypt, stash_version)?);
        }

        decrypt.read_block_end();

        Ok(Self { tabs })
    }
}

pub fn read_stash_tab(decrypt: &mut Decrypt, version: u32) -> Result<Vec<Item>, Error> {
    let mut items = Vec::new();
    let block_start = decrypt.read_block_start();
    if block_start != 0 {
        println!("Expected stash tab block start to be 0...");
    }
    let _stash_width = decrypt.read_int();
    let _stash_height = decrypt.read_int();
    let item_count = decrypt.read_int();

    #[allow(unused_variables)]
    for i in 0..item_count {
        #[cfg(feature = "debug-bytes")]
        println!("item {i}, ver {version}");
        let si = StorageItem::read(decrypt, version, StorageType::Stash);
        items.push(si.item);
    }
    if version >= 9 {
        let _border_index = decrypt.read_int();
        let _border_color_index = decrypt.read_int();
        let _symbol_index = decrypt.read_int();
        let _symbol_color_index = decrypt.read_int();
        let _button_name = decrypt.read_wide_string();
    }
    decrypt.read_block_end();
    Ok(items)
}
