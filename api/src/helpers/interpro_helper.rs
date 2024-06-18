use std::collections::HashMap;

use datastore::InterproStore;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub enum InterproEntry {
    Default {
        code: String,
        protein_count: u32,
    },
    Extra {
        code: String,
        protein_count: u32,
        name: String
    },
    ExtraDomains {
        code: String,
        protein_count: u32,
        name: String,
        #[serde(rename = "type")]
        domain: String
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InterproEntries {
    Default (Vec<InterproEntry>),
    Domains (Vec<HashMap<String, Vec<InterproEntry>>>)
}

pub fn interpro_entries(fa_data: &HashMap<String, u32>, interpro_store: &InterproStore, extra: bool, domains: bool) -> InterproEntries {
    let iprs = fa_data.into_iter().filter(|(key, _)| key.starts_with("IPR:"));

    if domains {
        let mut interpro_domains = HashMap::new();
        for (key, &count) in iprs {
            if let Some((domain, name)) = interpro_store.get(&key[4..]) {
                if extra {
                    interpro_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(InterproEntry::Extra {
                        code: key[4..].to_string(),
                        protein_count: count,
                        name: name.to_string()
                    });
                } else {
                    interpro_domains.entry(domain.to_string()).or_insert_with(Vec::new).push(InterproEntry::Default {
                        code: key[4..].to_string(),
                        protein_count: count
                    });
                }
            }
        }

        let mut result = Vec::new();
        for (key, value) in interpro_domains.into_iter() {
            let mut mapping = HashMap::new();
            mapping.insert(key, value);
            result.push(mapping);
        }

        InterproEntries::Domains(result)
    } else if extra {
        InterproEntries::Default(
            iprs.filter_map(|(key, &count)| {
                if let Some((domain, name)) = interpro_store.get(&key[4..]) {
                    Some(InterproEntry::ExtraDomains {
                        code: key[4..].to_string(),
                        protein_count: count,
                        name: name.to_string(),
                        domain: domain.to_string()
                    })
                } else { None }
            }).collect()
        )
    } else {
        InterproEntries::Default(
            iprs.map(|(key, &count)| InterproEntry::Default {
                code: key[4..].to_string(),
                protein_count: count
            }).collect()
        )
    }
}
