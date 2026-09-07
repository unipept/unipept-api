use std::{
    collections::HashMap,
    io::{BufRead, BufReader}
};

use crate::errors::EcStoreError;

#[derive(Clone)]
pub struct EcStore {
    mapper: HashMap<String, String>
}

impl EcStore {
    pub fn try_from_file(file: &str) -> Result<Self, EcStoreError> {
        let file = std::fs::File::open(file).map_err(|_| EcStoreError::FileNotFound(file.to_string()))?;

        let mut mapper = HashMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;

            // Only a truly empty line: `trim()` would also erase a row of nothing but tabs, and
            // a row of delimiters is malformed input rather than an absence of input.
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 3 {
                return Err(EcStoreError::UnexpectedColumnCount { line: index + 1, expected: 3, found: parts.len() });
            }

            mapper.insert(parts[1].to_string(), parts[2].to_string());
        }

        Ok(EcStore { mapper })
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.mapper.get(key)
    }
}
