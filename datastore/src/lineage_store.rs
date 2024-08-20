use std::{
    collections::HashMap,
    io::{BufRead, BufReader}
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

pub struct LineageStore<'a> {
    pub mapper: HashMap<u32, Lineage>,
    pub index_references: Vec<HashMap<u32, &'a Lineage>>
}

impl<'a> LineageStore<'a> {
    pub fn try_from_file(file: &str) -> Result<Self, LineageStoreError> {
        let file = std::fs::File::open(file)?;

        let mut mapper = HashMap::new();

        let mut index_references: Vec<HashMap<u32, &'a Lineage>> = Vec::new();

        for i in 0..28 {
            index_references.push(HashMap::new());
        }

        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut splitted_line = line.split('\t');

            let taxon_id: u32 = splitted_line.next().unwrap().parse().unwrap();
            let parts: Vec<Option<i32>> =
                splitted_line.map(|x| if x == "\\N" { None } else { Some(x.parse::<i32>().unwrap()) }).collect();

            if parts.len() == 27 {
                let lin = Lineage {
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
                };

                mapper.insert(taxon_id, lin);
            }
        }

        for (taxon_id, lin) in &mapper {
            index_references.get_mut(0).unwrap().insert(lin.superkingdom.unwrap().abs() as u32, lin);
        }

        println!("Amount of lineages in database: {}", mapper.len());

        Ok(Self { mapper, index_references })
    }

    pub fn get(&self, key: u32) -> Option<&Lineage> {
        let result = self.mapper.get(&key);
        // We need to automatically dereference the value lin here to avoid having to return a
        // double reference.
        match result {
            Some(lin) => Some(lin),
            None => None
        }
    }
}
