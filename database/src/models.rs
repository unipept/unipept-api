use diesel::prelude::*;
use crate::schema::uniprot_entries;

#[derive(Selectable, Queryable, Debug)]
#[diesel(table_name = uniprot_entries)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct UniprotEntry {
    pub id: u32,
    pub uniprot_accession_number: String,
    pub version: u32,
    pub taxon_id: u32,
    pub db_type: String,
    pub name: String,
    pub protein: String,
    pub fa: String,
}
