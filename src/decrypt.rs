use crate::byte_reader::ByteReader;

use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::Path;

const PRIME: u32 = 39916801;

pub struct Block {
    #[allow(dead_code)]
    pub len: u32,
    pub end: u32,
}

pub struct Decrypt {
    byte_reader: ByteReader,
    table: [u32; 256],
    key: u32,
}

impl Decrypt {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        let _len = file.read_to_end(&mut bytes)?;
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
        })
    }

    pub fn read_int(&mut self) -> u32 {
        let num = self.byte_reader.read_u32();
        let ret = num ^ self.key;
        for byte in num.to_be_bytes() {
            self.key ^= self.table[byte as usize];
        }
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
        let byte = self.byte_reader.read_byte();
        self.key ^= self.table[byte as usize];
        byte ^ (self.key as u8)
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_byte() != 0
    }

    pub fn read_str(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let len = self.read_int();
        if len > 0 {
            let mut str_buf = self.byte_reader.read_n_bytes(len).to_owned();
            for i in 0..len {
                let byte = (str_buf[i as usize] as u32 ^ self.key) as u8;
                self.key ^= self.table[str_buf[i as usize] as usize];
                str_buf[i as usize] = byte;
            }
            // TODO error handling for invalid strings
            let ret_str = String::from_utf8(str_buf)?;
            return Ok(ret_str);
        }
        Ok("".to_string())
    }

    pub fn read_wide_string(&mut self) -> Result<String, Error> {
        let len_u16 = self.read_int();
        if len_u16 > 0 {
            let len_u8 = len_u16 * 2;
            let mut str_buf = self.byte_reader.read_n_bytes(len_u8).to_owned();

            for i in 0..len_u8 {
                let byte = (str_buf[i as usize] as u32 ^ self.key) as u8;
                self.key ^= self.table[str_buf[i as usize] as usize];
                str_buf[i as usize] = byte;
            }

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
            return Ok(ret_str);
        }
        Ok("".to_string())
    }

    pub fn read_block_start(&mut self) -> (u32, Block) {
        let block_start = self.read_int();
        let len = self.next_int();
        let index: u32 = self.byte_reader.index.try_into().unwrap();
        let end = index + len;
        (block_start, Block { len, end })
    }

    pub fn read_block_end(&mut self, block: &Block) {
        let index: u32 = self.byte_reader.index.try_into().unwrap();
        if block.end != index {
            panic!("Stream position is {index} but block end is {}. Delta: {}", block.end, index.abs_diff(block.end));
        }

        let end_int = self.next_int();
        if end_int != 0 {
            panic!("Expected end of block character 0, got {end_int}.");
        }
    }
}
