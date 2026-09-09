//! The ranks a taxon can carry, and the one list that names them.

use std::{fmt, str::FromStr};

use crate::errors::TaxonStoreError;

/// Every rank, from the broadest to the narrowest, in the order a lineage row holds its columns.
///
/// **The one place the ranks are named.** The columns of a lineage row, the rank a taxon is stored
/// with, and the fields a lineage answers with are all derived from this list, so a rank added here
/// is carried by every one of them.
///
/// Spelled as the taxon table spells them. A lineage column keys the two multi-word ranks with an
/// underscore instead; [`TaxonRank::from_column_name`] reads either.
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

/// The position of a rank in [`RANK_NAMES`], resolved while compiling.
///
/// A name no rank carries fails the build. A rank renamed in the list therefore stops the build at
/// the constant that named it, rather than addressing the wrong column — which matters, because
/// ranks are renamed: `domain` was `superkingdom`.
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
///
/// A rank is an index rather than a variant of its own because the code names only a handful of
/// them; the rest are carried, compared and written back without ever being mentioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaxonRank(u8);

impl TaxonRank {
    /// A taxon the taxonomy places at no rank of its own. Not a lineage column.
    pub const NO_RANK: Self = Self(u8::MAX);

    /// The broadest rank, which is the first column of a lineage.
    ///
    /// Named for where it sits rather than for what it is called, because what it is called
    /// changes: this rank was `superkingdom` before it was `domain`. Everything below the root is
    /// reached by walking the taxa at this rank.
    pub const TOP: Self = Self(0);

    /// Named rather than positional, because the rule that reads them is about these two ranks.
    ///
    /// `calculate_lca` will not let two taxa agree at genus or species by both recording nothing
    /// there. Rename either in the list and the build stops here.
    pub const GENUS: Self = Self(rank_index("genus"));
    /// See [`Self::GENUS`].
    pub const SPECIES: Self = Self(rank_index("species"));

    /// Every rank a lineage row holds a column for, in column order. `NO_RANK` is not among them.
    pub fn columns() -> impl Iterator<Item = Self> {
        (0..RANK_COUNT as u8).map(Self)
    }

    /// The name the taxon table spells, and every response carries.
    pub fn as_str(&self) -> &'static str {
        RANK_NAMES.get(self.0 as usize).copied().unwrap_or("no rank")
    }

    /// The lineage column this rank addresses, or `None` for [`Self::NO_RANK`].
    pub fn lineage_index(&self) -> Option<usize> {
        (self.0 as usize).lt(&RANK_COUNT).then_some(self.0 as usize)
    }

    /// The rank a lineage column is keyed on, which spells a multi-word rank with an underscore.
    ///
    /// Only the separator is read either way. The rest is matched exactly, because a rank names a
    /// column rather than being text a reader typed.
    pub fn from_column_name(name: &str) -> Option<Self> {
        let spelled = name.replace('_', " ");

        RANK_NAMES.iter().position(|rank| *rank == spelled).map(|index| Self(index as u8))
    }
}

impl FromStr for TaxonRank {
    type Err = TaxonStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "no rank" {
            return Ok(Self::NO_RANK);
        }

        RANK_NAMES
            .iter()
            .position(|rank| *rank == s)
            .map(|index| Self(index as u8))
            .ok_or_else(|| TaxonStoreError::InvalidRankError(s.to_string()))
    }
}

impl fmt::Display for TaxonRank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<TaxonRank> for String {
    fn from(rank: TaxonRank) -> Self {
        rank.as_str().to_string()
    }
}

impl From<&TaxonRank> for String {
    fn from(rank: &TaxonRank) -> Self {
        rank.as_str().to_string()
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

    /// A rank addresses the column at its own position, and `NO_RANK` addresses none.
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

    /// The case is not normalised: a rank names a column rather than being text a reader typed.
    #[test]
    fn a_column_name_is_matched_with_regard_to_case() {
        assert_eq!(TaxonRank::from_column_name("Genus"), None);
        assert_eq!("Genus".parse::<TaxonRank>().ok(), None);
    }

    /// An unknown rank is an error rather than a rank of its own.
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

    /// The broadest rank is the first column, whatever the taxonomy calls it.
    #[test]
    fn the_top_rank_is_the_first_column() {
        assert_eq!(TaxonRank::TOP.lineage_index(), Some(0));
        assert_eq!(TaxonRank::TOP.as_str(), RANK_NAMES[0]);
    }
}
