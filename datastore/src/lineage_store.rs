use std::{
    borrow::Cow,
    collections::HashMap,
    io::{BufRead, BufReader},
    sync::Arc
};

use serde::Serialize;

use crate::{errors::LineageStoreError, taxon_store::LineageRank};

/// Declares the lineage columns once: the struct that holds them, the index each answers to, and
/// the name that addresses it.
///
/// The order is the order of the columns in the lineage file, and every one of the three follows
/// from it, so a rank cannot be added to one and forgotten in another.
macro_rules! lineage_columns {
    ($($rank:ident),*) => {
        #[derive(Clone, Debug, Serialize, Default)]
        pub struct Lineage {
            $(pub $rank: Option<i32>),*
        }

        impl Lineage {
            /// Fills every column from one row's parsed values, in declaration order.
            pub fn set_ranks(&mut self, values: &[Option<i32>]) {
                let mut index = 0;
                $(
                    self.$rank = values.get(index).copied().flatten();
                    index += 1;
                )*
                let _ = index;
            }

            /// The id at a rank index, in the order the columns are declared in.
            pub fn get_rank(&self, rank_index: usize) -> Option<i32> {
                let mut index = 0;
                $(
                    if index == rank_index {
                        return self.$rank;
                    }
                    index += 1;
                )*
                let _ = index;
                None
            }
        }

        impl LineageStore {
            /// The index a rank name addresses, or `None` for a name no column carries.
            fn index_of(name: &str) -> Option<usize> {
                let mut index = 0;
                $(
                    if name == stringify!($rank) {
                        return Some(index);
                    }
                    index += 1;
                )*
                let _ = index;
                None
            }
        }

        /// Every column name, in order.
        pub const RANK_NAMES: [&str; RANK_COUNT] = [$(stringify!($rank)),*];
    };
}

/// How many columns a lineage row carries after its taxon id.
pub const RANK_COUNT: usize = 28;

lineage_columns!(
    domain,
    realm,
    kingdom,
    subkingdom,
    superphylum,
    phylum,
    subphylum,
    superclass,
    class,
    subclass,
    superorder,
    order,
    suborder,
    infraorder,
    superfamily,
    family,
    subfamily,
    tribe,
    subtribe,
    genus,
    subgenus,
    species_group,
    species_subgroup,
    species,
    subspecies,
    strain,
    varietas,
    forma
);

impl Lineage {
    /// Retrieves the ID of this lineage at a specific rank name. If the provided rank is invalid
    /// None is returned.
    pub fn get_taxon_id_at_rank(&self, rank_name: &str) -> Option<i32> {
        self.get_rank(LineageStore::rank_to_idx(rank_name)?)
    }
}

pub struct LineageStore {
    // Keep track of all lineages (id -> lineage)
    pub mapper: HashMap<u32, Arc<Lineage>>,
    // Make it possible to retrieve a Lineage based upon the values in one of its columns.
    pub index_references: Vec<HashMap<u32, Vec<Arc<Lineage>>>>
}

impl LineageStore {
    /// The number of rank columns a lineage row carries. [`RANK_NAMES`] names them.
    pub const AMOUNT_OF_RANKS: usize = RANK_COUNT;

    /// The lineage column a rank name addresses.
    ///
    /// Either spelling is read: the columns are keyed on `species_group`, and the taxonomy — and
    /// so every response — writes `species group`. A caller that hands back a rank the API gave it
    /// is answered rather than refused.
    ///
    /// Case is not normalised. A rank argument names a column, so it is matched exactly; the taxon
    /// filter, which matches a rank as text a user typed, does its own lowercasing and does not
    /// come through here.
    ///
    /// A caller that holds a `LineageRank` has [`LineageRank::lineage_index`] instead.
    pub fn rank_to_idx(s: &str) -> Option<usize> {
        let key = if s.contains(' ') { Cow::Owned(s.replace(' ', "_")) } else { Cow::Borrowed(s) };

        Self::index_of(key.as_ref())
    }

