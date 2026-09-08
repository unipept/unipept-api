use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    sync::Arc
};

use serde::Serialize;

use crate::{errors::LineageStoreError, taxon_store::LineageRank};

#[derive(Clone, Debug, Serialize, Default)]
pub struct Lineage {
    pub domain: Option<i32>,
    pub realm: Option<i32>,
    pub kingdom: Option<i32>,
    pub subkingdom: Option<i32>,
    pub superphylum: Option<i32>,
    pub phylum: Option<i32>,
    pub subphylum: Option<i32>,
    pub superclass: Option<i32>,
    pub class: Option<i32>,
    pub subclass: Option<i32>,
    pub superorder: Option<i32>,
    pub order: Option<i32>,
    pub suborder: Option<i32>,
    pub infraorder: Option<i32>,
    pub superfamily: Option<i32>,
    pub family: Option<i32>,
    pub subfamily: Option<i32>,
    pub tribe: Option<i32>,
    pub subtribe: Option<i32>,
    pub genus: Option<i32>,
    pub subgenus: Option<i32>,
    pub species_group: Option<i32>,
    pub species_subgroup: Option<i32>,
    pub species: Option<i32>,
    pub subspecies: Option<i32>,
    pub strain: Option<i32>,
    pub varietas: Option<i32>,
    pub forma: Option<i32>
}

impl Lineage {
    /// Retrieves the ID of this lineage at a specific rank name. If the provided rank is invalid
    /// None is returned.
    pub fn get_taxon_id_at_rank(&self, rank_name: &str) -> Option<i32> {
        match rank_name {
            "domain" => self.domain,
            "realm" => self.realm,
            "kingdom" => self.kingdom,
            "subkingdom" => self.subkingdom,
            "superphylum" => self.superphylum,
            "phylum" => self.phylum,
            "subphylum" => self.subphylum,
            "superclass" => self.superclass,
            "class" => self.class,
            "subclass" => self.subclass,
            "superorder" => self.superorder,
            "order" => self.order,
            "suborder" => self.suborder,
            "infraorder" => self.infraorder,
            "superfamily" => self.superfamily,
            "family" => self.family,
            "subfamily" => self.subfamily,
            "tribe" => self.tribe,
            "subtribe" => self.subtribe,
            "genus" => self.genus,
            "subgenus" => self.subgenus,
            "species_group" => self.species_group,
            "species_subgroup" => self.species_subgroup,
            "species" => self.species,
            "subspecies" => self.subspecies,
            "strain" => self.strain,
            "varietas" => self.varietas,
            "forma" => self.forma,
            _ => None
        }
    }

    /// Retrieves the ID of this lineage at a rank index, in the same order as
    /// [`LineageStore::rank_to_idx`]. If the index is out of range, None is returned.
    pub fn get_rank(&self, rank_index: usize) -> Option<i32> {
        match rank_index {
            0 => self.domain,
            1 => self.realm,
            2 => self.kingdom,
            3 => self.subkingdom,
            4 => self.superphylum,
            5 => self.phylum,
            6 => self.subphylum,
            7 => self.superclass,
            8 => self.class,
            9 => self.subclass,
            10 => self.superorder,
            11 => self.order,
            12 => self.suborder,
            13 => self.infraorder,
            14 => self.superfamily,
            15 => self.family,
            16 => self.subfamily,
            17 => self.tribe,
            18 => self.subtribe,
            19 => self.genus,
            20 => self.subgenus,
            21 => self.species_group,
            22 => self.species_subgroup,
            23 => self.species,
            24 => self.subspecies,
            25 => self.strain,
            26 => self.varietas,
            27 => self.forma,
            _ => None
        }
    }
}

pub struct LineageStore {
    // Keep track of all lineages (id -> lineage)
    pub mapper: HashMap<u32, Arc<Lineage>>,
    // Make it possible to retrieve a Lineage based upon the values in one of its columns.
    pub index_references: Vec<HashMap<u32, Vec<Arc<Lineage>>>>
}

impl LineageStore {
    /// The number of rank columns a lineage row carries. `LineageRank::LINEAGE_ORDER` names them.
    pub const AMOUNT_OF_RANKS: usize = 28;

