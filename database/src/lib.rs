use std::ops::Deref;

use deadpool_diesel::mysql::{Manager, Pool};
use diesel::{MysqlConnection, QueryDsl};
use diesel::prelude::*;

mod schema;
mod models;

pub struct Database {
    pool: Pool
}

impl Database {
    pub fn try_from_url(url: &str) -> Self {
        let manager = Manager::new(url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager).build().unwrap();        
        Self { pool }
    }
}

pub fn get_accessions(conn: &mut MysqlConnection, accessions: &Vec<String>) -> Result<Vec<models::UniprotEntry>, diesel::result::Error> {
    use schema::uniprot_entries::dsl::*;

    uniprot_entries
        .filter(uniprot_accession_number.eq_any(accessions))
        .load(conn)
}

impl Deref for Database {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
