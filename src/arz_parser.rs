use rayon::iter::Either;
use rayon::prelude::*;

use crate::byte_reader::{self, ByteReader};
use std::collections::HashMap;
use std::io::Error;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct ArzRecordHeader {
    string_index: u32,
    record_type: String,
    offset: u32,
    size_compressed: u32,
    size_decompressed: u32,
}

impl ArzRecordHeader {
    fn read(byte_reader: &mut ByteReader) -> Self {
        let string_index = byte_reader.read_u32();
        let str_len = byte_reader.read_u32();
        let record_type = byte_reader.read_str(str_len).to_string();

        Self {
            string_index,
            record_type,
            offset: byte_reader.read_u32(),
            size_compressed: byte_reader.read_u32(),
            size_decompressed: byte_reader.read_u32(),
        }
    }
}

// v3 of the header?
struct ArzArchiveHeader {
    unknown: u16, // Item Assistant code thinks this is the version check?
    version: u16,
    records_start: u32,
    #[allow(dead_code)]
    records_len: u32,
    records_count: u32,
    strings_start: u32,
    strings_size: u32,
}

impl ArzArchiveHeader {
    fn new(byte_vec: &mut ByteReader) -> Self {
        Self {
            unknown: byte_vec.read_u16(),
            version: byte_vec.read_u16(),
            records_start: byte_vec.read_u32(),
            records_len: byte_vec.read_u32(),
            records_count: byte_vec.read_u32(),
            strings_start: byte_vec.read_u32(),
            strings_size: byte_vec.read_u32(),
        }
    }
}

#[derive(Debug)]
struct EntryHeader {
    entry_type: u16,
    entry_count: u16,
    string_index: usize,
}

impl EntryHeader {
    fn read(byte_vec: &mut ByteReader) -> Self {
        Self {
            entry_type: byte_vec.read_u16(),
            entry_count: byte_vec.read_u16(),
            string_index: byte_vec.read_u32() as usize,
        }
    }
}

pub type Items = HashMap<String, ItemData>;
pub type Affixes = HashMap<String, AffixData>;

pub fn read_archive(path: &PathBuf) -> Result<(Items, Affixes), Error> {
    let mut byte_reader = ByteReader::from_file(path)?;

    let archive_header = ArzArchiveHeader::new(&mut byte_reader);

    // Asserts copied from Item Assistant example
    assert_eq!(archive_header.unknown, 2);
    assert_eq!(archive_header.version, 3);

    let record_headers = read_record_headers(&mut byte_reader, &archive_header);
    let strings = &read_strings(&byte_reader.bytes, &mut byte_reader.index, &archive_header);

    let (affixes, items): (Affixes, Items) = record_headers
        .par_iter()
        .filter_map(|record_header| {
            const ITEM_TYPES: [&str; 4] = ["Armor", "QuestItem", "Weapon", "OneShot_Scroll"];
            const IGNORED_ITEMS: [&str; 4] = [
                "ItemTransmuter",
                "ItemTransmuterSet",
                "ItemSetFormula",
                "ItemRandomSetFormula",
            ];
            const IGNORED_AFFIXES: [&str; 5] = [
                // Searching for unique affixes. Maybe later.
                "records/items/lootaffixes/prefixunique/",
                "records/items/lootaffixes/suffixunique/",
                "records/items/lootaffixes/completionrelics",
                "records/items/lootaffixes/completion",
                "records/items/lootaffixes/crafting",
            ];

            let record_name;

            match record_header.record_type.as_str() {
                // handle affix
                "LootRandomizer" => {
                    record_name = strings[record_header.string_index as usize].to_string();
                    if IGNORED_AFFIXES.iter().any(|s| record_name.starts_with(s)) {
                        return None;
                    }

                    let mut byte_reader = byte_reader.clone();
                    let data = decompress(&mut byte_reader, record_header);
                    let affix = parse_affix(record_header, data, strings);
                    Some((record_name, EntryType::Affix(affix)))
                }
                // handle items
                str if ITEM_TYPES.iter().any(|s| str.starts_with(s))
                    || str.starts_with("Item") && !IGNORED_ITEMS.iter().any(|s| str.starts_with(s)) =>
                {
                    record_name = strings[record_header.string_index as usize].to_string();

                    const INCLUDED_RECORDS: [&str; 3] = [
                        "records/creatures/npcs/npcgear/",
                        "records/storyelements/",
                        "records/endlessdungeon/",
                    ];
                    const IGNORE_LIST: [&str; 2] = ["records/items/enemygear/", "records/items/transmutes/"];

                    let handle_record = match record_name.as_str() {
                        str if INCLUDED_RECORDS.iter().any(|s| str.starts_with(s)) => true,
                        str if str.starts_with("records/items/") && !IGNORE_LIST.iter().any(|s| str.starts_with(s)) => {
                            true
                        }
                        _ => false,
                    };
                    if !handle_record {
                        return None;
                    }
                    let mut byte_reader = byte_reader.clone();
                    let data = decompress(&mut byte_reader, record_header);

                    let item = parse_item(record_header, data, &record_name, strings);
                    Some((record_name, EntryType::Item(item)))
                }
                _ => None,
            }
        })
        .partition_map(|(record_name, entry)| match entry {
            EntryType::Affix(affix_data) => Either::Left((record_name, affix_data)),
            EntryType::Item(item_data) => Either::Right((record_name, item_data)),
        });
    Ok((items, affixes))
}

// Used by the logic in parse_record(). Knowing the type of the record could be important later.
#[derive(Debug)]
#[allow(dead_code)]
enum EntryValue {
    Float(f32),
    Text(String),
    Int(u32),
}

