use std::collections::HashMap;

use datastore::EcStore;
use serde::Serialize;

use crate::helpers::{family_from_list, family_from_map, is_zero};

/// The prefix an EC annotation carries in the aggregated counts.
const PREFIX: &str = "EC:";

#[derive(Serialize, Clone)]
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
    ec_numbers(family_from_map(fa_data, PREFIX), ec_store, extra)
}

pub fn ec_numbers_from_list(fa_data: &[&str], ec_store: &EcStore, extra: bool) -> Vec<EcNumber> {
    ec_numbers(family_from_list(fa_data, PREFIX), ec_store, extra)
}

fn ec_numbers(ecs: Vec<(&str, u32)>, ec_store: &EcStore, extra: bool) -> Vec<EcNumber> {
    ecs.into_iter().map(|(key, count)| ec_number(key, count, ec_store, extra)).collect()
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