    pub fn try_from_file(file: &str) -> Result<Self, LineageStoreError> {
        let file = std::fs::File::open(file).map_err(|_| LineageStoreError::FileNotFound(file.to_string()))?;

        let mut mapper = HashMap::new();

        let mut index_references: Vec<HashMap<u32, Vec<Arc<Lineage>>>> = Vec::new();

        for _ in 0..LineageStore::AMOUNT_OF_RANKS {
            index_references.push(HashMap::new());
        }

        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line_number = index + 1;

            // A blank line is not a malformed row: `lines()` yields one for every empty line in
            // the file, including the one a file ending in two newlines produces. Only a truly
            // empty line, though — `trim()` would also erase a row of nothing but tabs, and a row
            // of delimiters is malformed input, not an absence of input.
            if line.is_empty() {
                continue;
            }

            // Counted before anything is parsed, so a row of the wrong width is reported as such
            // rather than as whichever of its fields happens to fail parsing first.
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != LineageStore::AMOUNT_OF_RANKS + 1 {
                return Err(LineageStoreError::UnexpectedColumnCount {
                    line: line_number,
                    expected: LineageStore::AMOUNT_OF_RANKS + 1,
                    found: fields.len()
                });
            }

            let taxon_id: u32 = fields[0]
                .parse()
                .map_err(|_| LineageStoreError::InvalidTaxonId { line: line_number, value: fields[0].to_string() })?;

            // Enumerated for the column number: a lineage row has 28 rank fields, and an error
            // naming only the offending value leaves the reader counting tabs to find it.
            let mut parts: Vec<Option<i32>> = Vec::with_capacity(LineageStore::AMOUNT_OF_RANKS);
            for (rank, field) in fields[1..].iter().enumerate() {
                parts.push(match *field {
                    "\\N" => None,
                    value => Some(value.parse::<i32>().map_err(|_| LineageStoreError::InvalidRankId {
                        line: line_number,
                        column: rank + 2,
                        value: value.to_string()
                    })?)
                });
            }

            let mut lin = Lineage::default();
            lin.set_ranks(&parts);
            let lin = Arc::new(lin);

            mapper.insert(taxon_id, Arc::clone(&lin));

            // Zipped rather than indexed: both sides are `AMOUNT_OF_RANKS` long, and pairing them
            // this way says so without a bounds check that could fail.
            for (rank_map, part) in index_references.iter_mut().zip(parts.iter()) {
                if let Some(id) = part {
                    rank_map.entry(id.unsigned_abs()).or_default().push(Arc::clone(&lin));
                }
            }
        }

        Ok(Self { mapper, index_references })
    }

    pub fn get(&self, key: u32) -> Option<&Arc<Lineage>> {
        self.mapper.get(&key)
    }

    pub fn get_lineages_at_rank(&self, rank: &LineageRank, taxon_id: u32) -> Option<&Vec<Arc<Lineage>>> {
        rank.lineage_index()
            .and_then(|idx| self.index_references.get(idx))
            .and_then(|map| map.get(&taxon_id))
    }

    /// Returns all unique taxon IDs at a specific rank in the NCBI taxonomy.
    /// Ascending by taxon id.
    pub fn get_all_taxon_ids_at_rank(&self, rank: &LineageRank) -> Option<Vec<u32>> {
        rank.lineage_index().and_then(|idx| self.index_references.get(idx)).map(|map| {
            let mut ids: Vec<u32> = map.keys().cloned().collect();
            ids.sort_unstable();
            ids
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rank names index the lineage columns in order, and each one reads back its own column.
    ///
    /// `rank_to_idx`, `get_taxon_id_at_rank` and `get_rank` are three hand-written tables over the
    /// same 28 ranks, listed in the same order, and nothing else checks that they agree with each
    /// other or with the column order the parser fills.
    #[test]
    fn every_rank_key_addresses_its_own_column() {
        // A different value in every column, so a name reading the wrong one is visible.
        let mut lineage = Lineage::default();
        let columns: Vec<Option<i32>> = (0..RANK_COUNT).map(|position| Some(position as i32 + 1000)).collect();
        lineage.set_ranks(&columns);

        for (position, key) in RANK_NAMES.iter().enumerate() {
            assert_eq!(LineageStore::rank_to_idx(key), Some(position), "rank_to_idx({key})");
            assert_eq!(lineage.get_taxon_id_at_rank(key), Some(position as i32 + 1000), "get_taxon_id_at_rank({key})");
            assert_eq!(lineage.get_rank(position), Some(position as i32 + 1000), "get_rank({position})");
        }
    }

    #[test]
    fn an_unknown_rank_key_addresses_nothing() {
        assert_eq!(LineageStore::rank_to_idx("nonsense"), None);
    }

    /// A rank name is read whether it is spelled with a space or an underscore.
    ///
    /// The columns are keyed on the underscore; the taxonomy and every response write the space.
    #[test]
    fn a_space_and_an_underscore_address_the_same_column() {
        for (spaced, keyed) in [("species group", "species_group"), ("species subgroup", "species_subgroup")] {
            let by_key = LineageStore::rank_to_idx(keyed);

            assert!(by_key.is_some(), "`{keyed}`");
            assert_eq!(LineageStore::rank_to_idx(spaced), by_key, "`{spaced}`");
        }
    }

    /// Only the separator is normalised. A rank argument names a column and is matched exactly.
    #[test]
    fn a_rank_name_is_read_case_sensitively() {
        for name in ["SPECIES", "Species", "SPECIES_GROUP", "Species Group"] {
            assert_eq!(LineageStore::rank_to_idx(name), None, "`{name}`");
        }
    }

    /// Normalising leaves the twenty-six single-word ranks exactly as they were.
    #[test]
    fn a_single_word_rank_is_unchanged_by_normalising() {
        for (position, key) in RANK_NAMES.iter().enumerate() {
            if !key.contains('_') {
                assert_eq!(LineageStore::rank_to_idx(key), Some(position), "`{key}`");
            }
        }
        assert_eq!(Lineage::default().get_taxon_id_at_rank("nonsense"), None);
        assert_eq!(Lineage::default().get_rank(LineageStore::AMOUNT_OF_RANKS), None);
    }
}
