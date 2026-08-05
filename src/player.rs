use crate::inventory_item::Item;
use crate::inventory_item::StorageItem;
use crate::stash;

use super::decrypt::Decrypt;

use std::io::Error;
use std::path::Path;

pub struct PlayerStash {
    pub tabs: Vec<Vec<Item>>,
}
impl PlayerStash {
    fn read(decrypt: &mut Decrypt) -> Result<PlayerStash, Error> {
        let block_start = decrypt.read_block_start();
        assert!(block_start == 4, "Expected player stash block to start with 4, was {block_start}.");
        let stash_version = decrypt.read_int();
        assert!((6..=11).contains(&stash_version), "Expected player stash version to be between 6 and 11.");
        let num_tabs = decrypt.read_int();
        let mut tabs = Vec::with_capacity(num_tabs as usize);
        for _ in 0..num_tabs {
            // println!("stash tab {i}");
            tabs.push(stash::read_stash_tab(decrypt, stash_version)?);
        }
        decrypt.read_block_end();
        Ok(PlayerStash { tabs })
    }
}

const EQUIPMENT_SLOTS: usize = 12;
const WEAPON_SLOTS: usize = 2;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Inventory {
    num_bags: u32,
    pub bags: Vec<Bag>,
    pub equipment: [Item; EQUIPMENT_SLOTS],
    pub weapon_set_1: [Item; WEAPON_SLOTS],
    pub weapon_set_2: [Item; WEAPON_SLOTS],
    focused: u32,
    selected: u32,
    flag: u8,
    use_alternate: u8,
    alternate_1: u8,
    alternate_2: u8,
}

#[derive(Debug)]
pub struct InventoryEquipment {
    pub item: Item,
    #[allow(dead_code)]
    attached: u8,
}

impl InventoryEquipment {
    fn read(decrypt: &mut Decrypt, version: u32) -> Self {
        let ret = Self {
            item: Item::read(decrypt, version),
            attached: decrypt.read_byte(),
        };

        #[cfg(feature = "debug-bytes")]
        println!("attached is {}", ret.attached);

        ret
    }
}

impl AsRef<Item> for InventoryEquipment {
    fn as_ref(&self) -> &Item {
        &self.item
    }
}

#[derive(Default, Debug)]
pub struct Bag {
    pub items: Vec<Item>,
}

impl Bag {
    fn read(decrypt: &mut Decrypt, version: u32) -> Self {
        let block_start = decrypt.read_block_start();
        assert_eq!(block_start, 0, "Expected start of bag block to be zero");

        let _is_ok = decrypt.read_bool(); // IA uses this as an "ok" check..?
        // if !is_ok {
        //     println!("not ok!");
        // }

        let ret = Self {
            items: {
                let len = decrypt.read_int();

                #[cfg(feature = "debug-bytes")]
                println!("there are {len} items");

                let mut ret = Vec::with_capacity(len as usize);
                #[allow(unused_variables)]
                for i in 0..len {
                    #[cfg(feature = "debug-bytes")]
                    println!("item{i}, ver {version}");

                    let si = StorageItem::read(decrypt, version, crate::inventory_item::StorageType::Bag);
                    ret.push(si.item);
                }
                ret
            },
        };
        decrypt.read_block_end();
        ret
    }
}

impl Inventory {
    fn read(decrypt: &mut Decrypt) -> Option<Self> {
        let block_start = decrypt.read_block_start();
        assert_eq!(block_start, 3);
        let inventory_version = decrypt.read_int();
        assert!((4..=11).contains(&inventory_version), "Expected inventory version to be between 4 and 11.");
        let flag = decrypt.read_byte();
        // if flag != 0 {
        //     println!(
        //         "This byte was supposed to be 0, was {flag}. The file format might be wrong, leading to unexpected results."
        //     );
        // }

        // Special case for characters that have not entered the game yet
        if decrypt.blocks.last().unwrap().len == 5 {
            decrypt.read_block_end();
            return None;
        }

        let num_bags = decrypt.read_int();
        let focused = decrypt.read_int();
        let selected = decrypt.read_int();

        let mut bags = Vec::with_capacity(num_bags as usize);

        #[cfg(feature = "debug-bytes")]
        println!("there are {num_bags} bags");

        for _ in 0..num_bags {
            bags.push(Bag::read(decrypt, inventory_version));
        }

        #[cfg(feature = "debug-bytes")]
        println!("end of bags");
        let use_alternate = decrypt.read_byte(); // weapon swapping enabled?

        #[cfg(feature = "debug-bytes")]
        println!("use_alternate {use_alternate}\nequipment");

        let equipment = std::array::from_fn(|_| InventoryEquipment::read(decrypt, inventory_version)).map(|i| i.item);
        let alternate_1 = decrypt.read_byte();

        #[cfg(feature = "debug-bytes")]
        println!("alternate1 {alternate_1}\nweaponset1");
        let weapon_set_1 =
            std::array::from_fn(|_| InventoryEquipment::read(decrypt, inventory_version)).map(|i| i.item);
        let alternate_2 = decrypt.read_byte();

        #[cfg(feature = "debug-bytes")]
        println!("alternate2 {alternate_2}\nweaponset2");
        let weapon_set_2 =
            std::array::from_fn(|_| InventoryEquipment::read(decrypt, inventory_version)).map(|i| i.item);

        let ret = Self {
            num_bags,
            bags,
            equipment,
            weapon_set_1,
            weapon_set_2,
            focused,
            selected,
            flag,
            use_alternate,
            alternate_1,
            alternate_2,
        };
        decrypt.read_block_end();
        Some(ret)
    }
}

