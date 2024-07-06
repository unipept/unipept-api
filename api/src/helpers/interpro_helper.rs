use std::collections::HashMap;

use datastore::InterproStore;
use serde::Serialize;

use crate::helpers::is_zero;

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
    let interpro_entries = fa_data.iter().filter(|(key, _)| key.starts_with("IPR:"));

    if domains {
        handle_domains(interpro_entries.map(|(key, count)| (key.as_str(), count)), interpro_store, extra)
    } else {
        InterproEntries::Default(
            interpro_entries
                .filter_map(|(key, &count)| interpro_entry(key, count, interpro_store, extra, false))
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
    let interpro_entries = fa_data.iter().filter(|key| key.starts_with("IPR:"));

    if domains {
        handle_domains(interpro_entries.map(|&key| (key, &0u32)), interpro_store, extra)
    } else {
        InterproEntries::Default(
            interpro_entries.filter_map(|key| interpro_entry(key, 0, interpro_store, extra, false)).collect()
        )
    }
}

fn handle_domains<'a>(
    iprs: impl Iterator<Item = (&'a str, &'a u32)>,
    interpro_store: &InterproStore,
    extra: bool
) -> InterproEntries {
    let mut interpro_domains = HashMap::new();
    for (key, &count) in iprs {
        if let Some(entry) = interpro_entry(key, count, interpro_store, extra, true) {
            if let InterproEntry::Domains { domain, .. } | InterproEntry::ExtraDomains { domain, .. } = &entry {
                interpro_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(entry);
            }
        }
    }

    let result: Vec<HashMap<String, Vec<InterproEntry>>> = interpro_domains
        .into_iter()
        .map(|(key, value)| {
            let mut mapping = HashMap::new();
            mapping.insert(key, value);
            mapping
        })
        .collect();

    InterproEntries::Domains(result)
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
