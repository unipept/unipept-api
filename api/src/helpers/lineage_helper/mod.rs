pub mod v1;
pub mod v2;

use datastore::{LineageStore, TaxonStore};
use serde::Serialize;
use v1::{lineages_v1, LineageV1};
use v2::{lineages_v2, LineageV2};

#[derive(Clone, Copy)]
pub enum LineageVersion {
    V1,
    V2
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Lineage {
    V1(LineageV1),
    V2(LineageV2)
}

pub fn lineages(lca: u32, names: bool, lineage_store: &LineageStore, taxon_store: &TaxonStore, version: LineageVersion) -> Option<Lineage> {
    match version {
        LineageVersion::V1 => lineages_v1(lca, names, lineage_store, taxon_store).map(Lineage::V1),
        LineageVersion::V2 => lineages_v2(lca, names, lineage_store, taxon_store).map(Lineage::V2)
    }
}

fn get_name(taxon_id: Option<i32>, taxon_store: &TaxonStore) -> String {
    taxon_id.and_then(|id| taxon_store.get(id.abs() as u32).map(|(name, _)| name.to_string())).unwrap_or("".to_string())
}
