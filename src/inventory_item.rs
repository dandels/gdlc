use super::decrypt::Decrypt;

#[derive(Debug)]
pub struct StorageItem {
    pub item: Item,
    #[allow(dead_code)]
    x_offset: u32,
    #[allow(dead_code)]
    y_offset: u32,
}

#[derive(Debug)]
pub enum StorageType {
    Bag,
    Stash,
}

impl StorageItem {
    pub fn read(decrypt: &mut Decrypt, version: u32, storage_type: StorageType) -> Self {
        let item = Item::read(decrypt, version);

        let mut x_offset = decrypt.read_int();
        let mut y_offset = decrypt.read_int();

        if let StorageType::Stash = storage_type {
            x_offset = f32::from_le_bytes(x_offset.to_le_bytes()) as u32;
            y_offset = f32::from_le_bytes(y_offset.to_le_bytes()) as u32;
        }

        let ret = Self {
            item,
            x_offset,
            y_offset,
        };

        #[cfg(feature = "debug-bytes")]
        {
            println!("{:?}", storage_type);
            println!("x is {}", ret.x_offset);
            println!("y is {}", ret.y_offset);
        }

        ret
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Item {
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
    pub ascendant_record_2h: Option<String>,
    pub unknown: u32, // seems to always just be 0
    pub augment_seed: u32,
    pub materia_combines: u32, // what is this?
    pub stack_count: u32,
    pub seed_rerolls: u32,
    pub affix_rerolls: u32,
}

impl Item {
    pub fn read(decrypter: &mut Decrypt, version: u32) -> Self {
        /* This function just reads and decrypts bytes into the fields of the struct, but it's error prone and ints and
         * other byte sequences get decrypted differently.
         * Uncommenting these prints is useful in combination with printing decrypted bytes to stdout inside Decrypt.
         * You can look at the output in a hex viewer and figure out where things go wrong.
         */

        #[cfg(feature = "debug-bytes")]
        println!("--ver {version}--\nbase_name:");
        let base_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nprefix_name:");
        let prefix_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nsuffix_name:");
        let suffix_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nmodifier_name:");
        let modifier_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\ntransmute_name:");
        let transmute_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nseed:");
        let seed = decrypter.read_int();

        #[cfg(feature = "debug-bytes")]
        println!("\ncomponent:");
        let component_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nrelicbonus:");
        let relic_completion_bonus = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nrelicseed:");
        let relic_seed = decrypter.read_int();

        #[cfg(feature = "debug-bytes")]
        println!("\naugment:");
        let augment_name = decrypter.read_string().unwrap();

        #[cfg(feature = "debug-bytes")]
        println!("\nunknown:");
        let unknown = decrypter.read_int();

        #[cfg(feature = "debug-bytes")]
        assert_eq!(unknown, 0, "Unknown was not 0, was {unknown}. Needs checking.");

        let mut ascendant_record = None;
        let mut ascendant_record_2h = None;

        #[cfg(feature = "debug-bytes")]
        println!("\naugment seed:");

        let augment_seed = decrypter.read_int();
        #[cfg(feature = "debug-bytes")]
        println!("\naugment seed {augment_seed}");

        if version >= 8 {
            #[cfg(feature = "debug-bytes")]
            println!("\nascendant_str:");
            let ascendant_str = decrypter.read_string().unwrap();
            ascendant_record = Some(ascendant_str);

            #[cfg(feature = "debug-bytes")]
            println!("\nascendant_str_2h:");
            let ascendant_str_2h = decrypter.read_string().unwrap();

            #[cfg(feature = "debug-bytes")]
            assert!(
                ascendant_str_2h.is_empty(),
                "Ascendant 2h record was unexpectedly not empty, was {ascendant_str_2h}"
            );
            ascendant_record_2h = Some(ascendant_str_2h);
        }

        let materia_combines = decrypter.read_int();
        #[cfg(feature = "debug-bytes")]
        println!("\nmateria combines:{materia_combines}");

        let stack_count = decrypter.read_int();
        #[cfg(feature = "debug-bytes")]
        println!("\nstack count: {stack_count}");

        let mut seed_rerolls = 0;
        let mut affix_rerolls = 0;
        if version >= 8 {
            seed_rerolls = decrypter.read_int();
            #[cfg(feature = "debug-bytes")]
            println!("\nseed rerolls: {seed_rerolls}");
            if version >= 11 {
                affix_rerolls = decrypter.read_int();
                #[cfg(feature = "debug-bytes")]
                println!("\naffix rerolls: {affix_rerolls}");
            }
        }

        let ret = Self {
            base_name: base_name.clone(),
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
            ascendant_record_2h,
            seed_rerolls,
            affix_rerolls,
        };
        #[cfg(feature = "debug-bytes")]
        print!("\n||||");
        ret
    }
}
