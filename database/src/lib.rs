use std::{collections::HashMap, ops::Deref};
use std::collections::HashSet;
use deadpool_diesel::mysql::{Manager, Object, Pool};
pub use deadpool_diesel::InteractError;
use deadpool_diesel::ManagerConfig;
use diesel::{prelude::*, sql_query, MysqlConnection, QueryDsl};
use diesel::sql_types::{BigInt, Integer, Text, Unsigned};
pub use errors::DatabaseError;
use models::UniprotEntry;
use itertools::Itertools;

mod errors;
mod models;
mod schema;

pub struct Database {
    pool: Pool
}

const COUNT_THRESHOLD: u32 = 100000;

impl Database {
    pub fn try_from_url(url: &str) -> Result<Self, DatabaseError> {
        let manager = Manager::from_config(url, deadpool_diesel::Runtime::Tokio1, ManagerConfig {
            recycling_method: deadpool_diesel::RecyclingMethod::Verified
        });
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

/// Retrieves protein information from the database for a given set of UniProt accession IDs
///
/// # Arguments
/// * `conn` - Database connection handle 
/// * `accessions` - Set of UniProt accession IDs to retrieve data for
///
/// # Returns
/// * Vector of `UniprotEntry` records containing protein info from the database, ordered to match
///   the order of accessions in the input set
/// * `DatabaseError` if the database operation fails
pub fn get_accessions(
    conn: &mut MysqlConnection,
    accessions: &HashSet<String>,
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

/// Gets protein information as a map with UniProt accession IDs as keys and UniprotEntry objects as values
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `accessions` - Set of UniProt accession IDs to retrieve data for
///
/// # Returns
/// * HashMap mapping UniProt accession IDs to their corresponding UniprotEntry records
/// * `DatabaseError` if the database operation fails
///
/// This function returns the same protein information as `get_accessions()` but organized as a lookup map
/// instead of a vector, allowing direct access to entries by their accession ID.
pub fn get_accessions_map(
    conn: &mut MysqlConnection,
    accessions: &HashSet<String>,
) -> Result<HashMap<String, UniprotEntry>, DatabaseError> {
    Ok(get_accessions(conn, accessions)?
        .into_iter()
        .map(|entry| (entry.uniprot_accession_number.clone(), entry))
        .collect())
}

/// Counts the number of UniProt entries in the database that match the given filter string. Returns
/// COUNT_THRESHOLD if the number of matching items is more than this threshold.
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `filter` - String to filter entries by. If empty, returns total count of all entries
///
/// # Returns
/// * Number of matching entries (as u32)
/// * `DatabaseError` if the database operation fails
///
/// This function counts UniProt entries where either:
/// - Entry name contains the filter string (case-sensitive)
/// - UniProt accession number contains the filter string 
/// - Database type contains the filter string
/// - Taxon ID exactly matches filter string if it can be parsed as u32
///
/// The filter is applied as a partial match (using SQL LIKE with wildcards),
/// except for taxon_id which requires an exact match.
pub fn get_accessions_count_by_filter(
    conn: &mut MysqlConnection,
    filter: String,
) -> Result<u32, DatabaseError> {
    use schema::uniprot_entries::dsl::*;

    if filter.is_empty() {
        return Ok(COUNT_THRESHOLD);
    }

    let filter_pattern = format!("%{}%", filter);

    #[derive(QueryableByName)]
    struct CountResult {
        #[sql_type = "diesel::sql_types::BigInt"]
        total_count: i64,
    }


    let query: CountResult = sql_query(
        "SELECT COUNT(*) AS total_count FROM (
            SELECT `uniprot_entries`.`uniprot_accession_number`
            FROM `uniprot_entries`
            WHERE (
                (`uniprot_entries`.`name` LIKE ?) 
                OR (`uniprot_entries`.`uniprot_accession_number` LIKE ?)
                OR (`uniprot_entries`.`type` LIKE ?)
                OR (`uniprot_entries`.`taxon_id` = ?)
            )
            LIMIT ?
        ) AS subquery"
    )
        .bind::<Text, _>(filter_pattern.clone())
        .bind::<Text, _>(filter_pattern.clone())
        .bind::<Text, _>(filter_pattern.clone())
        .bind::<Unsigned<Integer>, _>(filter.parse::<u32>().unwrap_or(0)) // Replace "0" with taxon_id logic if needed
        .bind::<Unsigned<Integer>, _>(COUNT_THRESHOLD) // LIMIT clause value
        .get_result(conn)?; // Replace `conn` with your MySQL connection handle

    Ok(query.total_count as u32)
}

/// Gets UniProt accession IDs from the database that match the given filter criteria
///
/// # Arguments
/// * `conn` - Database connection handle
/// * `filter` - String to filter entries by. If empty, returns unfiltered results
/// * `start` - Starting index for pagination
/// * `end` - Ending index for pagination 
/// * `sort_by` - Field to sort results by (name, uniprot_accession_number, db_type, or taxon_id)
/// * `sort_descending` - Whether to sort in descending order
///
/// # Returns
/// * Vector of UniProt accession IDs that match the filter criteria
/// * `DatabaseError` if the database operation fails
///
/// This function returns UniProt accession IDs where either:
/// - Entry name contains the filter string (case-sensitive)
/// - UniProt accession number contains the filter string
/// - Database type contains the filter string
/// - Taxon ID exactly matches filter string if it can be parsed as u32
///
/// The filter is applied as a partial match (using SQL LIKE with wildcards),
/// except for taxon_id which requires an exact match.
/// Results are paginated based on start/end indices and can be sorted by the specified field.
pub fn get_accessions_by_filter(
    conn: &mut MysqlConnection,
    filter: String,
    start: usize,
    end: usize,
    sort_by: String,
    sort_descending: bool,
) -> Result<Vec<String>, DatabaseError> {
    use schema::uniprot_entries::dsl::*;

    let filter_pattern = format!("%{}%", filter);

    let mut query = uniprot_entries.select(uniprot_accession_number).into_boxed();

    if !filter.is_empty() {
        query = query.filter(
            name.like(&filter_pattern)
                .or(uniprot_accession_number.like(&filter_pattern))
                .or(db_type.like(&filter_pattern))
                .or(taxon_id.eq(filter.parse::<u32>().unwrap_or(0)))
        );
    }

    // Apply sorting
    if !sort_by.is_empty() {
        query = match sort_by.as_str() {
            "name" => if sort_descending { query.order(name.desc()) } else { query.order(name.asc()) },
            "uniprot_accession_number" => if sort_descending { query.order(uniprot_accession_number.desc()) } else { query.order(uniprot_accession_number.asc()) },
            "db_type" => if sort_descending { query.order(db_type.desc()) } else { query.order(db_type.asc()) },
            "taxon_id" => if sort_descending { query.order(taxon_id.desc()) } else { query.order(taxon_id.asc()) },
            _ => query
        };
    }

    // Apply pagination
    query = query.offset(start as i64).limit((end - start) as i64);

    Ok(query.load::<String>(conn)?)
}
