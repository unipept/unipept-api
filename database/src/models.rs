use serde::Deserialize;
use serde::de::{self, Deserializer};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct UniprotEntry {
    #[serde(alias = "uniprot_accession_number")]
    pub uniprot_accession_number: String,
    #[serde(alias = "version", deserialize_with = "string_to_u32")]
    pub version: u32,
    #[serde(alias = "taxon_id", deserialize_with = "string_to_u32")]
    pub taxon_id: u32,
    #[serde(alias = "type")]
    pub db_type: String,
    pub name: String,
    #[serde(alias = "sequence")]
    pub protein: String,
    pub fa: String
}

fn string_to_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?; // Deserialize input as a String
    u32::from_str(&s).map_err(de::Error::custom) // Convert the String to u32
}
