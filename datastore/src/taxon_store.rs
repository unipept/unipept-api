use std::{
    collections::HashMap,
    fmt,
    io::{BufRead, BufReader},
    str::FromStr
};

use crate::{errors::TaxonStoreError, lineage_store::LineageStore};

pub type TaxonInformation = (String, LineageRank, bool);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineageRank {
    NoRank,
    Domain,
    Realm,
    Kingdom,
    Subkingdom,
    Superphylum,
    Phylum,
    Subphylum,
    Superclass,
    Class,
    Subclass,
    Superorder,
    Order,
    Suborder,
    Infraorder,
    Superfamily,
    Family,
    Subfamily,
    Tribe,
    Subtribe,
    Genus,
    Subgenus,
    SpeciesGroup,
    SpeciesSubgroup,
    Species,
    Subspecies,
    Strain,
    Varietas,
    Forma
}

impl fmt::Display for LineageRank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if *self == LineageRank::NoRank { write!(f, "{:?}", "root") } else { write!(f, "{:?}", self) }
    }
}

pub struct TaxonStore {
    pub mapper: HashMap<u32, TaxonInformation>
}

impl TaxonStore {
    pub fn try_from_file(file: &str) -> Result<Self, TaxonStoreError> {
        let file = std::fs::File::open(file).map_err(|_| TaxonStoreError::FileNotFound(file.to_string()))?;

        let mut mapper = HashMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line_number = index + 1;

            // Only a truly empty line: `trim()` would also erase a row of nothing but tabs, and
            // a row of delimiters is malformed input rather than an absence of input.
            if line.is_empty() {
                continue;
            }

            // Not trimmed before splitting: `trim_end` would eat a trailing delimiter and let a
            // six-column row pass as five. `lines()` has already removed the newline, CRLF
            // included, and neither validity byte is whitespace, so there is nothing left to trim.
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 5 {
                return Err(TaxonStoreError::UnexpectedColumnCount {
                    line: line_number,
                    expected: 5,
                    found: parts.len()
                });
            }

            let taxon_id: u32 = parts[0]
                .parse()
                .map_err(|_| TaxonStoreError::InvalidTaxonId { line: line_number, value: parts[0].to_string() })?;

            let rank = parts[2]
                .parse::<LineageRank>()
                .map_err(|_| TaxonStoreError::InvalidRank { line: line_number, value: parts[2].to_string() })?;

            // The validity flag is a MySQL boolean dump: 0x01 for a valid taxon, 0x00 otherwise.
            // Neither byte is whitespace, so `trim_end` above leaves the column intact.
            mapper.insert(taxon_id, (parts[1].to_string(), rank, matches!(parts[4], "\x01")));
        }

        Ok(Self { mapper })
    }

    pub fn get(&self, key: u32) -> Option<&TaxonInformation> {
        self.mapper.get(&key)
    }

    pub fn get_name(&self, key: u32) -> Option<&String> {
        self.mapper.get(&key).map(|(name, _, _)| name)
    }

    pub fn is_valid(&self, key: u32) -> bool {
        self.mapper.contains_key(&key) && self.mapper[&key].2
    }
}

impl LineageRank {
    /// The 28 ranks a lineage row carries, in the order `LineageStore` reads its columns.
    ///
    /// `NoRank` is not among them: it is a rank a taxon can have, not a column a lineage has.
    /// `LineageStore::rank_to_idx` indexes the same ranks in the same order, and
    /// `the_lineage_order_matches_the_column_index` holds the two together.
    pub const LINEAGE_ORDER: [LineageRank; LineageStore::AMOUNT_OF_RANKS] = [
        LineageRank::Domain,
        LineageRank::Realm,
        LineageRank::Kingdom,
        LineageRank::Subkingdom,
        LineageRank::Superphylum,
        LineageRank::Phylum,
        LineageRank::Subphylum,
        LineageRank::Superclass,
        LineageRank::Class,
        LineageRank::Subclass,
        LineageRank::Superorder,
        LineageRank::Order,
        LineageRank::Suborder,
        LineageRank::Infraorder,
        LineageRank::Superfamily,
        LineageRank::Family,
        LineageRank::Subfamily,
        LineageRank::Tribe,
        LineageRank::Subtribe,
        LineageRank::Genus,
        LineageRank::Subgenus,
        LineageRank::SpeciesGroup,
        LineageRank::SpeciesSubgroup,
        LineageRank::Species,
        LineageRank::Subspecies,
        LineageRank::Strain,
        LineageRank::Varietas,
        LineageRank::Forma
    ];
}

