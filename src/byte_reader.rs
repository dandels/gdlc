use std::cell::Cell;
use std::ffi::CStr;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

// Allow cloning the reader to have multiple views into the same underlying data
#[derive(Clone)]
pub struct ByteReader {
    pub bytes: Arc<Vec<u8>>,
    pub index: Cell<usize>,
}

impl ByteReader {
    // thread_local!(pub static INDEX: Cell<usize> = const { Cell::new(0) });

    pub fn from_file(path: &PathBuf) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        let _len = file.read_to_end(&mut bytes)?;
        Ok(Self {
            bytes: Arc::new(bytes),
            index: Cell::new(0),
        })
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
            index: Cell::new(0),
        }
    }

    pub fn read_byte(&self) -> u8 {
        let index = self.index.get();
        let ret = self.bytes[index];
        self.index.set(index + 1);
        ret
    }

    pub fn read_u16(&self) -> u16 {
        let index = self.index.get();
        let new_index = index + 2;
        let ret = u16::from_ne_bytes(<[u8; 2]>::try_from(&self.bytes[index..new_index]).unwrap());
        self.index.set(new_index);
        ret
    }

    pub fn read_u32(&self) -> u32 {
        let index = self.index.get();
        let new_index = index + 4;
        let ret = u32::from_ne_bytes(<[u8; 4]>::try_from(&self.bytes[index..new_index]).unwrap());
        self.index.set(new_index);
        ret
    }

    pub fn read_f32(&self) -> f32 {
        let index = self.index.get();
        let new_index = index + 4;
        let ret = f32::from_ne_bytes(<[u8; 4]>::try_from(&self.bytes[index..new_index]).unwrap());
        self.index.set(new_index);
        ret
    }

    pub fn read_u64(&self) -> u64 {
        let index = self.index.get();
        let new_index = index + 8;
        let ret = u64::from_ne_bytes(<[u8; 8]>::try_from(&self.bytes[index..new_index]).unwrap());
        self.index.set(new_index);
        ret
    }

    pub fn read_n_bytes(&self, n: u32) -> &[u8] {
        let index = self.index.get();
        let new_index = index + (n as usize);
        let ret = &self.bytes[index..new_index];
        self.index.set(new_index);
        ret
    }

    pub fn read_str(&self, len: u32) -> &str {
        let bytes = self.read_n_bytes(len);
        str::from_utf8(bytes).unwrap()
    }

    pub fn read_cstr(&self) -> &CStr {
        let index = self.index.get();
        let ret = CStr::from_bytes_until_nul(&self.bytes[index..self.bytes.len()]).unwrap();
        self.index.set(index + ret.count_bytes() + 1);
        ret
    }
}
