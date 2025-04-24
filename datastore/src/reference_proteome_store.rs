use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use crate::errors::ReferenceProteomeStoreError;

// Taxon id, protein count, protein list
pub type ReferenceProteomeDescription = (u32, u32, String);

#[derive(Clone)]
pub struct ReferenceProteomeStore {
    pub mapper: HashMap<String, ReferenceProteomeDescription>
}

impl ReferenceProteomeStore {
    pub fn try_from_file(file: &str) -> Result<Self, ReferenceProteomeStoreError> {
        let file = std::fs::File::open(file).map_err(
            |_| ReferenceProteomeStoreError::FileNotFound(file.to_string())
        )?;

        let mut mapper = HashMap::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 5 {
                let taxon_id = parts[2].parse::<u32>().map_err(|_| ReferenceProteomeStoreError::ParseError(format!("Could not parse taxon ID: {}", parts[2])))?;
                let protein_count = parts[3].parse::<u32>().map_err(|_| ReferenceProteomeStoreError::ParseError(format!("Could not parse protein count: {}", parts[3])))?;
                let proteins = parts[4].to_string();
                mapper.insert(parts[1].to_string(), (taxon_id, protein_count, proteins));
            }
        }

        Ok(ReferenceProteomeStore { mapper })
    }

    pub fn get(&self, key: &str) -> Option<&ReferenceProteomeDescription> {
        self.mapper.get(key)
    }
    
    pub fn get_taxon_id(&self, key: &str) -> Option<u32> {
        self.mapper.get(key).map(|(taxon_id, _, _)| *taxon_id)
    }
    
    pub fn get_protein_count(&self, key: &str) -> Option<u32> {
        self.mapper.get(key).map(|(_, protein_count, _)| *protein_count)
    }

    pub fn get_proteins(&self, key: &str) -> Option<Vec<&str>> {
        self.mapper.get(key).map(|(_, _, proteins)| proteins.split(';').filter(|s| !s.is_empty()).collect())
    }
}
