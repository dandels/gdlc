use crate::byte_reader;

use super::ByteReader;
use std::collections::HashMap;
use std::ffi::CStr;
use std::io::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ArcRecordHeader {
    pub record_type: u32,
    pub offset: u32,
    pub len_compressed: u32,
    pub len_decompressed: u32,
    pub unknown: u32,
    pub filetime: u64,
    pub parts_count: u32,
    pub index: u32,
    pub str_len: u32,
    pub str_offset: u32,
}

impl ArcRecordHeader {
    pub fn new(byte_vec: &mut ByteReader) -> Self {
        Self {
            record_type: byte_vec.read_u32(),
            offset: byte_vec.read_u32(),
            len_compressed: byte_vec.read_u32(),
            len_decompressed: byte_vec.read_u32(),
            unknown: byte_vec.read_u32(),
            filetime: byte_vec.read_u64(),
            parts_count: byte_vec.read_u32(),
            index: byte_vec.read_u32(),
            str_len: byte_vec.read_u32(),
            str_offset: byte_vec.read_u32(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArcArchiveHeader {
    #[allow(dead_code)]
    pub unknown: u32,
    pub version: u32,
    pub files_count: u32,
    pub records_count: u32,
    pub record_len: u32,
    pub string_table_len: u32,
    pub record_offset: u32,
}

impl ArcArchiveHeader {
    fn new(byte_vec: &mut ByteReader) -> Self {
        Self {
            unknown: byte_vec.read_u32(),
            version: byte_vec.read_u32(),
            files_count: byte_vec.read_u32(),
            records_count: byte_vec.read_u32(),
            record_len: byte_vec.read_u32(),
            string_table_len: byte_vec.read_u32(),
            record_offset: byte_vec.read_u32(),
        }
    }
}

#[derive(Debug)]
struct ArcRecordPartMetadata {
    // are these partial or whole records?
    pub offset: u32,
    pub len_compressed: u32,
    pub len_decompressed: u32,
}

impl ArcRecordPartMetadata {
    pub fn new(byte_vec: &mut ByteReader) -> Self {
        Self {
            offset: byte_vec.read_u32(),
            len_compressed: byte_vec.read_u32(),
            len_decompressed: byte_vec.read_u32(),
        }
    }
}

pub fn read_archive(path: &PathBuf) -> Result<HashMap<String, String>, Error> {
    let mut byte_vec = ByteReader::from_file(path)?;
    let archive_header = ArcArchiveHeader::new(&mut byte_vec);
    assert!(archive_header.version == 3, "expected header version 3, is {}", archive_header.version);

    let record_headers = read_record_headers(&mut byte_vec, &archive_header);
    let record_parts_metadata = read_record_metadata(&mut byte_vec, &archive_header);

    let strings = read_strings(&mut byte_vec, &archive_header);
    //let mut items_index = None;
    let mut indices = Vec::new();
    for (i, string) in strings.iter().enumerate() {
        let file_names = [
            // just guessing all the gdx3 file names, they need checking
            c"tags_items.txt",
            c"tagsgdx1_items.txt",
            c"tagsgdx2_items.txt",
            c"tagsgdx3_items.txt",
            c"tagsgdx2_endlessdungeon.txt",
            c"tagsgdx3_endlessdungeon.txt",
            c"tags_storyelements.txt",
            c"tagsgdx1_storyelements.txt",
            c"tagsgdx2_storyelements.txt",
            c"tagsgdx3_storyelements.txt",
        ];
        if file_names.iter().any(|s| string == s) {
            indices.push(i);
        }
    }
    assert_eq!(archive_header.files_count as usize, record_headers.len());
    let mut map = HashMap::new();
    for i in indices {
        let data = decompress(&mut byte_vec, &record_parts_metadata[i]);
        for string in String::from_utf8(data).unwrap().lines() {
            if string.is_empty() || string.starts_with("#") {
                continue;
            }
            if let Some((key, value)) = string.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    Ok(map)
}

fn read_record_metadata(byte_reader: &mut ByteReader, header: &ArcArchiveHeader) -> Vec<ArcRecordPartMetadata> {
    let mut record_metadatas: Vec<ArcRecordPartMetadata> = Vec::with_capacity(header.records_count as usize);
    byte_reader.index = header.record_offset as usize;
    for _ in 0..header.records_count {
        record_metadatas.push(ArcRecordPartMetadata::new(byte_reader));
    }
    record_metadatas
}

fn read_strings<'a>(byte_reader: &'a mut ByteReader, header: &ArcArchiveHeader) -> Vec<&'a CStr> {
    let mut strings = Vec::new();
    byte_reader.index = (header.record_offset + header.record_len) as usize;

    for _ in 0..header.files_count {
        let string = byte_reader::read_cstr(&byte_reader.bytes, &mut byte_reader.index);
        strings.push(string);
    }
    strings
}

fn read_record_headers(byte_reader: &mut ByteReader, header: &ArcArchiveHeader) -> Vec<ArcRecordHeader> {
    let mut records = Vec::new();

    byte_reader.index = (header.record_offset + header.record_len + header.string_table_len) as usize;
    for _ in 0..header.files_count {
        records.push(ArcRecordHeader::new(byte_reader));
    }
    records
}

fn decompress(byte_reader: &mut ByteReader, metadata: &ArcRecordPartMetadata) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    byte_reader.index = metadata.offset as usize;
    if metadata.len_compressed == metadata.len_decompressed {
        data.append(&mut byte_reader.read_n_bytes(metadata.len_compressed).to_vec());
    } else {
        let mut buf = vec![0; metadata.len_decompressed as usize];
        let compressed_data = &byte_reader.read_n_bytes(metadata.len_compressed);
        lz4::block::decompress_to_buffer(
            compressed_data,
            Some(metadata.len_decompressed.try_into().unwrap()),
            &mut buf,
        )
        .unwrap();
        data.append(&mut buf.to_vec());
    }
    data
}
