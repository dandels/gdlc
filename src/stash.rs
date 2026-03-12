use crate::VersionNumber;

use super::decrypt::Decrypt;
use super::inventory_item::InventoryItem;
use std::io::Error;
use std::path::Path;

#[derive(Debug)]
pub struct StashItem {
    pub item: InventoryItem,
    #[allow(dead_code)]
    x_offset: u32,
    #[allow(dead_code)]
    y_offset: u32,
}

impl StashItem {
    pub fn read(decrypt: &mut Decrypt, version: &VersionNumber) -> Result<Self, Error> {
        let item = InventoryItem::read(decrypt, version)?;
        let x_offset = decrypt.read_int();
        let y_offset = decrypt.read_int();

        Ok(Self {
            item,
            x_offset,
            y_offset,
        })
    }
}

pub struct Stash {
    pub tabs: Vec<Vec<InventoryItem>>,
}

impl Stash {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut decrypt = Decrypt::new(path)?;
        let val = decrypt.read_int();
        assert_eq!(val, 2);
        let (block_pos, block) = decrypt.read_block_start();
        assert_eq!(block_pos, 18);
        let stash_version = decrypt.read_int();
        if !(stash_version == 5 || stash_version == 9) {
            println!("Unexpected stash version {stash_version}. Continuing, but something might break.");
        }
        assert_eq!(decrypt.next_int(), 0);
        let _str_mod = decrypt.read_str()?;
        if stash_version >= 5 {
            let _has_expansion1 = decrypt.read_bool(); // does this refer to AoM?
        }
        let stash_version = VersionNumber::Inventory(stash_version);

        let tabs_count = decrypt.read_int();
        let mut tabs = Vec::new();

        for _ in 0..tabs_count {
            tabs.push(read_stash_tab(&mut decrypt, &stash_version)?);
        }
        decrypt.read_block_end(&block);

        Ok(Self { tabs })
    }
}

pub fn read_stash_tab(decrypt: &mut Decrypt, version: &VersionNumber) -> Result<Vec<InventoryItem>, Error> {
    let mut items = Vec::new();
    let (block_start, tab_block) = decrypt.read_block_start();
    if block_start != 0 {
        println!("Expected stash tab block start to be 0...");
    }
    let _stash_width = decrypt.read_int();
    let _stash_height = decrypt.read_int();
    let item_count = decrypt.read_int();

    for _ in 0..item_count {
        let si = StashItem::read(decrypt, version)?;
        items.push(si.item);
    }
    decrypt.read_block_end(&tab_block);
    Ok(items)
}
