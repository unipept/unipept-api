//! A taxon's ancestors, in the shape a response carries them.
//!
//! Two shapes: the id of each ancestor, and the id with its name. Both are written from
//! [`datastore::RANK_NAMES`], so a rank is named nowhere here.

use std::sync::LazyLock;

use datastore::{LineageStore, RANK_COUNT, RANK_NAMES, TaxonStore};
use serde::{Serialize, Serializer, ser::SerializeMap};

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
///
/// Boxed rather than held inline: this is one variant of [`LineageResponse`], and an enum is as
/// large as its largest variant.
#[derive(Debug, Clone)]
pub struct LineageIds {
    ranks: Box<[Option<i32>; RANK_COUNT]>
}

/// The same, with the name of each ancestor beside its id.
#[derive(Debug, Clone)]
pub struct LineageWithNames {
    ranks: Box<[(Option<i32>, String); RANK_COUNT]>
}

impl Serialize for LineageIds {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut lineage = serializer.serialize_map(Some(FIELDS.len()))?;
        for ((id_field, _), id) in FIELDS.iter().zip(self.ranks.iter()) {
            lineage.serialize_entry(id_field, id)?;
        }
        lineage.end()
    }
}

impl Serialize for LineageWithNames {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut lineage = serializer.serialize_map(Some(FIELDS.len() * 2))?;
        for ((id_field, name_field), (id, name)) in FIELDS.iter().zip(self.ranks.iter()) {
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

/// The lineage a request asked for, or none if it asked for no lineage at all.
///
/// `extra` is what turns a lineage on; `names` chooses between the two shapes.
pub fn lineage_for(
    taxon_id: u32,
    extra: bool,
    names: bool,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Option<LineageResponse> {
    match (extra, names) {
        (true, true) => get_lineage_with_names(taxon_id, lineage_store, taxon_store),
        (true, false) => get_lineage(taxon_id, lineage_store),
        (false, _) => None
    }
}

pub fn get_lineage(taxon_id: u32, lineage_store: &LineageStore) -> Option<LineageResponse> {
    let lineage = lineage_store.get(taxon_id)?;

    Some(LineageResponse::Ids(LineageIds { ranks: Box::new(lineage.ranks.map(reported)) }))
}

pub fn get_empty_lineage() -> Option<LineageResponse> {
    Some(LineageResponse::Ids(LineageIds { ranks: Box::new([None; RANK_COUNT]) }))
}

pub fn get_lineage_with_names(
    taxon_id: u32,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Option<LineageResponse> {
    let lineage = lineage_store.get(taxon_id)?;
    let ranks = lineage.ranks.map(|id| (reported(id), name_of(id, taxon_store)));

    Some(LineageResponse::WithNames(LineageWithNames { ranks: Box::new(ranks) }))
}

pub fn get_empty_lineage_with_names() -> Option<LineageResponse> {
    let ranks = std::array::from_fn(|_| (None, String::new()));

    Some(LineageResponse::WithNames(LineageWithNames { ranks: Box::new(ranks) }))
}

/// Borrowed for the miss, so [`reported_ranks`] answers a slice in both arms rather than
/// allocating an empty lineage for a taxon the store does not hold.
static NO_LINEAGE: [Option<i32>; RANK_COUNT] = [None; RANK_COUNT];

/// The ancestor ids, in column order, read out of the store rather than copied.
///
/// [`get_lineage_array`] is this collected. Take this instead wherever the ranks are only read:
/// the array is already in the store, and it is `RANK_COUNT` wide however few ancestors are
/// looked at.
/// The rank array the store holds, or an empty one for a taxon it does not hold.
fn ranks_of(taxon_id: u32, lineage_store: &LineageStore) -> &[Option<i32>] {
    lineage_store.get(taxon_id).map_or(&NO_LINEAGE[..], |lineage| &lineage.ranks[..])
}

pub fn reported_ranks(taxon_id: u32, lineage_store: &LineageStore) -> impl Iterator<Item = Option<i32>> + '_ {
    ranks_of(taxon_id, lineage_store).iter().map(|&id| reported(id))
}

/// The ancestor at one rank column, without reading the others.
///
/// A rank index past the end answers `None`, as does a taxon the store does not hold.
pub fn reported_rank_at(taxon_id: u32, rank_index: usize, lineage_store: &LineageStore) -> Option<i32> {
    reported(lineage_store.get(taxon_id)?.get_rank(rank_index))
}

/// The ancestor ids alone, in column order, for the endpoints that answer a list rather than an
/// object.
///
/// Collected from the slice rather than from [`reported_ranks`]: an `impl Iterator` return type
/// hides `TrustedLen`, which is not an auto trait, and `Vec::from_iter` needs to see it to reserve
/// exactly once instead of checking the capacity per element.
pub fn get_lineage_array(taxon_id: u32, lineage_store: &LineageStore) -> Vec<Option<i32>> {
    ranks_of(taxon_id, lineage_store).iter().map(|&id| reported(id)).collect()
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
}
