use crate::arz_parser::{AffixData, ItemData};
use crate::inventory_item::InventoryItem;

use std::collections::HashMap;

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
    pub name_printable: String,
    pub name_searchable: String,
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

#[allow(clippy::too_many_arguments)]
fn fmt_printable(
    base_name: String,
    prefix: Option<String>,
    suffix: Option<String>,
    quantity: u32,
    level_req: u32,
    item_rarity: Rarity,
    prefix_rarity: &Rarity,
    suffix_rarity: &Rarity,
) -> String {
    let quantity_str = {
        if quantity > 1 {
            format!("(x{}) ", quantity)
        } else {
            "".to_string()
        }
    };
    let lvl_req_str = {
        let req = level_req;
        format!("[lvl {req}]")
    };

    let name_colored = color_item_by_rarity(base_name, &item_rarity);
    let suffix_colored = color_affix_by_rarity(suffix.clone().unwrap_or("".to_string()), suffix_rarity);
    let prefix_colored = color_affix_by_rarity(prefix.clone().unwrap_or("".to_string()), prefix_rarity);
    if prefix.is_none() {
        if suffix.is_none() {
            format!("{lvl_req_str} {quantity_str}{name_colored}")
        } else {
            format!("{lvl_req_str} {quantity_str}{name_colored} {suffix_colored}")
        }
    } else if suffix.is_none() {
        format!("{lvl_req_str} {prefix_colored} {name_colored}")
    } else {
        format!("{lvl_req_str} {prefix_colored} {name_colored} {suffix_colored}")
    }
}

impl ItemLookup {
    pub fn lookup_item(&self, inventory_item: &InventoryItem) -> Option<CompleteItem> {
        #[allow(clippy::manual_map)]
        #[allow(clippy::needless_match)]
        let item_data_opt = match self.tag_names.items.get(&inventory_item.base_name) {
            Some(id) => Some(id),
            None => {
                // TODO handle records caught by this
                // println!("nothing found for {:?}", inventory_item);
                None
            }
        };

        let ItemData {
            record_name: _record_name,
            tag_name,
            rarity,
            level_req,
            localized_item_name,
        } = item_data_opt?;
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

        // Increase item lvl if affix req is higher than base item
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

        let name_searchable = format!(
            "{} {} {}", // correct amount of whitespace is not important for search
            prefix.as_ref().unwrap_or(&"".into()),
            &item_name,
            suffix.as_ref().unwrap_or(&"".into())
        )
        .to_lowercase();

        let name_printable = fmt_printable(
            item_name,
            prefix,
            suffix,
            quantity,
            req_incl_affix,
            item_rarity,
            &prefix_rarity,
            &suffix_rarity,
        );

        let ci = CompleteItem {
            name_printable,
            name_searchable,
        };
        Some(ci)
    }

    pub fn search_uncached(&self, owned_items: &Vec<(String, Vec<InventoryItem>)>) {
        owned_items.par_iter().for_each(|(item_source, items)| {
            items.par_iter().for_each(|inventory_item| {
                if let Some(ci) = self.lookup_item(inventory_item) {
                    if ci.name_searchable.contains(&self.search_term) {
                        println!("{item_source}: {}", ci.name_printable);
                    }
                // There are some items with blank fields that might be unused assets. Otherwise log an error.
                // TODO filter them out earlier?
                } else if !inventory_item.base_name.is_empty() {
                    println!("No tag found for {}, ", inventory_item.base_name);
                }
            });
        });
    }
}
