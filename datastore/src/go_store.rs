use std::{collections::HashMap, io::{BufRead, BufReader}};

pub type GoTermDescription = (String, String);

#[derive(Clone)]
pub struct GoStore {
    mapper: HashMap<String, GoTermDescription>
}

impl GoStore {
    pub fn try_from_file(file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(file)?;

        let mut mapper = HashMap::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() == 4 {
                mapper.insert(parts[1].to_string(), (
                    parts[2].to_string(),
                    parts[3].to_string()
                ));
            }
        }

        Ok(GoStore { mapper })
    }

    pub fn get(&self, key: &str) -> Option<&GoTermDescription> {
        self.mapper.get(key)
    }
}
