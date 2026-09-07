use std::{
    collections::HashMap,
    io::{BufRead, BufReader}
};

use crate::errors::ReferenceProteomeStoreError;

// Taxon id, protein count, protein list
pub type ReferenceProteomeDescription = (u32, u32, String);

#[derive(Clone)]
pub struct ReferenceProteomeStore {
    pub mapper: HashMap<String, ReferenceProteomeDescription>
}

impl ReferenceProteomeStore {
    pub fn try_from_file(file: &str) -> Result<Self, ReferenceProteomeStoreError> {
        let file =
            std::fs::File::open(file).map_err(|_| ReferenceProteomeStoreError::FileNotFound(file.to_string()))?;

        let mut mapper = HashMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line_number = index + 1;

            // Only a truly empty line: `trim()` would also erase a row of nothing but tabs, and
            // a row of delimiters is malformed input rather than an absence of input.
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 5 {
                return Err(ReferenceProteomeStoreError::UnexpectedColumnCount {
                    line: line_number,
                    expected: 5,
                    found: parts.len()
                });
            }

            let taxon_id = parts[2].parse::<u32>().map_err(|_| {
                ReferenceProteomeStoreError::ParseError(format!(
                    "Line {line_number}: could not parse taxon ID: {}",
                    parts[2]
                ))
            })?;
            let protein_count = parts[3].parse::<u32>().map_err(|_| {
                ReferenceProteomeStoreError::ParseError(format!(
                    "Line {line_number}: could not parse protein count: {}",
                    parts[3]
                ))
            })?;
            let proteins = parts[4].to_string();
            mapper.insert(parts[1].to_string(), (taxon_id, protein_count, proteins));
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
