use std::collections::HashMap;

use datastore::InterproStore;
use serde::Serialize;

use crate::helpers::{by_count_then_key, grouped_by_domain, is_zero};

#[derive(Serialize)]
#[serde(untagged)]
pub enum InterproEntry {
    Default {
        code: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32
    },
    Domains {
        code: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        #[serde(skip_serializing)]
        domain: String
    },
    Extra {
        code: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        name: String,
        #[serde(rename = "type")]
        domain: String
    },
    ExtraDomains {
        code: String,
        #[serde(skip_serializing_if = "is_zero")]
        protein_count: u32,
        name: String,
        #[serde(skip_serializing)]
        domain: String
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InterproEntries {
    Default(Vec<InterproEntry>),
    Domains(Vec<HashMap<String, Vec<InterproEntry>>>)
}

pub fn interpro_entries_from_map(
    fa_data: &HashMap<String, u32>,
    interpro_store: &InterproStore,
    extra: bool,
    domains: bool
) -> InterproEntries {
    let interpro_entries = by_count_then_key(
        fa_data.iter().filter(|(key, _)| key.starts_with("IPR:")).map(|(key, &count)| (key.as_str(), count))
    );

    if domains {
        handle_domains(interpro_entries, interpro_store, extra)
    } else {
        InterproEntries::Default(
            interpro_entries
                .into_iter()
                .filter_map(|(key, count)| interpro_entry(key, count, interpro_store, extra, false))
                .collect()
        )
    }
}

pub fn interpro_entries_from_list(
    fa_data: &[&str],
    interpro_store: &InterproStore,
    extra: bool,
    domains: bool
) -> InterproEntries {
    let interpro_entries = by_count_then_key(fa_data.iter().filter(|key| key.starts_with("IPR:")).map(|&key| (key, 0)));

    if domains {
        handle_domains(interpro_entries, interpro_store, extra)
    } else {
        InterproEntries::Default(
            interpro_entries
                .into_iter()
                .filter_map(|(key, count)| interpro_entry(key, count, interpro_store, extra, false))
                .collect()
        )
    }
}

/// An entry the store knows nothing about is dropped, and so is one built in a shape that carries
/// no domain — neither can be filed under a namespace.
fn handle_domains(iprs: Vec<(&str, u32)>, interpro_store: &InterproStore, extra: bool) -> InterproEntries {
    InterproEntries::Domains(grouped_by_domain(iprs.into_iter().filter_map(|(key, count)| {
        let entry = interpro_entry(key, count, interpro_store, extra, true)?;
        match &entry {
            InterproEntry::Domains { domain, .. } | InterproEntry::ExtraDomains { domain, .. } => {
                let domain = domain.clone();
                Some((domain, entry))
            }
            _ => None
        }
    })))
}

fn interpro_entry(
    key: &str,
    count: u32,
    interpro_store: &InterproStore,
    extra: bool,
    domains: bool
) -> Option<InterproEntry> {
    let trimmed_key = &key[4..];

    let (domain, name) = interpro_store.get(trimmed_key)?;

    if domains {
        if extra {
            Some(InterproEntry::ExtraDomains {
                code: trimmed_key.to_string(),
                protein_count: count,
                name: name.to_string(),
                domain: domain.to_string()
            })
        } else {
            Some(InterproEntry::Domains {
                code: trimmed_key.to_string(),
                protein_count: count,
                domain: domain.to_string()
            })
        }
    } else if extra {
        Some(InterproEntry::Extra {
            code: trimmed_key.to_string(),
            protein_count: count,
            name: name.to_string(),
            domain: domain.to_string()
        })
    } else {
        Some(InterproEntry::Default { code: trimmed_key.to_string(), protein_count: count })
    }
}
