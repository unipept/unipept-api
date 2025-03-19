use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    sync::Arc
};

use serde::Serialize;

use crate::errors::LineageStoreError;

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
            _ => None,
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
    const AMOUNT_OF_RANKS: usize = 26;

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
            _ => None,
        }
    }

    pub fn try_from_file(file: &str) -> Result<Self, LineageStoreError> {
        let file = std::fs::File::open(file).map_err(
            |_| LineageStoreError::FileNotFound(file.to_string())
        )?;

        let mut mapper = HashMap::new();

        let mut index_references: Vec<HashMap<u32, Vec<Arc<Lineage>>>> = Vec::new();

        for _ in 0..LineageStore::AMOUNT_OF_RANKS {
            index_references.push(HashMap::new());
        }

        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut splitted_line = line.split('\t');

            let taxon_id: u32 = splitted_line.next().unwrap().parse().unwrap();
            let parts: Vec<Option<i32>> =
                splitted_line.map(|x| if x == "\\N" { None } else { Some(x.parse::<i32>().unwrap()) }).collect();

            if parts.len() == 27 {
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

                for (i, part) in parts.iter().enumerate().take(LineageStore::AMOUNT_OF_RANKS) {
                    if let Some(id) = part {
                        let rank_map = index_references.get_mut(i).unwrap();
                        let id: u32 = id.unsigned_abs();
                        rank_map.entry(id).or_insert_with(Vec::new);
                        let vec = rank_map.get_mut(&id).unwrap();
                        vec.push(Arc::clone(&lin));
                    }
                }

            }
        }

        Ok(Self { mapper, index_references })
    }

    pub fn get(&self, key: u32) -> Option<&Arc<Lineage>> {
        self.mapper.get(&key)
    }

    pub fn get_lineages_at_rank(&self, rank: &str, taxon_id: u32) -> Option<&Vec<Arc<Lineage>>> {
        LineageStore::rank_to_idx(rank)
            .and_then(|idx| self.index_references.get(idx))
            .and_then(|map| map.get(&taxon_id))
    }

    /// Returns all unique taxon IDs at a specific rank in the NCBI taxonomy.
    pub fn get_all_taxon_ids_at_rank(&self, rank: &str) -> Option<Vec<u32>> {
        LineageStore::rank_to_idx(rank)
            .and_then(|idx| self.index_references.get(idx)).map(|map| map.keys().cloned().collect())
    }
}
