use crate::byte_reader::{self, ByteReader};
use std::collections::HashMap;
use std::io::Error;
use std::path::PathBuf;
// use std::sync::Arc;
// use std::sync::{RwLock, mpsc};
use std::sync::{OnceLock, mpsc};
use std::thread;

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
    string_index: u32,
}

impl EntryHeader {
    fn read(byte_vec: &mut ByteReader) -> Self {
        Self {
            entry_type: byte_vec.read_u16(),
            entry_count: byte_vec.read_u16(),
            string_index: byte_vec.read_u32(),
        }
    }
}

type Items = HashMap<String, ItemData>;
type Affixes = HashMap<String, AffixData>;

pub fn read_archive(path: &PathBuf) -> Result<(Items, Affixes), Error> {
    let mut byte_reader = ByteReader::from_file(path)?;

    let archive_header = ArzArchiveHeader::new(&mut byte_reader);

    // Asserts copied from Item Assistant example
    assert_eq!(archive_header.unknown, 2);
    assert_eq!(archive_header.version, 3);

    let record_headers = read_record_headers(&mut byte_reader, &archive_header);
    let strings = read_strings(&byte_reader.bytes, &mut byte_reader.index, &archive_header);

    let (tx, rx) = mpsc::channel();

    let mut items = Items::new();
    let mut affixes = Affixes::new();

    let mut thread_count = 0;
    thread::scope(|s| {
        // let chunk_size = record_headers.len() % 10;
        // for thread_work in record_headers.split(chunk_size);
        'header_loop: for record_header in record_headers {
            let record_name = strings[record_header.string_index as usize].to_string();
            // Uncomment to debug why something is not getting properly read
            // note for debugging: record_type.is_empty() also yields values
            //let catch = "records/items/crafting/blueprints/other/craft_potion_royaljellyointment.dbr";
            //if record_name == catch {
            //    println!("{record_name}: {:?}", record_header.record_type);
            //}

            if record_header.record_type.starts_with("Armor")
            || record_header.record_type.starts_with("Item")
            || record_header.record_type.starts_with("QuestItem")
            || record_header.record_type.starts_with("Weapon")
            || record_header.record_type.starts_with("OneShot_Scroll")
            // starts_with() would also match "LootRandomizerTable"
            || record_header.record_type == "LootRandomizer"
            {
                if record_header.record_type.starts_with("Item") {
                    let ignore_list = [
                        "ItemTransmuter",
                        "ItemTransmuterSet",
                        "ItemSetFormula",
                        "ItemRandomSetFormula",
                    ];
                    for ign in ignore_list {
                        if record_header.record_type.starts_with(ign) {
                            continue 'header_loop;
                        }
                    }

                    //println!("{}", record_header.record_type);
                }
                if record_name.starts_with("records/items/")
                    || record_name.starts_with("records/creatures/npcs/npcgear/")
                    || record_name.starts_with("records/storyelements/")
                    || record_name.starts_with("records/endlessdungeon/")
                {
                    //println!("record type {}", record_header.record_type);
                    let ignore_list = [
                        "records/items/enemygear/",
                        "records/items/transmutes/",
                        // Searching for unique affixes. Maybe later.
                        "records/items/lootaffixes/prefixunique/",
                        "records/items/lootaffixes/suffixunique/",
                        "records/items/lootaffixes/completionrelics",
                        "records/items/lootaffixes/completion",
                        "records/items/lootaffixes/crafting",
                    ];
                    for ign in ignore_list {
                        if record_name.starts_with(ign) {
                            continue 'header_loop;
                        }
                    }

                    thread_count += 1;
                    let strings = &strings;
                    // let record_header = record_header.clone();

                    let tx = &tx;
                    // let items = &items;
                    // let affixes = &affixes;
                    // TODO this spawns needlessly many threads
                    // let record_name = record_name.clone();
                    let mut byte_reader = byte_reader.clone();
                    s.spawn(move || {
                        let data = decompress(&mut byte_reader, &record_header);
                        let is_affix = record_header.record_type == "LootRandomizer";
                        if is_affix {
                            let affix = parse_affix(&record_header, data, strings);
                            tx.send((record_name, EntryType::Affix(affix))).unwrap();
                        } else {
                            // TODO do item lvls still need to be fixed inside here?
                            let item = parse_item(&record_header, data, &record_name, strings);
                            tx.send((record_name, EntryType::Item(item))).unwrap();
                        }
                    });
                }
            }
        }
    });

    for _ in 0..thread_count {
        let (record_name, entry) = rx.recv().unwrap();
        match entry {
            EntryType::Affix(data) => {
                affixes.insert(record_name, data);
            }
            EntryType::Item(data) => {
                items.insert(record_name, data);
            }
        }
    }
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
    pub name: OnceLock<String>, // The localized name of the affix is filled in later
    pub level_req: Option<u32>,
}

