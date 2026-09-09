//! The ranks a taxon can carry, and the one list that names them.

use std::{fmt, str::FromStr};

use crate::errors::TaxonStoreError;

/// Every rank, from the broadest to the narrowest, in the order a lineage row holds its columns.
///
/// **The one place the ranks are named.** Spelled as the taxon table spells them; a lineage column
/// keys the two multi-word ranks with an underscore instead, which
/// [`TaxonRank::from_column_name`] reads.
pub const RANK_NAMES: [&str; 28] = [
    "domain",
    "realm",
    "kingdom",
    "subkingdom",
    "superphylum",
    "phylum",
    "subphylum",
    "superclass",
    "class",
    "subclass",
    "superorder",
    "order",
    "suborder",
    "infraorder",
    "superfamily",
    "family",
    "subfamily",
    "tribe",
    "subtribe",
    "genus",
    "subgenus",
    "species group",
    "species subgroup",
    "species",
    "subspecies",
    "strain",
    "varietas",
    "forma"
];

/// How many columns a lineage row carries after its taxon id.
pub const RANK_COUNT: usize = RANK_NAMES.len();

/// Whether two names are the same.
///
/// Spelled out rather than `==`, which a `const fn` cannot call: `PartialEq` is not a const trait
/// yet, and neither is `slice::iter`, so `position` is out too.
/// What the taxon table writes for a taxon at no rank of its own.
const NO_RANK_NAME: &str = "no rank";

/// Whether a rank name and a column name are the same, reading a space and an underscore alike.
///
/// Whether a rank name and a column name are the same, reading a space and an underscore alike.
///
/// Compares in place, because a coarse rank is looked up once per lineage.
fn spelled_alike(rank: &str, column: &str) -> bool {
    rank.len() == column.len()
        && rank
            .bytes()
            .zip(column.bytes())
            .all(|(rank, column)| rank == column || (rank == b' ' && column == b'_'))
}

const fn is(name: &str, other: &str) -> bool {
    let (name, other) = (name.as_bytes(), other.as_bytes());
    if name.len() != other.len() {
        return false;
    }

    let mut index = 0;
    while index < name.len() {
        if name[index] != other[index] {
            return false;
        }
        index += 1;
    }

    true
}

/// The position of a rank in [`RANK_NAMES`], resolved while compiling. An unknown name fails the
/// build.
pub const fn rank_index(name: &str) -> u8 {
    let mut index = 0;
    while index < RANK_NAMES.len() {
        if is(RANK_NAMES[index], name) {
            return index as u8;
        }
        index += 1;
    }

    panic!("no rank carries this name")
}

/// One rank, held as its position in [`RANK_NAMES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxonRank(u8);

impl TaxonRank {
    /// A taxon the taxonomy places at no rank of its own. Not a lineage column.
    pub const NO_RANK: Self = Self(u8::MAX);

    /// The first column of a lineage, whatever the taxonomy calls it. Every taxon below the root
    /// is reached through it.
    pub const TOP_RANK: Self = Self(0);

    /// `calculate_lca` will not let two taxa agree at genus or species by both recording nothing
    /// there, so these two are named. A rename stops the build here.
    pub const GENUS: Self = Self(rank_index("genus"));
    /// See [`Self::GENUS`].
    pub const SPECIES: Self = Self(rank_index("species"));

    /// The rank a name addresses, resolved while compiling, so a caller may name one without
    /// repeating the string as a literal.
    pub const fn named(name: &str) -> Self {
        Self(rank_index(name))
    }

    /// The rank at a column position, or `None` past the last column.
    pub fn from_index(column: usize) -> Option<Self> {
        (column < RANK_COUNT).then_some(Self(column as u8))
    }

    /// Every rank a lineage row holds a column for, in column order. `NO_RANK` is not among them.
    pub fn columns() -> impl Iterator<Item = Self> {
        (0..RANK_COUNT as u8).map(Self)
    }

