use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    sync::Arc
};

use serde::Serialize;

use crate::errors::LineageStoreError;

#[derive(Clone, Debug, Serialize, Default)]
pub struct Lineage {
    pub superkingdom: Option<i32>,
    pub kingdom: Option<i32>,
    pub subkingdom: Option<i32>,
    pub superphylum: Option<i32>,
    pub phylum: Option<i32>,
    pub subphylum: Option<i32>,
    pub superclass: Option<i32>,
    pub class: Option<i32>,
    pub subclass: Option<i32>,
    pub infraclass: Option<i32>,
    pub superorder: Option<i32>,
    pub order: Option<i32>,
    pub suborder: Option<i32>,
    pub infraorder: Option<i32>,
    pub parvorder: Option<i32>,
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
            "superkingdom" => self.superkingdom,
            "kingdom" => self.kingdom,
            "subkingdom" => self.subkingdom,
            "superphylum" => self.superphylum,
            "phylum" => self.phylum,
            "subphylum" => self.subphylum,
            "superclass" => self.superclass,
            "class" => self.class,
            "subclass" => self.subclass,
            "infraclass" => self.infraclass,
            "superorder" => self.superorder,
            "order" => self.order,
            "suborder" => self.suborder,
            "infraorder" => self.infraorder,
            "parvorder" => self.parvorder,
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
            "superkingdom" => Some(0),
            "kingdom" => Some(1),
            "subkingdom" => Some(2),
            "superphylum" => Some(3),
            "phylum" => Some(4),
            "subphylum" => Some(5),
            "superclass" => Some(6),
            "class" => Some(7),
            "subclass" => Some(8),
            "superorder" => Some(9),
            "order" => Some(10),
            "suborder" => Some(11),
            "infraorder" => Some(12),
            "superfamily" => Some(13),
            "family" => Some(14),
            "subfamily" => Some(15),
            "tribe" => Some(16),
            "subtribe" => Some(17),
            "genus" => Some(18),
            "subgenus" => Some(19),
            "species_group" => Some(20),
            "species_subgroup" => Some(21),
            "species" => Some(22),
            "subspecies" => Some(23),
            "strain" => Some(24),
            "varietas" => Some(25),
            "forma" => Some(26),
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
                    superkingdom: parts[0],
                    kingdom: parts[1],
                    subkingdom: parts[2],
                    superphylum: parts[3],
                    phylum: parts[4],
                    subphylum: parts[5],
                    superclass: parts[6],
                    class: parts[7],
                    subclass: parts[8],
                    infraclass: None,
                    superorder: parts[9],
                    order: parts[10],
                    suborder: parts[11],
                    infraorder: parts[12],
                    parvorder: None,
                    superfamily: parts[13],
                    family: parts[14],
                    subfamily: parts[15],
                    tribe: parts[16],
                    subtribe: parts[17],
                    genus: parts[18],
                    subgenus: parts[19],
                    species_group: parts[20],
                    species_subgroup: parts[21],
                    species: parts[22],
                    subspecies: parts[23],
                    strain: parts[24],
                    varietas: parts[25],
                    forma: parts[26]
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
