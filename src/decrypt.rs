use crate::byte_reader::ByteReader;

#[cfg(feature = "debug-bytes")]
use std::io::Write;

use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::Path;

const PRIME: u32 = 39916801;

#[derive(Clone)]
pub struct Block {
    #[allow(dead_code)]
    pub len: u32,
    pub end: u32,
}

pub struct Decrypt {
    byte_reader: ByteReader,
    table: [u32; 256],
    key: u32,
    pub blocks: Vec<Block>,
}

impl Decrypt {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        let len = file.read_to_end(&mut bytes)?;
        let mut reader = ByteReader::from(bytes);
        let key = reader.read_u32() ^ 0x55555555;
        let mut k = key;
        let mut table = [0; 256];
        for i in &mut table {
            k = k.rotate_right(1).wrapping_mul(PRIME);
            *i = k;
        }

        Ok(Self {
            byte_reader: reader,
            table,
            key,
            blocks: Vec::from([Block {
                len: len as u32,
                end: len as u32,
            }]),
        })
    }

    pub fn assert_is_within_block(&mut self, i: u32) {
        let index = self.byte_reader.index as u32;
        let end = self.blocks.last().unwrap().end;

        let remaining = end - self.byte_reader.index as u32;
        let delta = remaining.abs_diff(i);

        let is_in_bounds = i <= end - index;

        #[cfg(feature = "debug-bytes")]
        if !is_in_bounds {
            println!(
                "Exceeded block bounds. Len was {i}, index {index}, end {end}, remaining {remaining}, delta {delta}."
            );
            // Dump remaining decrypted bytes before crashing. Any integers will only have a correct first byte,
            // because the bytes are interpreted differently depending on whether you xor 4 individual bytes, or
            // entire ints.
            let bytes = self.read_n_bytes(remaining).to_owned();
            println!("FFFFFFFF"); // Easy to find marker at the end of the output
        }

        assert!(
            is_in_bounds,
            "Tried to read outside block. Exceeded block bounds. Len was {i}, remaining {remaining}, delta {delta}."
        )
    }

    fn rotate_key(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.key ^= self.table[*byte as usize];
        }
    }

    pub fn read_int(&mut self) -> u32 {
        let num = self.byte_reader.read_u32();
        let ret = num ^ self.key;
        self.rotate_key(&num.to_le_bytes());

        #[cfg(feature = "debug-bytes")]
        std::io::stdout().write_all(&ret.to_le_bytes()).unwrap();

        ret
    }

    pub fn next_int(&mut self) -> u32 {
        self.byte_reader.read_u32() ^ self.key
    }

    #[allow(dead_code)]
    fn next_float(&mut self) -> f32 {
        self.next_int() as f32
    }

    pub fn read_byte(&mut self) -> u8 {
        let mut byte = self.byte_reader.read_byte();
        self.key ^= self.table[byte as usize];
        byte ^= self.key as u8;

        #[cfg(feature = "debug-bytes")]
        std::io::stdout().write_all(&[byte]).unwrap();

        byte
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_byte() != 0
    }

    pub fn read_n_bytes(&mut self, n: u32) -> Vec<u8> {
        self.assert_is_within_block(n);
        let mut bytes = self.byte_reader.read_n_bytes(n).to_owned();
        for i in 0..n {
            let byte = bytes[i as usize] ^ self.key as u8;
            self.key ^= self.table[bytes[i as usize] as usize];
            bytes[i as usize] = byte;
        }

        #[cfg(feature = "debug-bytes")]
        std::io::stdout().write_all(&bytes).unwrap();

        bytes
    }

    pub fn read_string(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let len = self.read_int();
        if len == 0 {
            Ok("".to_string())
        } else {
            self.assert_is_within_block(len);
            let str_buf = self.read_n_bytes(len).to_owned();
            // TODO error handling for invalid strings
            let ret = String::from_utf8(str_buf).unwrap();
            assert_eq!(ret.len(), len as usize);
            Ok(ret)
        }
    }

    pub fn read_wide_string(&mut self) -> String {
        let len_u16 = self.read_int();

        if len_u16 > 0 {
            let len_u8 = len_u16 * 2;
            let str_buf = self.read_n_bytes(len_u8).to_owned();

            let mut wstr_buf: Vec<u16> = vec![0; len_u16 as usize];

            // Gritty manual way to convert Vec<u8> to Vec<u16>. Luckily we only do this once per character name.
            let mut k = 0;
            while k < len_u16 as usize {
                let j = k * 2;
                let mut wchar: u16 = str_buf[j] as u16;
                wchar |= (str_buf[j + 1] as u16) << 8;
                wstr_buf[k] = wchar;
                k += 1;
            }

            // TODO error handling for invalid strings
            let ret_str = String::from_utf16(&wstr_buf).unwrap();
            return ret_str;
        }
        "".to_string()
    }

    pub fn read_block_start(&mut self) -> u32 {
        let block_start = self.read_int();
        let len = self.next_int();
        let index: u32 = self.byte_reader.index.try_into().unwrap();
        let end = index + len;
        let block = Block { len, end };
        self.blocks.push(block);
        self.assert_is_within_block(len);
        block_start
    }

    pub fn read_block_end(&mut self) {
        let index: u32 = self.byte_reader.index as u32;
        let block = self.blocks.pop().unwrap();
        assert_eq!(index, block.end);
        let end_int = self.next_int();
        if end_int != 0 {
            panic!("Expected end of block character 0, got {end_int}.");
        }
    }
}
