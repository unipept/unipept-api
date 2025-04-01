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

const COUNT_THRESHOLD: u32 = 1000;

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
    if filter.is_empty() {
        return Ok(COUNT_THRESHOLD);
    }

    let filter_pattern = format!("{}*", filter);

    #[derive(QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total_count: i64,
    }
    
    let query: CountResult = sql_query(
        "SELECT COUNT(*) AS total_count FROM (
            SELECT `uniprot_entries`.`uniprot_accession_number`
            FROM `uniprot_entries`
            WHERE (
                MATCH(`uniprot_entries`.`name`) AGAINST (? IN BOOLEAN MODE) 
                OR MATCH(`uniprot_entries`.`uniprot_accession_number`) AGAINST (? IN BOOLEAN MODE)
                OR (`uniprot_entries`.`taxon_id` = ?)
            )
            LIMIT ?
        ) AS subquery"
    )
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
/// * `sort_by` - Field to sort results by (name, uniprot_accession_number, or taxon_id)
/// * `sort_descending` - Whether to sort in descending order
///
/// # Returns
/// * Vector of UniProt accession IDs that match the filter criteria
/// * `DatabaseError` if the database operation fails
///
/// This function returns UniProt accession IDs where either:
/// - Entry name contains the filter string (case-sensitive)
/// - UniProt accession number contains the filter string
/// - Taxon ID exactly matches filter string if it can be parsed as u32
///
/// The filter is applied as a partial match (using SQL LIKE with wildcards),
/// except for taxon_id which requires an exact match.
/// Results are paginated based on start/end indices and can be sorted by the specified field.
#[allow(clippy::needless_late_init)]
pub fn get_accessions_by_filter(
    conn: &mut MysqlConnection,
    filter: String,
    start: usize,
    end: usize,
    sort_by: String,
    sort_descending: bool,
) -> Result<Vec<String>, DatabaseError> {
    // Define filter pattern with `*` for prefix matching in BOOLEAN MODE
    let filter_pattern = if filter.is_empty() {
        String::new()
    } else {
        format!("{}*", filter)
    };

    #[derive(QueryableByName)]
    struct AccessionResult {
        #[diesel(sql_type = diesel::sql_types::Text)]
        uniprot_accession_number: String,
    }

    let base_query = {
        let mut sql = String::from(
            "SELECT `uniprot_entries`.`uniprot_accession_number` \
            FROM `uniprot_entries` ",
        );

        // Build conditions for FILTER (MATCH, taxon_id)
        if !filter.is_empty() {
            sql.push_str(
                " WHERE (MATCH(`uniprot_entries`.`name`) AGAINST (? IN BOOLEAN MODE) \
                OR MATCH(`uniprot_entries`.`uniprot_accession_number`) AGAINST (? IN BOOLEAN MODE) \
                OR `uniprot_entries`.`taxon_id` = ?) ",
            );
        }

        // Append ORDER BY logic
        match sort_by.as_str() {
            "name" => sql.push_str(&format!(
                "ORDER BY `uniprot_entries`.`name` {} ",
                if sort_descending { "DESC" } else { "ASC" }
            )),
            "uniprot_accession_number" => sql.push_str(&format!(
                "ORDER BY `uniprot_entries`.`uniprot_accession_number` {} ",
                if sort_descending { "DESC" } else { "ASC" }
            )),
            "taxon_id" => sql.push_str(&format!(
                "ORDER BY `uniprot_entries`.`taxon_id` {} ",
                if sort_descending { "DESC" } else { "ASC" }
            )),
            _ => (), // No ordering
        }

        // Append LIMIT and OFFSET for pagination
        sql.push_str("LIMIT ? OFFSET ?");

        sql
    };
    
    let results: Vec<AccessionResult>;
    
    if !filter.is_empty() {
        let query = sql_query(base_query)
            .bind::<Text, _>(&filter_pattern)
            .bind::<Text, _>(&filter_pattern)         // For MATCH on uniprot_accession_number
            .bind::<Unsigned<Integer>, _>(filter.parse::<u32>().unwrap_or(0)) // For taxon_id
            .bind::<BigInt, _>(end as i64 - start as i64) // LIMIT clause
            .bind::<BigInt, _>(start as i64);           // OFFSET clause

        // Execute the query and collect the results
        results = query.get_results(conn)?;
    } else {
        let query = sql_query(base_query)
            .bind::<BigInt, _>(end as i64 - start as i64) // LIMIT clause
            .bind::<BigInt, _>(start as i64);           // OFFSET clause

        // Execute the query and collect the results
        results = query.get_results(conn)?;
    }

    // Map the results to a vector of accession numbers
    Ok(results
        .into_iter()
        .map(|r| r.uniprot_accession_number)
        .collect())
}