    /// The lineage column a rank name addresses, keyed on the spelling the columns use:
    /// `species_group` with an underscore. A caller that holds a `LineageRank` has
    /// [`LineageRank::lineage_index`] instead.
    pub fn rank_to_idx(s: &str) -> Option<usize> {
        match s {
            "domain" => Some(0),
            "realm" => Some(1),
            "kingdom" => Some(2),
            "subkingdom" => Some(3),
            "superphylum" => Some(4),
            "phylum" => Some(5),
            "subphylum" => Some(6),
            "superclass" => Some(7),
            "class" => Some(8),
            "subclass" => Some(9),
            "superorder" => Some(10),
            "order" => Some(11),
            "suborder" => Some(12),
            "infraorder" => Some(13),
            "superfamily" => Some(14),
            "family" => Some(15),
            "subfamily" => Some(16),
            "tribe" => Some(17),
            "subtribe" => Some(18),
            "genus" => Some(19),
            "subgenus" => Some(20),
            "species_group" => Some(21),
            "species_subgroup" => Some(22),
            "species" => Some(23),
            "subspecies" => Some(24),
            "strain" => Some(25),
            "varietas" => Some(26),
            "forma" => Some(27),
            _ => None
        }
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

            let lin = Arc::new(Lineage {
                domain: parts[0],
                realm: parts[1],
                kingdom: parts[2],
                subkingdom: parts[3],
                superphylum: parts[4],
                phylum: parts[5],
                subphylum: parts[6],
                superclass: parts[7],
                class: parts[8],
                subclass: parts[9],
                superorder: parts[10],
                order: parts[11],
                suborder: parts[12],
                infraorder: parts[13],
                superfamily: parts[14],
                family: parts[15],
                subfamily: parts[16],
                tribe: parts[17],
                subtribe: parts[18],
                genus: parts[19],
                subgenus: parts[20],
                species_group: parts[21],
                species_subgroup: parts[22],
                species: parts[23],
                subspecies: parts[24],
                strain: parts[25],
                varietas: parts[26],
                forma: parts[27]
            });

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
    pub fn get_all_taxon_ids_at_rank(&self, rank: &LineageRank) -> Option<Vec<u32>> {
        rank.lineage_index()
            .and_then(|idx| self.index_references.get(idx))
            .map(|map| map.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANK_KEYS: [&str; 28] = [
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
        "species_group",
        "species_subgroup",
        "species",
        "subspecies",
        "strain",
        "varietas",
        "forma"
    ];

    /// The rank names index the lineage columns in order, and each one reads back its own column.
    ///
    /// `rank_to_idx`, `get_taxon_id_at_rank` and `get_rank` are three hand-written tables over the
    /// same 28 ranks, listed in the same order, and nothing else checks that they agree with each
    /// other or with the column order the parser fills.
    #[test]
    fn every_rank_key_addresses_its_own_column() {
        let mut lineage = Lineage::default();
        let fields: [&mut Option<i32>; 28] = [
            &mut lineage.domain,
            &mut lineage.realm,
            &mut lineage.kingdom,
            &mut lineage.subkingdom,
            &mut lineage.superphylum,
            &mut lineage.phylum,
            &mut lineage.subphylum,
            &mut lineage.superclass,
            &mut lineage.class,
            &mut lineage.subclass,
            &mut lineage.superorder,
            &mut lineage.order,
            &mut lineage.suborder,
            &mut lineage.infraorder,
            &mut lineage.superfamily,
            &mut lineage.family,
            &mut lineage.subfamily,
            &mut lineage.tribe,
            &mut lineage.subtribe,
            &mut lineage.genus,
            &mut lineage.subgenus,
            &mut lineage.species_group,
            &mut lineage.species_subgroup,
            &mut lineage.species,
            &mut lineage.subspecies,
            &mut lineage.strain,
            &mut lineage.varietas,
            &mut lineage.forma
        ];
        for (position, field) in fields.into_iter().enumerate() {
            *field = Some(position as i32 + 1000);
        }

        for (position, key) in RANK_KEYS.iter().enumerate() {
            assert_eq!(LineageStore::rank_to_idx(key), Some(position), "rank_to_idx({key})");
            assert_eq!(lineage.get_taxon_id_at_rank(key), Some(position as i32 + 1000), "get_taxon_id_at_rank({key})");
            assert_eq!(lineage.get_rank(position), Some(position as i32 + 1000), "get_rank({position})");
        }
    }

    #[test]
    fn an_unknown_rank_key_addresses_nothing() {
        assert_eq!(LineageStore::rank_to_idx("nonsense"), None);
        assert_eq!(Lineage::default().get_taxon_id_at_rank("nonsense"), None);
        assert_eq!(Lineage::default().get_rank(LineageStore::AMOUNT_OF_RANKS), None);
    }
}
