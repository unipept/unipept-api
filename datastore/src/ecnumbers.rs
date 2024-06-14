use std::{collections::HashMap, io::{BufRead, BufReader}};

#[derive(Clone)]
pub struct EcNumbers {
    mapper: HashMap<String, String>
}

impl EcNumbers {
    pub fn try_from_file(file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file)?;

        let mut mapper = HashMap::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 3 {
                mapper.insert(parts[1].to_string(), parts[2].to_string());
            }
        }

        Ok(EcNumbers { mapper })
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.mapper.get(key)
    }
}
