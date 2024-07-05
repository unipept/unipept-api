use std::collections::HashMap;
use std::ops::Deref;

use deadpool_diesel::mysql::{Manager, Pool};
use diesel::{MysqlConnection, QueryDsl};
use diesel::prelude::*;
pub use errors::DatabaseError;
use models::UniprotEntry;

mod errors;
mod schema;
mod models;

pub struct Database {
    pool: Pool
}

impl Database {
    pub fn try_from_url(url: &str) -> Result<Self, DatabaseError> {
        let manager = Manager::new(url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager).build().map_err(|err| DatabaseError::BuildPoolError(err.to_string()))?;        
        Ok(Self { pool })
    }
}

impl Deref for Database {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

pub fn get_accessions(conn: &mut MysqlConnection, accessions: &Vec<String>) -> Result<Vec<UniprotEntry>, DatabaseError> {
    use schema::uniprot_entries::dsl::*;

    Ok(uniprot_entries
        .filter(uniprot_accession_number.eq_any(accessions))
        .load(conn)?)
}

pub fn get_accessions_map(conn: &mut MysqlConnection, accessions: &Vec<String>) -> Result<HashMap<String, UniprotEntry>, DatabaseError> {
    Ok(
        get_accessions(conn, accessions)?
            .into_iter()
            .map(|entry| (entry.uniprot_accession_number.clone(), entry))
            .collect()
    )
}
