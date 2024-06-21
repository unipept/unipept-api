pub mod v1;
pub mod v2;

use datastore::{LineageStore, TaxonStore};
use serde::Serialize;

macro_rules! create_lineages {
    ($($field:ident),*) => {
        paste! {
            #[derive(Serialize, Default)]
            pub struct Lineage {
                $(
                    [<$field _id>]: Option<i32>,
                )*
            }

            #[derive(Serialize, Default)]
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
                get_id(taxon_id).and_then(|id| taxon_store.get(id as u32).map(|(name, _)| name.to_string())).unwrap_or_default()
            }

            pub fn get_lineage(taxon_id: u32, lineage_store: &LineageStore) -> Option<Lineage> {
                let lineage = lineage_store.get(taxon_id)?;

                Some(Lineage {
                    $(
                        [<$field _id>]: get_id(lineage.$field),
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

            pub fn get_lineage_with_names(taxon_id: u32, lineage_store: &LineageStore, taxon_store: &TaxonStore) -> Option<LineageWithNames> {
                let lineage = lineage_store.get(taxon_id)?;

                Some(LineageWithNames {
                    $(
                        [<$field _id>]: get_id(lineage.$field),
                        [<$field _name>]: get_name(lineage.$field, taxon_store)
                    ),*
                })
            }
        }
    };
}

pub(crate) use create_lineages;

#[derive(Clone, Copy)]
pub enum LineageVersion {
    V1,
    V2
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Lineage {
    DefaultV1(v1::Lineage),
    NamesV1(v1::LineageWithNames),
    DefaultV2(v2::Lineage),
    NamesV2(v2::LineageWithNames)
}

pub fn get_lineage(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore) -> Option<Lineage> {
    match version {
        LineageVersion::V1 => v1::get_lineage(taxon_id, lineage_store).map(Lineage::DefaultV1),
        LineageVersion::V2 => v2::get_lineage(taxon_id, lineage_store).map(Lineage::DefaultV2)
    }
}

pub fn get_lineage_array(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore) -> Vec<Option<i32>> {
    match version {
        LineageVersion::V1 => v1::get_lineage_array(taxon_id, lineage_store),
        LineageVersion::V2 => v2::get_lineage_array(taxon_id, lineage_store)
    }
}

pub fn get_lineage_with_names(taxon_id: u32, version: LineageVersion, lineage_store: &LineageStore, taxon_store: &TaxonStore) -> Option<Lineage> {
    match version {
        LineageVersion::V1 => v1::get_lineage_with_names(taxon_id, lineage_store, taxon_store).map(Lineage::NamesV1),
        LineageVersion::V2 => v2::get_lineage_with_names(taxon_id, lineage_store, taxon_store).map(Lineage::NamesV2)
    }
}
