pub mod v2;

use datastore::{LineageStore, TaxonStore};
use serde::Serialize;

macro_rules! create_lineages {
    ($($field:ident),*) => {
        paste! {
            #[derive(Serialize, Default, Debug)]
            pub struct Lineage {
                $(
                    [<$field _id>]: Option<i32>,
                )*
            }

            #[derive(Serialize, Default, Debug)]
            pub struct LineageWithNames {
                $(
                    [<$field _id>]: Option<i32>,
                    [<$field _name>]: String
                ),*
            }

            fn get_id(taxon_id: Option<i32>) -> Option<i32> {
                taxon_id.filter(|&id| id != -1).map(|id| id.abs())
            }

            fn get_name(taxon_id: Option<i32>, taxon_store: &TaxonStore) -> String {
                get_id(taxon_id).and_then(|id| taxon_store.get(id as u32).map(|(name, _, _)| name.to_string())).unwrap_or_default()
            }

            pub fn get_lineage(taxon_id: u32, lineage_store: &LineageStore) -> Option<Lineage> {
                let lineage = lineage_store.get(taxon_id)?;

                Some(Lineage {
                    $(
                        [<$field _id>]: get_id(lineage.$field),
                    )*
                })
            }

            pub fn get_empty_lineage() -> Option<Lineage> {
                 Some(Lineage {
                    $(
                        [<$field _id>]: None,
                    )*
                })
            }

            pub fn get_lineage_array(taxon_id: u32, lineage_store: &LineageStore) -> Vec<Option<i32>> {
                let lineage = lineage_store.get(taxon_id).cloned().unwrap_or_default();

                vec![
                    $(
                        get_id(lineage.$field),
                    )*
                ]
            }

            pub fn get_lineage_array_numeric(taxon_id: u32, lineage_store: &LineageStore) -> Vec<i32> {
                let lineage = lineage_store.get(taxon_id).cloned().unwrap_or_default();
                
                vec![
                    $(
                        get_id(lineage.$field).unwrap_or(0),
                    )*
                ]
            }

            pub fn get_lineage_with_names(taxon_id: u32, lineage_store: &LineageStore, taxon_store: &TaxonStore) -> Option<LineageWithNames> {
                let lineage = lineage_store.get(taxon_id)?;
                
                Some(LineageWithNames {
                    $(
                        [<$field _id>]: get_id(lineage.$field),
                        [<$field _name>]: get_name(lineage.$field, taxon_store)
                    ),*
                })
            }

            pub fn get_empty_lineage_with_names() -> Option<LineageWithNames> {
                Some(LineageWithNames {
                    $(
                        [<$field _id>]: None,
                        [<$field _name>]: String::from("")
                    ),*
                })
            }

        }
    };
}

pub(crate) use create_lineages;

#[derive(Clone, Copy)]
pub enum LineageVersion {
    V2
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Lineage {
    DefaultV2(v2::Lineage),
    NamesV2(v2::LineageWithNames)
}

pub fn get_lineage(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore) -> Option<Lineage> {
    match version {
        LineageVersion::V2 => v2::get_lineage(taxon_id, lineage_store).map(Lineage::DefaultV2)
    }
}

pub fn get_empty_lineage(version: LineageVersion) -> Option<Lineage> {
    match version {
        LineageVersion::V2 => v2::get_empty_lineage().map(Lineage::DefaultV2)
    }
}

pub fn get_lineage_array(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore) -> Vec<Option<i32>> {
    match version {
        LineageVersion::V2 => v2::get_lineage_array(taxon_id, lineage_store)
    }
}

pub fn get_lineage_array_numeric(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore) -> Vec<i32> {
    match version {
        LineageVersion::V2 => v2::get_lineage_array_numeric(taxon_id, lineage_store)
    }
}

pub fn get_lineage_with_names(
    taxon_id: u32,
    version: LineageVersion,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Option<Lineage> {
    v2::get_lineage_with_names(taxon_id, lineage_store, taxon_store).map(Lineage::NamesV2)
}

pub fn get_empty_lineage_with_names(version: LineageVersion) -> Option<Lineage> {
    match version {
        LineageVersion::V2 => v2::get_empty_lineage_with_names().map(Lineage::NamesV2)
    }
}

pub fn get_amount_of_ranks(version: LineageVersion) -> u8 {
    match version {
        LineageVersion::V2 => 28
    }
}

pub fn get_genus_index(version: LineageVersion) -> u8 {
    match version {
        LineageVersion::V2 => 19
    }
}

pub fn get_species_index(version: LineageVersion) -> u8 {
    match version {
        LineageVersion::V2 => 23
    }
}
