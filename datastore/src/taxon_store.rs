use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    str::FromStr
};

use crate::errors::TaxonStoreError;

pub type TaxonInformation = (String, LineageRank, bool);

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
                mapper.insert(taxon_id, (
                    parts[0].to_string(), 
                    parts[1].parse::<LineageRank>()?,
                    match parts[2] {
                        "\x01" => true,
                        _ => false
                    }
                ));
            }
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

impl From<LineageRank> for String {
    fn from(val: LineageRank) -> Self {
        match val {
            LineageRank::NoRank => "no rank".to_string(),
            LineageRank::Superkindom => "superkingdom".to_string(),
            LineageRank::Kingdom => "kingdom".to_string(),
            LineageRank::Subkingdom => "subkingdom".to_string(),
            LineageRank::Superphylum => "superphylum".to_string(),
            LineageRank::Phylum => "phylum".to_string(),
            LineageRank::Subphylum => "subphylum".to_string(),
            LineageRank::Superclass => "superclass".to_string(),
            LineageRank::Class => "class".to_string(),
            LineageRank::Subclass => "subclass".to_string(),
            LineageRank::Superorder => "superorder".to_string(),
            LineageRank::Order => "order".to_string(),
            LineageRank::Suborder => "suborder".to_string(),
            LineageRank::Infraorder => "infraorder".to_string(),
            LineageRank::Superfamily => "superfamily".to_string(),
            LineageRank::Family => "family".to_string(),
            LineageRank::Subfamily => "subfamily".to_string(),
            LineageRank::Tribe => "tribe".to_string(),
            LineageRank::Subtribe => "subtribe".to_string(),
            LineageRank::Genus => "genus".to_string(),
            LineageRank::Subgenus => "subgenus".to_string(),
            LineageRank::SpeciesGroup => "species group".to_string(),
            LineageRank::SpeciesSubgroup => "species subgroup".to_string(),
            LineageRank::Species => "species".to_string(),
            LineageRank::Subspecies => "subspecies".to_string(),
            LineageRank::Strain => "strain".to_string(),
            LineageRank::Varietas => "varietas".to_string(),
            LineageRank::Forma => "forma".to_string()
        }
    }
}