#[derive(Debug)]
pub enum EntryType {
    Affix(AffixData),
    Item(ItemData),
}

#[derive(Debug)]
pub struct AffixData {
    pub tag_name: Option<String>,
    pub rarity: String,
    // pub level_req: Option<u32>,
    pub level_req: Option<u32>,
    pub name: OnceLock<String>, // The localized name of the affix is filled in later
}

#[derive(Debug)]
pub struct ItemData {
    pub record_name: String,
    pub tag_name: String,
    pub rarity: String,
    pub level_req: u32,
    pub name: OnceLock<String>, // The localized name of the item is filled in later
}

fn parse_item(
    record_header: &ArzRecordHeader,
    data: Vec<u8>,
    record_name: &str,
    strings: &[&str],
    // is_affix: bool,
) -> ItemData {
    let mut reader = ByteReader::from(data);

    let mut tag_name: Option<String> = None;
    let mut level_req = None;
    let mut rarity: Option<String> = None;

    'outer: while reader.index < record_header.size_decompressed as usize {
        let entry_header = EntryHeader::read(&mut reader);
        let entry_key = &strings[entry_header.string_index];

        match entry_header.entry_type {
            1 => {
                reader.index += size_of::<f32>() * entry_header.entry_count as usize;
            }
            2 => {
                let int = reader.read_u32();
                // Advance index by one less because we read the corresponding index in the string table
                reader.index += size_of::<u32>() * (entry_header.entry_count as usize - 1);
                match *entry_key {
                    // relics use "description" instead of "itemNameTag"
                    "itemNameTag" | "description" => {
                        tag_name = Some(strings[int as usize].to_string());

                        // Stop reading data once we found what we came for.
                        if rarity.is_some() && level_req.is_some() {
                            break 'outer;
                        }
                    }
                    "itemClassification" => {
                        rarity = Some(strings[int as usize].to_string());
                        if tag_name.is_some() && level_req.is_some() {
                            break 'outer;
                        }
                    }
                    _ => {}
                }
            }
            _num => {
                let int = reader.read_u32();
                reader.index += size_of::<u32>() * (entry_header.entry_count as usize - 1);
                if *entry_key == "levelRequirement" {
                    level_req = Some(int);
                    if tag_name.is_some() && rarity.is_some() {
                        break 'outer;
                    }
                }
            }
        };
    }

    ItemData {
        record_name: record_name.to_string(),
        tag_name: tag_name.unwrap_or_default(),
        rarity: rarity.unwrap_or_default(),
        level_req: level_req.unwrap_or_default(),
        name: Default::default(), // Initialized later
    }
}

fn parse_affix(record_header: &ArzRecordHeader, data: Vec<u8>, strings: &[&str]) -> AffixData {
    let mut reader = ByteReader::from(data);

    let mut tag_name: Option<String> = None; // used by most items and affixes
    let mut rarity: Option<String> = None;
    let mut level_req: Option<u32> = None;

    while reader.index < (record_header.size_decompressed as usize) {
        let entry_header = EntryHeader::read(&mut reader);
        let entry_key = &strings[entry_header.string_index];
        match entry_header.entry_type {
            1 => {
                reader.index += size_of::<f32>() * entry_header.entry_count as usize;
            }
            2 => {
                let int = reader.read_u32();
                reader.index += size_of::<u32>() * (entry_header.entry_count as usize - 1);
                match *entry_key {
                    "lootRandomizerName" => {
                        tag_name = Some(strings[int as usize].to_string());

                        if rarity.is_some() && level_req.is_some() {
                            break;
                        }
                    }
                    "itemClassification" => {
                        rarity = Some(strings[int as usize].to_string());

                        if tag_name.is_some() && level_req.is_some() {
                            break;
                        }
                    }
                    _str => {}
                }
            }
            _num => {
                let int = reader.read_u32();
                if *entry_key == "itemLevel" {
                    level_req = Some(int);

                    if tag_name.is_some() && rarity.is_some() {
                        break;
                    }
                }
                reader.index += size_of::<u32>() * (entry_header.entry_count as usize - 1);
            }
        };
    }

    AffixData {
        tag_name,
        rarity: rarity.unwrap_or_default(),
        level_req,
        name: Default::default(), // Initialized later
    }
}

fn decompress(byte_reader: &mut ByteReader, header: &ArzRecordHeader) -> Vec<u8> {
    byte_reader.index = header.offset as usize + 24;
    let slice = &byte_reader.read_n_bytes(header.size_compressed);
    lz4::block::decompress(slice, Some(header.size_decompressed.try_into().unwrap())).unwrap()
}

fn read_record_headers(byte_reader: &mut ByteReader, header: &ArzArchiveHeader) -> Vec<ArzRecordHeader> {
    let mut records = Vec::new();
    byte_reader.index = header.records_start as usize;
    for _ in 0..header.records_count {
        let record = ArzRecordHeader::read(byte_reader);
        records.push(record);
        byte_reader.index += 8;
    }
    records
}

fn read_strings<'a>(bytes: &'a [u8], index: &mut usize, header: &ArzArchiveHeader) -> Vec<&'a str> {
    let mut strings = Vec::new();
    *index = header.strings_start as usize;
    let end = (header.strings_start + header.strings_size) as usize;
    while *index < end {
        let count = byte_reader::read_u32(bytes, index);
        for _ in 0..count {
            let len = byte_reader::read_u32(bytes, index);
            let string = { byte_reader::read_str(bytes, index, len) };
            strings.push(string);
        }
    }
    strings
}
