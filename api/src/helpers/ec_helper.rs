use std::collections::HashMap;

use datastore::EcStore;
use serde::Serialize;

use crate::helpers::is_zero;

#[derive(Serialize)]
#[serde(untagged)]
pub enum EcNumber {
    Default {
        ec_number: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32
    },
    Extra {
        ec_number: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        name: String
    }
}

pub fn ec_numbers_from_map(fa_data: &HashMap<String, u32>, ec_store: &EcStore, extra: bool) -> Vec<EcNumber> {
    fa_data
        .iter()
        .filter(|(key, _)| key.starts_with("EC:"))
        .map(|(key, &count)| ec_number(key, count, ec_store, extra))
        .collect()
}

pub fn ec_numbers_from_list(fa_data: &[&str], ec_store: &EcStore, extra: bool) -> Vec<EcNumber> {
    fa_data
        .iter()
        .filter(|key| key.starts_with("EC:"))
        .map(|key| ec_number(key, 0, ec_store, extra))
        .collect()
}

fn ec_number(key: &str, count: u32, ec_store: &EcStore, extra: bool) -> EcNumber {
    if extra {
        EcNumber::Extra {
            ec_number: key[3..].to_string(),
            protein_count: count,
            name: ec_store.get(&key[3..]).cloned().unwrap_or_default()
        }
    } else {
        EcNumber::Default { ec_number: key[3..].to_string(), protein_count: count }
    }
}
