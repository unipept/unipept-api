use std::{collections::HashMap, ops::Deref};
use std::collections::HashSet;
use deadpool_diesel::mysql::{Manager, Object, Pool};
pub use deadpool_diesel::InteractError;
use diesel::{prelude::*, MysqlConnection, QueryDsl};
pub use errors::DatabaseError;
use models::UniprotEntry;
use itertools::Itertools;

mod errors;
mod models;
mod schema;

pub struct Database {
    pool: Pool
}

impl Database {
    pub fn try_from_url(url: &str) -> Result<Self, DatabaseError> {
        let manager = Manager::new(url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager).build().map_err(|err| DatabaseError::BuildPoolError(err.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn get_conn(&self) -> Result<Object, DatabaseError> {
        Ok(self.pool.get().await?)
    }
}

impl Deref for Database {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

pub fn get_accessions(
    conn: &mut MysqlConnection,
    accessions: &HashSet<String>
) -> Result<Vec<UniprotEntry>, DatabaseError> {
    use schema::uniprot_entries::dsl::*;

    let mut result: Vec<UniprotEntry> = Vec::new();

    accessions
        .iter()
        .chunks(1000)
        .into_iter()
        .for_each(|chunk| {
            let data = uniprot_entries.filter(uniprot_accession_number.eq_any(chunk)).load(conn);
            if let Ok(data) = data {
                result.extend(data);
            }
        });

    Ok(result)
}

pub fn get_accessions_map(
    conn: &mut MysqlConnection,
    accessions: &HashSet<String>
) -> Result<HashMap<String, UniprotEntry>, DatabaseError> {
    Ok(get_accessions(conn, accessions)?
        .into_iter()
        .map(|entry| (entry.uniprot_accession_number.clone(), entry))
        .collect())
}
