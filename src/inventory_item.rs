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
    pub ascendant_record: Option<String>,
    pub ascendant_rerolls: Option<u32>,
    pub unknown: u32,
    pub augment_seed: u32,
    pub materia_combines: u32, // what is this?
    pub stack_count: u32,
    pub seed_rerolls: u32,
    pub affix_rerolls: u32,
}

impl InventoryItem {
    pub fn read(decrypter: &mut Decrypt, version: u32) -> Self {
        let base_name = decrypter.read_str().unwrap();
        let prefix_name = decrypter.read_str().unwrap();
        let suffix_name = decrypter.read_str().unwrap();
        let modifier_name = decrypter.read_str().unwrap();
        let transmute_name = decrypter.read_str().unwrap();
        let seed = decrypter.read_int();
        let component_name = decrypter.read_str().unwrap();
        let relic_completion_bonus = decrypter.read_str().unwrap();
        let relic_seed = decrypter.read_int();
        let augment_name = decrypter.read_str().unwrap();

        let mut ascendant_record = None;
        let mut ascendant_rerolls = None;
        if version >= 8 {
            ascendant_record = Some(decrypter.read_str().unwrap());
            ascendant_rerolls = Some(decrypter.read_int());
        }

        let unknown = decrypter.read_int();
        let augment_seed = decrypter.read_int();
        let materia_combines = decrypter.read_int();
        let stack_count = decrypter.read_int();

        let mut seed_rerolls = 0;
        let mut affix_rerolls = 0;
        if version >= 8 {
            seed_rerolls = decrypter.read_int();
            if version >= 11 {
                affix_rerolls = decrypter.read_int();
            }
        }

        Self {
            base_name,
            prefix_name,
            suffix_name,
            modifier_name,
            transmute_name,
            seed,
            component_name,
            relic_completion_bonus,
            relic_seed,
            augment_name,
            unknown,
            augment_seed,
            materia_combines,
            stack_count,
            ascendant_record,
            ascendant_rerolls,
            seed_rerolls,
            affix_rerolls,
        }
    }
}
