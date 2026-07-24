use std::ffi::CStr;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

// Allow cloning the reader to have multiple views into the same underlying data
#[derive(Clone)]
pub struct ByteReader {
    pub bytes: Arc<[u8]>,
    pub index: usize,
}

impl ByteReader {
    pub fn from_file(path: &PathBuf) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        let _len = file.read_to_end(&mut bytes)?;
        Ok(Self {
            bytes: bytes.into(),
            index: 0,
        })
    }

    pub fn from<T>(bytes: T) -> Self
    where
        T: Into<Arc<[u8]>>,
    {
        Self {
            bytes: bytes.into(),
            index: 0,
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        let ret = self.bytes[self.index];
        self.index += 1;
        ret
    }

    pub fn read_u16(&mut self) -> u16 {
        let new_index = self.index + 2;
        let ret = u16::from_ne_bytes(<[u8; 2]>::try_from(&self.bytes[self.index..new_index]).unwrap());
        self.index = new_index;
        ret
    }

    pub fn read_u32(&mut self) -> u32 {
        let new_index = self.index + 4;
        let ret = u32::from_ne_bytes(<[u8; 4]>::try_from(&self.bytes[self.index..new_index]).unwrap());
        self.index = new_index;
        ret
    }

    #[allow(dead_code)]
    pub fn read_f32(&mut self) -> f32 {
        let new_index = self.index + 4;
        let ret = f32::from_ne_bytes(<[u8; 4]>::try_from(&self.bytes[self.index..new_index]).unwrap());
        self.index = new_index;
        ret
    }

    pub fn read_u64(&mut self) -> u64 {
        let new_index = self.index + 8;
        let ret = u64::from_ne_bytes(<[u8; 8]>::try_from(&self.bytes[self.index..new_index]).unwrap());
        self.index = new_index;
        ret
    }

    pub fn read_n_bytes(&mut self, n: u32) -> &[u8] {
        let new_index = self.index + (n as usize);
        let ret = &self.bytes[self.index..new_index];
        self.index = new_index;
        ret
    }

    pub fn read_str(&mut self, len: u32) -> &str {
        let bytes = self.read_n_bytes(len);
        str::from_utf8(bytes).unwrap()
    }

    #[allow(dead_code)]
    pub fn read_cstr(&mut self) -> &CStr {
        let ret = CStr::from_bytes_until_nul(&self.bytes[self.index..self.bytes.len()]).unwrap();
        self.index += ret.count_bytes() + 1;
        ret
    }
}

// Alternate functions that allow splitting ownership of the fields of ByteReader
pub fn read_cstr<'a>(bytes: &'a [u8], index: &mut usize) -> &'a CStr {
    let ret = CStr::from_bytes_until_nul(&bytes[*index..bytes.len()]).unwrap();
    *index += ret.count_bytes() + 1;
    ret
}

pub fn read_str<'a>(bytes: &'a [u8], index: &mut usize, len: u32) -> &'a str {
    let new_index = *index + (len as usize);
    let bytes = &bytes[*index..new_index];
    *index = new_index;
    str::from_utf8(bytes).unwrap()
}

pub fn read_u32(bytes: &[u8], index: &mut usize) -> u32 {
    let new_index = *index + 4;
    let ret = u32::from_ne_bytes(<[u8; 4]>::try_from(&bytes[*index..new_index]).unwrap());
    *index = new_index;
    ret
}
