//! A taxon's ancestors, in the shape a response carries them.
//!
//! Two shapes: ids alone, and ids with the name of each ancestor. Both are written from
//! [`datastore::RANK_NAMES`], so neither names a rank of its own.

use datastore::{LineageStore, RANK_NAMES, TaxonStore};
use serde::{Serialize, Serializer, ser::SerializeMap};

/// The field names a lineage answers with, built once from the rank names.
///
/// `serde` writes a field name it is given rather than one it derives, so the names are held here
/// as owned strings rather than rebuilt for every lineage in a response.
static FIELDS: std::sync::LazyLock<Vec<(String, String)>> =
    std::sync::LazyLock::new(|| RANK_NAMES.iter().map(|rank| (format!("{rank}_id"), format!("{rank}_name"))).collect());

/// A rank whose taxon the store marks invalid holds a negative id, and one it holds no taxon at
/// holds -1. Neither is reported as an ancestor.
fn reported(taxon_id: Option<i32>) -> Option<i32> {
    taxon_id.filter(|&id| id != -1).map(i32::abs)
}

fn name_of(taxon_id: Option<i32>, taxon_store: &TaxonStore) -> String {
    reported(taxon_id)
        .and_then(|id| taxon_store.get(id as u32).map(|(name, _, _)| name.to_string()))
        .unwrap_or_default()
}

/// One ancestor id per rank, answered as `{rank}_id`.
#[derive(Debug, Default)]
pub struct Lineage {
    ranks: Vec<Option<i32>>
}

/// The same, with the name of each ancestor beside its id.
#[derive(Debug, Default)]
pub struct LineageWithNames {
    ranks: Vec<(Option<i32>, String)>
}

impl Serialize for Lineage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut lineage = serializer.serialize_map(Some(FIELDS.len()))?;
        for ((id_field, _), id) in FIELDS.iter().zip(&self.ranks) {
            lineage.serialize_entry(id_field, id)?;
        }
        lineage.end()
    }
}

impl Serialize for LineageWithNames {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut lineage = serializer.serialize_map(Some(FIELDS.len() * 2))?;
        for ((id_field, name_field), (id, name)) in FIELDS.iter().zip(&self.ranks) {
            lineage.serialize_entry(id_field, id)?;
            lineage.serialize_entry(name_field, name)?;
        }
        lineage.end()
    }
}

/// Whichever shape a request asked for.
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum AnyLineage {
    Ids(Lineage),
    WithNames(LineageWithNames)
}

fn ranks_of(taxon_id: u32, lineage_store: &LineageStore) -> Option<Vec<Option<i32>>> {
    Some(lineage_store.get(taxon_id)?.ranks.to_vec())
}

pub fn get_lineage(taxon_id: u32, lineage_store: &LineageStore) -> Option<AnyLineage> {
    let ranks = ranks_of(taxon_id, lineage_store)?.into_iter().map(reported).collect();

    Some(AnyLineage::Ids(Lineage { ranks }))
}

pub fn get_empty_lineage() -> Option<AnyLineage> {
    Some(AnyLineage::Ids(Lineage { ranks: vec![None; RANK_NAMES.len()] }))
}

pub fn get_lineage_with_names(
    taxon_id: u32,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Option<AnyLineage> {
    let ranks = ranks_of(taxon_id, lineage_store)?
        .into_iter()
        .map(|id| (reported(id), name_of(id, taxon_store)))
        .collect();

    Some(AnyLineage::WithNames(LineageWithNames { ranks }))
}

pub fn get_empty_lineage_with_names() -> Option<AnyLineage> {
    let ranks = vec![(None, String::new()); RANK_NAMES.len()];

    Some(AnyLineage::WithNames(LineageWithNames { ranks }))
}

/// The ancestor ids alone, in column order, for the endpoints that answer a list rather than an
/// object.
pub fn get_lineage_array(taxon_id: u32, lineage_store: &LineageStore) -> Vec<Option<i32>> {
    lineage_store
        .get(taxon_id)
        .map_or_else(|| vec![None; RANK_NAMES.len()], |lineage| lineage.ranks.iter().map(|&id| reported(id)).collect())
}

/// As [`get_lineage_array`], with a rank holding no ancestor written as zero.
pub fn get_lineage_array_numeric(taxon_id: u32, lineage_store: &LineageStore) -> Vec<i32> {
    get_lineage_array(taxon_id, lineage_store).into_iter().map(|id| id.unwrap_or(0)).collect()
}

/// Kept as a function rather than read from `RANK_NAMES` at each call site, because the callers
/// read it as a `u8` bound.
pub fn get_amount_of_ranks() -> u8 {
    RANK_NAMES.len() as u8
}

pub fn get_genus_index() -> u8 {
    rank_index("genus")
}

pub fn get_species_index() -> u8 {
    rank_index("species")
}

fn rank_index(rank: &str) -> u8 {
    RANK_NAMES.iter().position(|name| *name == rank).expect("a rank the columns carry") as u8
}
