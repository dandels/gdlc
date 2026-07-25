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
    pub fn read(decrypt: &mut Decrypt, version: u32) -> Self {
        Self {
            item: InventoryItem::read(decrypt, version),
            x_offset: decrypt.read_int(),
            y_offset: decrypt.read_int(),
        }
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
        let (block_start, block_end) = decrypt.read_block_start();
        assert_eq!(block_start, 18);
        let stash_version = decrypt.read_int();
        assert!((5..=11).contains(&stash_version), "Expected stash version to be between 5 and 11.");
        assert_eq!(decrypt.next_int(), 0);
        let _str_mod = decrypt.read_str().unwrap();

        if stash_version >= 5 {
            let _has_expansion1 = decrypt.read_bool(); // does this refer to AoM?
        }

        let tabs_count = decrypt.read_int();
        let mut tabs = Vec::new();

        for _ in 0..tabs_count {
            tabs.push(read_stash_tab(&mut decrypt, stash_version)?);
        }

        decrypt.read_block_end(&block_end);

        Ok(Self { tabs })
    }
}

pub fn read_stash_tab(decrypt: &mut Decrypt, version: u32) -> Result<Vec<InventoryItem>, Error> {
    let mut items = Vec::new();
    let (block_start, block_end) = decrypt.read_block_start();
    if block_start != 0 {
        println!("Expected stash tab block start to be 0...");
    }
    let _stash_width = decrypt.read_int();
    let _stash_height = decrypt.read_int();
    let item_count = decrypt.read_int();

    for _ in 0..item_count {
        let si = StashItem::read(decrypt, version);
        items.push(si.item);
    }
    if version >= 9 {
        let _border_index = decrypt.read_int();
        let _border_color_index = decrypt.read_int();
        let _symbol_index = decrypt.read_int();
        let _symbol_color_index = decrypt.read_int();
        let _button_name = decrypt.read_wide_string().unwrap();
    }
    decrypt.read_block_end(&block_end);
    Ok(items)
}
