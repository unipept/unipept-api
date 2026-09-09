//! A taxon's ancestors, in the shape a response carries them.
//!
//! Two shapes: the id of each ancestor, and the id with its name. Both are written from
//! [`datastore::RANK_NAMES`], so a rank is named nowhere here.

use std::sync::LazyLock;

use datastore::{LineageStore, RANK_NAMES, TaxonStore};
use serde::{
    Serialize, Serializer,
    ser::{Error, SerializeMap}
};

/// The field names a lineage answers with, built once.
///
/// A rank whose name holds a space is written with an underscore, which is how every response has
/// always spelled these fields.
static FIELDS: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    RANK_NAMES
        .iter()
        .map(|rank| {
            let rank = rank.replace(' ', "_");
            (format!("{rank}_id"), format!("{rank}_name"))
        })
        .collect()
});

/// A rank the taxonomy records nothing at holds -1, and one whose taxon it marks invalid holds a
/// negative id. Neither is reported as an ancestor of its own.
fn reported(taxon_id: Option<i32>) -> Option<i32> {
    taxon_id.filter(|&id| id != -1).map(i32::abs)
}

fn name_of(taxon_id: Option<i32>, taxon_store: &TaxonStore) -> String {
    reported(taxon_id)
        .and_then(|id| taxon_store.get(id as u32).map(|(name, _, _)| name.to_string()))
        .unwrap_or_default()
}

/// The id of each ancestor, answered as `{rank}_id`.
#[derive(Debug, Default, Clone)]
pub struct LineageIds {
    ranks: Vec<Option<i32>>
}

/// The same, with the name of each ancestor beside its id.
#[derive(Debug, Default, Clone)]
pub struct LineageWithNames {
    ranks: Vec<(Option<i32>, String)>
}

/// Refuses to answer a lineage that does not carry one entry per rank, rather than writing a
/// shorter object than every other response.
fn checked<S: Serializer, T>(ranks: &[T]) -> Result<(), S::Error> {
    if ranks.len() == FIELDS.len() {
        return Ok(());
    }

    Err(S::Error::custom(format!("a lineage of {} ranks, where {} are named", ranks.len(), FIELDS.len())))
}

impl Serialize for LineageIds {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        checked::<S, _>(&self.ranks)?;

        let mut lineage = serializer.serialize_map(Some(FIELDS.len()))?;
        for ((id_field, _), id) in FIELDS.iter().zip(&self.ranks) {
            lineage.serialize_entry(id_field, id)?;
        }
        lineage.end()
    }
}

impl Serialize for LineageWithNames {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        checked::<S, _>(&self.ranks)?;

        let mut lineage = serializer.serialize_map(Some(FIELDS.len() * 2))?;
        for ((id_field, name_field), (id, name)) in FIELDS.iter().zip(&self.ranks) {
            lineage.serialize_entry(id_field, id)?;
            lineage.serialize_entry(name_field, name)?;
        }
        lineage.end()
    }
}

/// A lineage as a response carries it, in whichever shape the request asked for.
///
/// Not [`datastore::Lineage`], which is the row as the taxonomy wrote it: a rank there holds a
/// negative id where its taxon is invalid, and -1 where it holds no taxon, and each reader of the
/// store takes that differently. These are the ids a caller is shown.
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum LineageResponse {
    Ids(LineageIds),
    WithNames(LineageWithNames)
}

pub fn get_lineage(taxon_id: u32, lineage_store: &LineageStore) -> Option<LineageResponse> {
    let lineage = lineage_store.get(taxon_id)?;
    let ranks = lineage.ranks.iter().map(|&id| reported(id)).collect();

    Some(LineageResponse::Ids(LineageIds { ranks }))
}

pub fn get_empty_lineage() -> Option<LineageResponse> {
    Some(LineageResponse::Ids(LineageIds { ranks: vec![None; RANK_NAMES.len()] }))
}

pub fn get_lineage_with_names(
    taxon_id: u32,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Option<LineageResponse> {
    let lineage = lineage_store.get(taxon_id)?;
    let ranks = lineage.ranks.iter().map(|&id| (reported(id), name_of(id, taxon_store))).collect();

    Some(LineageResponse::WithNames(LineageWithNames { ranks }))
}

pub fn get_empty_lineage_with_names() -> Option<LineageResponse> {
    let ranks = vec![(None, String::new()); RANK_NAMES.len()];

    Some(LineageResponse::WithNames(LineageWithNames { ranks }))
}

/// The ancestor ids alone, in column order, for the endpoints that answer a list rather than an
/// object.
pub fn get_lineage_array(taxon_id: u32, lineage_store: &LineageStore) -> Vec<Option<i32>> {
    lineage_store
        .get(taxon_id)
        .map_or_else(|| vec![None; RANK_NAMES.len()], |lineage| lineage.ranks.iter().map(|&id| reported(id)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The response fields are the rank names, and nothing else states them.
    ///
    /// A rank renamed or reordered in the list changes what a lineage answers with, and this is
    /// what says so — the field names are built at runtime, so no compiler check reaches them.
    #[test]
    fn a_lineage_answers_a_field_per_rank() {
        let answered = serde_json::to_value(get_empty_lineage().expect("a lineage")).expect("serialisable");
        let fields: Vec<&str> = answered.as_object().expect("an object").keys().map(String::as_str).collect();

        let expected: Vec<String> = RANK_NAMES.iter().map(|rank| format!("{}_id", rank.replace(' ', "_"))).collect();

        assert_eq!(fields.len(), RANK_NAMES.len());
        for field in expected {
            assert!(fields.contains(&field.as_str()), "{field} is not among {fields:?}");
        }
    }

    /// The named shape carries both a rank's id and its name.
    #[test]
    fn a_named_lineage_answers_two_fields_per_rank() {
        let answered = serde_json::to_value(get_empty_lineage_with_names().expect("a lineage")).expect("serialisable");
        let fields = answered.as_object().expect("an object");

        assert_eq!(fields.len(), RANK_NAMES.len() * 2);
        for rank in RANK_NAMES {
            let rank = rank.replace(' ', "_");
            assert!(fields.contains_key(&format!("{rank}_id")), "{rank}_id");
            assert!(fields.contains_key(&format!("{rank}_name")), "{rank}_name");
        }
    }

    /// A lineage carrying the wrong number of ranks is refused rather than answered short.
    #[test]
    fn a_lineage_of_the_wrong_width_is_not_answered() {
        let short = LineageResponse::Ids(LineageIds { ranks: vec![None; RANK_NAMES.len() - 1] });

        assert!(serde_json::to_value(short).is_err());
    }
}
