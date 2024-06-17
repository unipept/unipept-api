use std::{collections::HashMap, io::{BufRead, BufReader}};

pub type InterproEntryDescription = (String, String);

#[derive(Clone)]
pub struct InterproStore {
    mapper: HashMap<String, InterproEntryDescription>
}

impl InterproStore {
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
