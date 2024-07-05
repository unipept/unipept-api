use std::{
    collections::HashMap,
    io::{
        BufRead,
        BufReader
    },
    str::FromStr
};

use crate::errors::TaxonStoreError;

pub type TaxonInformation = (String, LineageRank);

#[derive(Debug, Clone)]
pub enum LineageRank {
    NoRank,
    Superkindom,
    Kingdom,
    Subkingdom,
    Superphylum,
    Phylum,
    Subphylum,
    Superclass,
    Class,
    Subclass,
    Superorder,
    Order,
    Suborder,
    Infraorder,
    Superfamily,
    Family,
    Subfamily,
    Tribe,
    Subtribe,
    Genus,
    Subgenus,
    SpeciesGroup,
    SpeciesSubgroup,
    Species,
    Subspecies,
    Strain,
    Varietas,
    Forma
}

pub struct TaxonStore {
    pub mapper: HashMap<u32, TaxonInformation>
}

impl TaxonStore {
    pub fn try_from_file(file: &str) -> Result<Self, TaxonStoreError> {
        let file = std::fs::File::open(file)?;

        let mut mapper = HashMap::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut splitted_line = line.split('\t');

            let taxon_id: u32 = splitted_line.next().unwrap().parse().unwrap();
            let parts: Vec<&str> = splitted_line.collect();

            if parts.len() == 4 {
                mapper.insert(taxon_id, (parts[0].to_string(), parts[1].parse::<LineageRank>()?));
            }
        }

        Ok(Self {
            mapper
        })
    }

    pub fn get(&self, key: u32) -> Option<&TaxonInformation> {
        self.mapper.get(&key)
    }
}

impl FromStr for LineageRank {
    type Err = TaxonStoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "no rank" => Ok(Self::NoRank),
            "superkingdom" => Ok(Self::Superkindom),
            "kingdom" => Ok(Self::Kingdom),
            "subkingdom" => Ok(Self::Subkingdom),
            "superphylum" => Ok(Self::Superphylum),
            "phylum" => Ok(Self::Phylum),
            "subphylum" => Ok(Self::Subphylum),
            "superclass" => Ok(Self::Superclass),
            "class" => Ok(Self::Class),
            "subclass" => Ok(Self::Subclass),
            "superorder" => Ok(Self::Superorder),
            "order" => Ok(Self::Order),
            "suborder" => Ok(Self::Suborder),
            "infraorder" => Ok(Self::Infraorder),
            "superfamily" => Ok(Self::Superfamily),
            "family" => Ok(Self::Family),
            "subfamily" => Ok(Self::Subfamily),
            "tribe" => Ok(Self::Tribe),
            "subtribe" => Ok(Self::Subtribe),
            "genus" => Ok(Self::Genus),
            "subgenus" => Ok(Self::Subgenus),
            "species group" => Ok(Self::SpeciesGroup),
            "species subgroup" => Ok(Self::SpeciesSubgroup),
            "species" => Ok(Self::Species),
            "subspecies" => Ok(Self::Subspecies),
            "strain" => Ok(Self::Strain),
            "varietas" => Ok(Self::Varietas),
            "forma" => Ok(Self::Forma),
            _ => Err(TaxonStoreError::InvalidRankError(s.to_string()))
        }
    }
}

impl Into<String> for LineageRank {
    fn into(self) -> String {
        match self {
            Self::NoRank => "no rank".to_string(),
            Self::Superkindom => "superkingdom".to_string(),
            Self::Kingdom => "kingdom".to_string(),
            Self::Subkingdom => "subkingdom".to_string(),
            Self::Superphylum => "superphylum".to_string(),
            Self::Phylum => "phylum".to_string(),
            Self::Subphylum => "subphylum".to_string(),
            Self::Superclass => "superclass".to_string(),
            Self::Class => "class".to_string(),
            Self::Subclass => "subclass".to_string(),
            Self::Superorder => "superorder".to_string(),
            Self::Order => "order".to_string(),
            Self::Suborder => "suborder".to_string(),
            Self::Infraorder => "infraorder".to_string(),
            Self::Superfamily => "superfamily".to_string(),
            Self::Family => "family".to_string(),
            Self::Subfamily => "subfamily".to_string(),
            Self::Tribe => "tribe".to_string(),
            Self::Subtribe => "subtribe".to_string(),
            Self::Genus => "genus".to_string(),
            Self::Subgenus => "subgenus".to_string(),
            Self::SpeciesGroup => "species group".to_string(),
            Self::SpeciesSubgroup => "species subgroup".to_string(),
            Self::Species => "species".to_string(),
            Self::Subspecies => "subspecies".to_string(),
            Self::Strain => "strain".to_string(),
            Self::Varietas => "varietas".to_string(),
            Self::Forma => "forma".to_string()
        }
    }
}