impl FromStr for LineageRank {
    type Err = TaxonStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "no rank" => Ok(Self::NoRank),
            "domain" => Ok(Self::Domain),
            "realm" => Ok(Self::Realm),
            "kingdom" => Ok(Self::Kingdom),
            "subkingdom" => Ok(Self::Subkingdom),
            "superphylum" => Ok(Self::Superphylum),
            "phylum" => Ok(Self::Phylum),
            "subphylum" => Ok(Self::Subphylum),
            "superclass" => Ok(Self::Superclass),
            "class" => Ok(Self::Class),
            "subclass" => Ok(Self::Subclass),
            "superorder" => Ok(Self::Superorder),
            "order" => Ok(Self::Order),
            "suborder" => Ok(Self::Suborder),
            "infraorder" => Ok(Self::Infraorder),
            "superfamily" => Ok(Self::Superfamily),
            "family" => Ok(Self::Family),
            "subfamily" => Ok(Self::Subfamily),
            "tribe" => Ok(Self::Tribe),
            "subtribe" => Ok(Self::Subtribe),
            "genus" => Ok(Self::Genus),
            "subgenus" => Ok(Self::Subgenus),
            "species group" => Ok(Self::SpeciesGroup),
            "species subgroup" => Ok(Self::SpeciesSubgroup),
            "species" => Ok(Self::Species),
            "subspecies" => Ok(Self::Subspecies),
            "strain" => Ok(Self::Strain),
            "varietas" => Ok(Self::Varietas),
            "forma" => Ok(Self::Forma),
            _ => Err(TaxonStoreError::InvalidRankError(s.to_string()))
        }
    }
}

impl From<LineageRank> for String {
    fn from(val: LineageRank) -> Self {
        match val {
            LineageRank::NoRank => "no rank".to_string(),
            LineageRank::Domain => "domain".to_string(),
            LineageRank::Realm => "realm".to_string(),
            LineageRank::Kingdom => "kingdom".to_string(),
            LineageRank::Subkingdom => "subkingdom".to_string(),
            LineageRank::Superphylum => "superphylum".to_string(),
            LineageRank::Phylum => "phylum".to_string(),
            LineageRank::Subphylum => "subphylum".to_string(),
            LineageRank::Superclass => "superclass".to_string(),
            LineageRank::Class => "class".to_string(),
            LineageRank::Subclass => "subclass".to_string(),
            LineageRank::Superorder => "superorder".to_string(),
            LineageRank::Order => "order".to_string(),
            LineageRank::Suborder => "suborder".to_string(),
            LineageRank::Infraorder => "infraorder".to_string(),
            LineageRank::Superfamily => "superfamily".to_string(),
            LineageRank::Family => "family".to_string(),
            LineageRank::Subfamily => "subfamily".to_string(),
            LineageRank::Tribe => "tribe".to_string(),
            LineageRank::Subtribe => "subtribe".to_string(),
            LineageRank::Genus => "genus".to_string(),
            LineageRank::Subgenus => "subgenus".to_string(),
            LineageRank::SpeciesGroup => "species group".to_string(),
            LineageRank::SpeciesSubgroup => "species subgroup".to_string(),
            LineageRank::Species => "species".to_string(),
            LineageRank::Subspecies => "subspecies".to_string(),
            LineageRank::Strain => "strain".to_string(),
            LineageRank::Varietas => "varietas".to_string(),
            LineageRank::Forma => "forma".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rank the taxon table can hold: the lineage columns, plus the one that is not a column.
    fn all_ranks() -> Vec<LineageRank> {
        std::iter::once(LineageRank::NoRank).chain(LineageRank::LINEAGE_ORDER).collect()
    }

    /// Every rank survives a trip through its string form and back.
    ///
    /// The two directions are written out as separate 29-arm tables, so nothing but this stops one
    /// gaining a rank the other does not have — and a rank that fails to round-trip is one the
    /// taxon table can hold and the parser cannot read back.
    #[test]
    fn every_rank_round_trips_through_its_string_form() {
        for rank in all_ranks() {
            let text: String = rank.clone().into();
            let parsed: LineageRank = text.parse().unwrap_or_else(|_| panic!("`{text}` does not parse back"));
            assert_eq!(parsed, rank, "`{text}` parsed as a different rank");
        }
    }

    #[test]
    fn an_unknown_rank_string_is_rejected() {
        assert!("not a rank".parse::<LineageRank>().is_err());
    }

    /// `LINEAGE_ORDER` and `rank_to_idx` are two hand-written tables over the same 28 ranks. A
    /// caller that indexes a lineage column with one and names it with the other gets the wrong
    /// rank if they ever disagree.
    ///
    /// The two spell a rank differently — the columns use `species_group`, the taxon table's rank
    /// column `species group` — so the comparison goes through the underscore form.
    #[test]
    fn the_lineage_order_matches_the_column_index() {
        for (index, rank) in LineageRank::LINEAGE_ORDER.into_iter().enumerate() {
            let name: String = rank.into();
            assert_eq!(LineageStore::rank_to_idx(&name.replace(' ', "_")), Some(index), "`{name}`");
        }
    }
}
