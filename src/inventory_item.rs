use crate::VersionNumber;

use super::decrypt::Decrypt;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InventoryItem {
    pub base_name: String,
    pub prefix_name: String,
    pub suffix_name: String,
    pub modifier_name: String,
    pub transmute_name: String,
    pub seed: u32,
    pub component_name: String,
    pub relic_completion_bonus: String,
    pub relic_seed: u32,
    pub augment_name: String,
    pub unknown: u32,
    pub augment_seed: u32,
    pub materia_combines: u32, // what is this?
    pub stack_count: u32,
}

impl InventoryItem {
    pub fn read(decrypter: &mut Decrypt, version: &VersionNumber) -> Result<Self, std::io::Error> {
        let ret = Self {
            base_name: decrypter.read_str()?,
            prefix_name: decrypter.read_str()?,
            suffix_name: decrypter.read_str()?,
            modifier_name: decrypter.read_str()?,
            transmute_name: decrypter.read_str()?,
            seed: decrypter.read_int(),
            component_name: decrypter.read_str()?,
            relic_completion_bonus: decrypter.read_str()?,
            relic_seed: decrypter.read_int(),
            augment_name: decrypter.read_str()?,
            unknown: decrypter.read_int(),
            augment_seed: decrypter.read_int(),
            materia_combines: decrypter.read_int(),
            stack_count: decrypter.read_int(),
        };
        // New values added in 1.3
        match *version {
            // as of 1.3, expectation is stash == 10 or inventory == 8
            VersionNumber::Stash(num) | VersionNumber::Inventory(num) if num >= 8 => {
                let _val1 = decrypter.read_int();
                let _val2 = decrypter.read_int();
                let _val3 = decrypter.read_int();
            }
            _ => {}
        }
        Ok(ret)
    }
}
