//! A taxon's ancestors, in the shape a response carries them.
//!
//! Two shapes: the id of each ancestor, and the id with its name. Both are written from
//! [`datastore::RANK_NAMES`], so a rank is named nowhere here.

use std::sync::LazyLock;

use datastore::{LineageStore, RANK_COUNT, RANK_NAMES, TaxonRank, TaxonStore};
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

/// The taxon a response names, beside its lineage.
///
/// One declaration, because the three field names and their order are a response contract: every
/// endpoint that carries a taxon has always written them exactly this way.
#[derive(Serialize, Clone)]
pub struct Taxon {
    taxon_id: u32,
    taxon_name: String,
    taxon_rank: String
}

impl Taxon {
    /// The taxon the store names, or `None` where it names none.
    ///
    /// This is the only place that decides what an unnamed taxon means, which is why the callers
    /// answer `None` for the whole row rather than each inventing a placeholder.
    pub fn new(taxon_id: u32, taxon_store: &TaxonStore) -> Option<Self> {
        let (name, rank, _) = taxon_store.get(taxon_id)?;

        Some(Self::named(taxon_id, name, *rank))
    }

    /// For a caller that has already read the store, or one naming a taxon the store does not
    /// hold — root, which `/api/v2/taxonomy` answers for without a row to read.
    pub fn named(taxon_id: u32, taxon_name: &str, taxon_rank: TaxonRank) -> Self {
        Taxon {
            taxon_id,
            taxon_name: taxon_name.to_string(),
            taxon_rank: taxon_rank.to_string()
        }
    }

    /// The id, for a caller that needs it again after handing the taxon over.
    pub fn id(&self) -> u32 {
        self.taxon_id
    }
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

/// The lineage of a taxon with no ancestors, in the shape the request asked for.
///
/// `lineage_for` over a taxon the store holds no row for; root is the caller, since it is below
/// nothing. The two read the same pair of flags, so they answer the same three cases.
pub fn empty_lineage_for(extra: bool, names: bool) -> Option<LineageResponse> {
    match (extra, names) {
        (true, true) => get_empty_lineage_with_names(),
        (true, false) => get_empty_lineage(),
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
}