#[derive(Debug)]
struct PlayerHeader {
    name: String,
    _sex: bool, // which is which?
    _class_tag: String,
    _level: u32,
    _hardcore: bool, // reversed?
}

// this is here just so it's decrypted correctly, simply reading sizeof() bytes didn't work somewhy
#[allow(dead_code)]
struct CharacterInfo {
    is_in_main_quest: u8,
    has_been_in_game: u8,
    difficulty: u8,
    greatest_difficulty: u8,
    money: u32,
    greatest_survival_difficulty: u8,
    current_tribute: u32,
    compass_state: u8,
    skill_window_show_help: u8,
    weapon_swap_active: u8,
    weapon_swap_enabled: u8,
    texture: String,
    loot_filter_len: u32,
    loot_filter: Vec<u8>,
}

fn skip_block_with_size_n(decrypt: &mut Decrypt, expected_start: u32, version: u32, size: usize) {
    let start = decrypt.read_block_start();
    assert_eq!(start, expected_start);
    assert_eq!(decrypt.read_int(), version);
    for _ in 0..size {
        decrypt.read_byte();
    }
    decrypt.read_block_end();
}

impl CharacterInfo {
    fn read(decrypt: &mut Decrypt) -> Self {
        let start = decrypt.read_block_start();
        assert_eq!(start, 1);
        let version = decrypt.read_int();
        assert_eq!(version, 5); // version == 5

        let is_in_main_quest = decrypt.read_byte();
        let has_been_in_game = decrypt.read_byte();
        let difficulty = decrypt.read_byte();
        let greatest_difficulty = decrypt.read_byte();
        let money = decrypt.read_int();
        let greatest_survival_difficulty = decrypt.read_byte();
        let current_tribute = decrypt.read_int();
        let compass_state = decrypt.read_byte();
        let skill_window_show_help = decrypt.read_byte();
        let weapon_swap_active = decrypt.read_byte();
        let weapon_swap_enabled = decrypt.read_byte();
        let texture = decrypt.read_string().unwrap();
        let loot_filter_len = decrypt.read_int();

        let mut loot_filter = Vec::with_capacity(loot_filter_len as usize);
        for _ in 0..loot_filter_len {
            loot_filter.push(decrypt.read_byte());
        }

        let ret = Self {
            is_in_main_quest,
            has_been_in_game,
            difficulty,
            greatest_difficulty,
            money,
            greatest_survival_difficulty,
            current_tribute,
            compass_state,
            skill_window_show_help,
            weapon_swap_active,
            weapon_swap_enabled,
            texture,
            loot_filter_len,
            loot_filter,
        };

        decrypt.read_block_end();
        ret
    }
}

impl PlayerHeader {
    fn read(decrypt: &mut Decrypt) -> Self {
        Self {
            name: decrypt.read_wide_string(),
            _sex: decrypt.read_bool(),
            _class_tag: decrypt.read_string().unwrap(),
            _level: decrypt.read_int(),
            _hardcore: decrypt.read_bool(),
        }
    }
}

pub struct CharacterItems {
    pub name: String,
    pub inventory: Inventory,
    pub stash: PlayerStash,
}

impl CharacterItems {
    pub fn read(path: impl AsRef<Path>) -> Result<Option<Self>, Error> {
        let mut decrypt = Decrypt::new(path)?;
        assert_eq!(decrypt.read_int().to_le_bytes(), "GDCX".as_bytes());
        assert_eq!(decrypt.read_int(), 2); // start of transmission?
        let player_header = PlayerHeader::read(&mut decrypt);
        // println!("Player name {}", header.name);
        let _byte = decrypt.read_byte();
        // println!("byte is {byte}");
        assert_eq!(decrypt.next_int(), 0); // end of block?
        let version = decrypt.read_int();
        assert_eq!(version, 8);

        let mut uid_buf: [u8; 16] = [0; 16];
        for byte in uid_buf.iter_mut() {
            *byte = decrypt.read_byte();
        }
        let _char_info = CharacterInfo::read(&mut decrypt);
        skip_block_with_size_n(&mut decrypt, 2, 8, 44); // skip character bio
        let inventory = Inventory::read(&mut decrypt);
        if inventory.is_none() {
            return Ok(None); // Character not logged in yet? In that case Stash is also always empty
        }
        let stash = PlayerStash::read(&mut decrypt).unwrap();

        Ok(Some(Self {
            name: player_header.name,
            inventory: inventory.unwrap(),
            stash,
        }))
    }
}
