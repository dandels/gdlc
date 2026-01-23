use crate::arz_parser::{AffixData, ItemData};
use crate::inventory_item::InventoryItem;

use std::collections::HashMap;
use std::{fmt, fmt::Display};

use colored::{ColoredString, Colorize};
use rayon::prelude::*;

pub type LocalizationStrings = HashMap<String, String>;

#[derive(Debug, Default)]
pub struct TagNames {
    pub items: HashMap<String, ItemData>,
    pub affixes: HashMap<String, AffixData>,
}

#[derive(Debug)]
pub struct ItemLookup {
    pub search_term: String,
    pub localization_data: LocalizationStrings,
    pub tag_names: TagNames,
}

pub struct CompleteItem {
    name: String,
    item_rarity: Rarity,
    prefix: Option<String>,
    prefix_rarity: Rarity,
    suffix: Option<String>,
    suffix_rarity: Rarity,
    // level_req: Option<u32>,
    level_req: u32,
    quantity: u32,
}

enum Rarity {
    Legendary,
    Epic,
    Rare,
    RareComponent,
    Magical,
    CommonOrUnknown,
}

impl From<&String> for Rarity {
    fn from(string: &String) -> Self {
        match string.to_lowercase().as_str() {
            "legendary" => Self::Legendary,
            "rare" => Self::Rare,
            "epic" => Self::Epic,
            "magical" => Self::Magical,
            _ => Self::CommonOrUnknown,
        }
    }
}

fn color_item_by_rarity(string: String, rarity: &Rarity) -> ColoredString {
    match rarity {
        Rarity::Legendary => string.purple(),
        Rarity::Epic => string.bright_blue(),
        Rarity::Rare => string.bright_green(),
        Rarity::RareComponent => string.yellow(),
        Rarity::Magical => string.bright_yellow(),
        Rarity::CommonOrUnknown => ColoredString::from(string),
    }
}

fn color_affix_by_rarity(string: String, rarity: &Rarity) -> ColoredString {
    match rarity {
        Rarity::Rare => string.green(),
        Rarity::Magical => string.yellow(),
        _ => ColoredString::from(string),
    }
}

impl CompleteItem {
    fn fmt_searchable_item_name(&self) -> String {
        format!(
            "{} {} {}", // correct amount of whitespace is not important for search
            self.prefix.as_ref().unwrap_or(&"".into()),
            &self.name,
            self.suffix.as_ref().unwrap_or(&"".into())
        )
    }
}

impl Display for CompleteItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let quantity = {
            if self.quantity > 1 {
                format!("(x{}) ", self.quantity)
            } else {
                "".to_string()
            }
        };
        let lvl_req = {
            let req = self.level_req;
            format!("[lvl {req}]")
        };

        let name_colored = color_item_by_rarity(self.name.clone(), &self.item_rarity);
        let suffix_colored = color_affix_by_rarity(self.suffix.clone().unwrap_or("".to_string()), &self.suffix_rarity);
        let prefix_colored = color_affix_by_rarity(self.prefix.clone().unwrap_or("".to_string()), &self.prefix_rarity);
        if self.prefix.is_none() {
            if self.suffix.is_none() {
                write!(f, "{lvl_req} {quantity}{name_colored}")
            } else {
                write!(f, "{lvl_req} {quantity}{name_colored} {suffix_colored}")
            }
        } else if self.suffix.is_none() {
            write!(f, "{lvl_req} {prefix_colored} {name_colored}")
        } else {
            write!(f, "{lvl_req} {prefix_colored} {name_colored} {suffix_colored}")
        }
    }
}

impl ItemLookup {
    pub fn lookup_item(&self, inventory_item: &InventoryItem) -> Option<CompleteItem> {
        let ItemData {
            record_name: _record_name,
            tag_name,
            rarity,
            level_req,
            localized_item_name,
        } = self.tag_names.items.get(&inventory_item.base_name)?; // TODO handle records caught by this

        let item_name: &String = match localized_item_name.get() {
            Some(name) => Some(name),
            None => {
                let ld = self.localization_data.get(tag_name);
                if ld.is_none() {
                    println!("No localization data found for {tag_name}");
                }
                ld
            }
        }?;

        // TODO fix this logic, hashmap needs to count tagnames and not record names
        // if ilvls.len() > 1 {
        // println!("{item_name} has {} tiers", ilvls.len());
        // }

        // Affixes can increase item lvl
        let mut req_incl_affix: u32 = *level_req;

        let mut prefix: Option<String> = None;
        let mut prefix_rarity = Rarity::CommonOrUnknown;
        if !inventory_item.prefix_name.is_empty()
            && let Some(prefix_info) = self.tag_names.affixes.get(&inventory_item.prefix_name)
        {
            if let Some(prefix_req) = prefix_info.level_req
                && prefix_req > req_incl_affix
            {
                req_incl_affix = prefix_req;
            }
            prefix_rarity = Rarity::from(&prefix_info.rarity);
            if let Some(prefix_name) = prefix_info.localized_affix_name.get() {
                prefix = Some(prefix_name.clone());
            } else if let Some(tag_name) = &prefix_info.tag_name
                && let Some(name) = self.localization_data.get(tag_name)
            {
                let _ = prefix_info.localized_affix_name.set(name.clone());
                prefix = Some(name.clone());
            }
        }

        let mut suffix: Option<String> = None;
        let mut suffix_rarity = Rarity::CommonOrUnknown;
        if !inventory_item.suffix_name.is_empty()
            && let Some(suffix_info) = self.tag_names.affixes.get(&inventory_item.suffix_name)
        {
            if let Some(suffix_req) = suffix_info.level_req
                && suffix_req > req_incl_affix
            {
                req_incl_affix = suffix_req;
            }
            suffix_rarity = Rarity::from(&suffix_info.rarity);
            if let Some(suffix_name) = suffix_info.localized_affix_name.get() {
                suffix = Some(suffix_name.clone());
            } else if let Some(tag_name) = &suffix_info.tag_name
                && let Some(name) = self.localization_data.get(tag_name)
            {
                let _ = suffix_info.localized_affix_name.set(name.clone());
                suffix = Some(name.clone());
            }
        }
        let quantity = inventory_item.stack_count;

        let mut item_name = item_name.clone();
        let item_rarity = {
            /* Rare components have this for some reason... Let's give them their own color since we aren't
             * detecting them in any other way. */
            if item_name.starts_with("^k") {
                item_name.drain(0..2);
                Rarity::RareComponent
            } else {
                Rarity::from(rarity)
            }
        };

        let ci = CompleteItem {
            name: item_name,
            item_rarity,
            prefix,
            prefix_rarity,
            suffix,
            suffix_rarity,
            level_req: req_incl_affix,
            quantity,
        };
        Some(ci)
    }

    pub fn check_items(&self, items: &[InventoryItem], item_source: &str) {
        items.par_iter().for_each(|inventory_item| {
            if let Some(ci) = self.lookup_item(inventory_item) {
                let item_name = ci.fmt_searchable_item_name();
                if item_name.to_lowercase().contains(&self.search_term) {
                    // Most of print logic is handled inside CompleteItem
                    println!("{item_source}: {ci}");
                }
            // There are some items with blank fields that might be unused assets. Otherwise log an error.
            } else if !inventory_item.base_name.is_empty() {
                println!("No tag found for {}, ", inventory_item.base_name);
            }
        });
    }
}
