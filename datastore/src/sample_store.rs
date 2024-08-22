use core::str;

use serde::{Deserialize, Serialize};

use crate::errors::{LineageStoreError, SampleStoreError};

#[derive(Clone, Serialize, Deserialize)]
pub struct SampleStore {
    sample_data: Vec<SampleDataItem>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SampleDataItem {
    id: i32,
    environment: String,
    reference: String,
    url: String,
    project_website: String,
    datasets: Vec<Dataset>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Dataset {
    name: String,
    order: i32,
    data: Vec<String>
}

impl SampleStore {
    pub fn try_from_file(file: &str) -> Result<Self, SampleStoreError> {
        let json = std::fs::read_to_string(file).map_err(
            |_| SampleStoreError::FileNotFound(file.to_string())
        )?;
        Ok(serde_json::from_str(&json)?)
    }
}