#[derive(Debug)]
pub struct ItemData {
    pub record_name: String,
    pub tag_name: String,
    pub rarity: String,
    pub level_req: u32,
}

fn parse_item(
    record_header: &ArzRecordHeader,
    data: Vec<u8>,
    record_name: &str,
    strings: &[&str],
    // is_affix: bool,
) -> ItemData {
    let mut reader = ByteReader::from_vec(data);

    let mut tag_name: Option<String> = None; // used by most items and affixes
    let mut description: Option<String> = None; // fallback for relics that don't have itemNameTag
    let mut rarity: Option<String> = None;
    let mut level_req: Option<u32> = None;

    //println!("Processing record: {record_name}");

    let mut i = 0;
    'outer: while i < record_header.size_decompressed / 4 {
        let entry_header = EntryHeader::read(&mut reader);
        i += 2 + entry_header.entry_count as u32;
        let entry_key = &strings[entry_header.string_index as usize];
        //println!("entry key {entry_key}");
        for _ in 0..entry_header.entry_count {
            match entry_header.entry_type {
                1 => {
                    let _ = reader.read_f32();
                }
                2 => {
                    let int = reader.read_u32();
                    let value = &strings[int as usize];
                    // if value == "Mythical" {
                    // println!("{record_name} {entry_key}: {value}");
                    // }
                    match *entry_key {
                        "itemNameTag" => {
                            tag_name = Some(value.to_string());
                        }
                        "itemClassification" => {
                            rarity = Some(value.to_string());
                        }
                        "description" => {
                            description = Some(value.to_string());
                        }
                        _ => {
                            // println!("Field for item was {other}");
                        }
                    }
                    // EntryValue::Text(value.to_string())
                }
                _ => {
                    let int = reader.read_u32();
                    //Seems like the "levelRequirement" field isn't useful..?
                    if *entry_key == "levelRequirement" {
                        level_req = Some(int);
                        // println!("Affix {record_name} had req {int}");
                    }
                    // if *entry_key == "itemLevel" {
                    //     level_req = Some(int);
                    //     println!("Affix {record_name} item lvl was {int}");
                    // }
                    // EntryValue::Int(int)
                }
            };

            // Stop reading data once we found what we came for.
            // We only need these fields for items (when !is_affix)
            if (tag_name.is_some() || description.is_some()) && level_req.is_some() && rarity.is_some() {
                break 'outer;
            }
            // println!("{entry_key}, {entry_value}")
        }
    }

    ItemData {
        record_name: record_name.to_string(),
        tag_name: tag_name.unwrap_or(description.unwrap_or_default()),
        rarity: rarity.unwrap_or_default(),
        level_req: level_req.unwrap_or_default(),
    }
}

fn parse_affix(record_header: &ArzRecordHeader, data: Vec<u8>, strings: &[&str]) -> AffixData {
    let mut reader = ByteReader::from_vec(data);
    let mut tag_name: Option<String> = None; // used by most items and affixes
    let mut rarity: Option<String> = None;
    let mut level_req: Option<u32> = None;

    let mut i = 0;
    'outer: while i < record_header.size_decompressed / 4 {
        let entry_header = EntryHeader::read(&mut reader);
        i += 2 + entry_header.entry_count as u32;
        let entry_key = &strings[entry_header.string_index as usize];
        for _ in 0..entry_header.entry_count {
            match entry_header.entry_type {
                1 => {
                    let _ = EntryValue::Float(reader.read_f32());
                }
                2 => {
                    let int = reader.read_u32();
                    let value = strings[int as usize];
                    match *entry_key {
                        "lootRandomizerName" => {
                            tag_name = Some(value.to_string());
                        }
                        "itemClassification" => {
                            rarity = Some(value.to_string());
                        }
                        _str => {}
                    }
                }
                _num => {
                    let int = reader.read_u32();
                    //Seems like the "levelRequirement" field isn't useful..?
                    if *entry_key == "itemLevel" {
                        level_req = Some(int);
                    }
                }
            };

            if tag_name.is_some() && rarity.is_some() && level_req.is_some() {
                break 'outer;
            }
        }
    }

    AffixData {
        tag_name,
        rarity: rarity.unwrap_or_default(),
        name: Default::default(), // Initialized later
        level_req,
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
