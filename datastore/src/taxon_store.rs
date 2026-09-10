use std::{
    collections::HashMap,
    io::{BufRead, BufReader}
};

use crate::{errors::TaxonStoreError, rank::TaxonRank};

pub type TaxonInformation = (String, TaxonRank, bool);

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
                .parse::<TaxonRank>()
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