    /// The name the taxon table spells, and every response carries.
    pub fn as_str(self) -> &'static str {
        self.lineage_index().map_or(NO_RANK_NAME, |column| RANK_NAMES[column])
    }

    /// The lineage column this rank addresses, or `None` for [`Self::NO_RANK`].
    pub fn lineage_index(self) -> Option<usize> {
        let column = self.0 as usize;

        (column < RANK_COUNT).then_some(column)
    }

    /// The rank a lineage column is keyed on, which spells a multi-word rank with an underscore.
    ///
    /// Either separator is read; the rest is matched exactly, since a rank names a column.
    pub fn from_column_name(name: &str) -> Option<Self> {
        Self::found(|rank| spelled_alike(rank, name))
    }

    fn found(is_wanted: impl Fn(&str) -> bool) -> Option<Self> {
        RANK_NAMES.iter().position(|rank| is_wanted(rank)).map(|index| Self(index as u8))
    }
}

impl FromStr for TaxonRank {
    type Err = TaxonStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == NO_RANK_NAME {
            return Ok(Self::NO_RANK);
        }

        Self::found(|rank| rank == s).ok_or_else(|| TaxonStoreError::InvalidRankError(s.to_string()))
    }
}

impl fmt::Display for TaxonRank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rank the taxon table can hold: the lineage columns, plus the one that is not a column.
    fn all_ranks() -> Vec<TaxonRank> {
        std::iter::once(TaxonRank::NO_RANK).chain(TaxonRank::columns()).collect()
    }

    #[test]
    fn every_rank_survives_its_string_form() {
        for rank in all_ranks() {
            assert_eq!(rank.as_str().parse::<TaxonRank>().expect("a known rank"), rank, "{rank}");
        }
    }

    #[test]
    fn a_rank_addresses_its_own_column() {
        for (index, rank) in TaxonRank::columns().enumerate() {
            assert_eq!(rank.lineage_index(), Some(index), "{rank}");
            assert_eq!(rank.as_str(), RANK_NAMES[index]);
        }

        assert_eq!(TaxonRank::NO_RANK.lineage_index(), None);
        assert_eq!(TaxonRank::NO_RANK.as_str(), "no rank");
    }

    /// A lineage column keys a multi-word rank with an underscore; the taxon table writes a space.
    #[test]
    fn a_column_name_is_read_with_either_separator() {
        assert_eq!(TaxonRank::from_column_name("species_group"), Some(TaxonRank::from_str("species group").unwrap()));
        assert_eq!(TaxonRank::from_column_name("species group"), Some(TaxonRank::from_str("species group").unwrap()));
        assert_eq!(TaxonRank::from_column_name("genus"), Some(TaxonRank::GENUS));
    }

    /// A rank names a column, so the case is not normalised.
    #[test]
    fn a_column_name_is_matched_with_regard_to_case() {
        assert_eq!(TaxonRank::from_column_name("Genus"), None);
        assert_eq!("Genus".parse::<TaxonRank>().ok(), None);
    }

    #[test]
    fn a_rank_no_column_carries_is_refused() {
        assert!("nonsense".parse::<TaxonRank>().is_err());
        assert_eq!(TaxonRank::from_column_name("nonsense"), None);
    }

    /// The named ranks are resolved while compiling, so this holds them to the list they came from.
    #[test]
    fn the_named_ranks_address_the_columns_they_name() {
        assert_eq!(TaxonRank::GENUS.as_str(), "genus");
        assert_eq!(TaxonRank::SPECIES.as_str(), "species");
        assert_eq!(TaxonRank::columns().count(), RANK_COUNT);
    }

    #[test]
    fn the_top_rank_is_the_first_column() {
        assert_eq!(TaxonRank::TOP_RANK.lineage_index(), Some(0));
        assert_eq!(TaxonRank::TOP_RANK.as_str(), RANK_NAMES[0]);
    }
}
