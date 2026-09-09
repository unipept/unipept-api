use std::{
    collections::HashMap,
    io::{BufRead, BufReader}
};

use crate::errors::InterproStoreError;

pub type InterproEntryDescription = (String, String);

#[derive(Clone)]
pub struct InterproStore {
    mapper: HashMap<String, InterproEntryDescription>
}

impl InterproStore {
    pub fn try_from_file(file: &str) -> Result<Self, InterproStoreError> {
        let file = std::fs::File::open(file).map_err(|_| InterproStoreError::FileNotFound(file.to_string()))?;

        let mut mapper = HashMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;

            // Only a truly empty line: `trim()` would also erase a row of nothing but tabs, and
            // a row of delimiters is malformed input rather than an absence of input.
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 4 {
                return Err(InterproStoreError::UnexpectedColumnCount {
                    line: index + 1,
                    expected: 4,
                    found: parts.len()
                });
            }

            // A row of the right width can still name nothing. Its entry would be reachable by no
            // lookup, and would be counted as though it were one.
            if parts[1].is_empty() {
                return Err(InterproStoreError::EmptyKey { line: index + 1 });
            }

            mapper.insert(parts[1].to_string(), (parts[2].to_string(), parts[3].to_string()));
        }

        Ok(InterproStore { mapper })
    }

    pub fn get(&self, key: &str) -> Option<&InterproEntryDescription> {
        self.mapper.get(key)
    }

    pub fn get_domain(&self, key: &str) -> Option<&str> {
        self.mapper.get(key).map(|(domain, _)| domain.as_str())
    }

    pub fn get_name(&self, key: &str) -> Option<&str> {
        self.mapper.get(key).map(|(_, name)| name.as_str())
    }
}
