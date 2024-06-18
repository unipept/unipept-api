use std::collections::HashMap;

use datastore::EcStore;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub enum EcNumber {
    Default {
        ec_number: String,
        protein_count: u32,
    },
    Extra {
        ec_number: String,
        protein_count: u32,
        name: String,
    }
}

pub fn ec_numbers(fa_data: &HashMap<String, u32>, ec_store: &EcStore, extra: bool) -> Vec<EcNumber> {
    let ecs = fa_data.into_iter().filter(|(key, _)| key.starts_with("EC:"));

    if extra {
        ecs.map(|(key, &count)| EcNumber::Extra {
            ec_number: key[3..].to_string(),
            protein_count: count,
            name: ec_store.get(&key[3..]).cloned().unwrap_or_default()
        }).collect()
    } else {
        ecs.map(|(key, &count)| EcNumber::Default {
            ec_number: key[3..].to_string(),
            protein_count: count
        }).collect()
    }
}
