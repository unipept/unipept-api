use std::sync::Arc;

use database::Database;
use datastore::DataStore;
use index::Index;
use tokio::net::TcpListener;

pub mod controllers;
pub mod errors;
pub mod helpers;
pub mod middleware;
pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub datastore: Arc<DataStore>,
    pub database: Arc<Database>,
    pub index: Arc<Index>
}

pub async fn start(
    index_location: &str,
    datastore_location: &str,
    database_address: &str
) -> Result<(), errors::AppError> {
    let version = format!("{}/.version", datastore_location);
    let sampledata = format!("{}/sampledata.json", datastore_location);
    let ec_numbers = format!("{}/ec_numbers.tsv", datastore_location);
    let go_terms = format!("{}/go_terms.tsv", datastore_location);
    let interpro_entries = format!("{}/interpro_entries.tsv", datastore_location);
    let lineages = format!("{}/lineages.tsv", datastore_location);
    let taxons = format!("{}/taxons.tsv", datastore_location);

    let sa = format!("{}/sa.bin", index_location);
    let proteins = format!("{}/proteins.tsv", index_location);

    let database = Database::try_from_url(database_address)?;

    let datastore = DataStore::try_from_files(
        &version,
        &sampledata,
        &ec_numbers,
        &go_terms,
        &interpro_entries,
        &lineages,
        &taxons
    )?;

    let index = Index::try_from_files(&sa, &proteins)?;

    let app_state = AppState {
        datastore: Arc::new(datastore),
        database: Arc::new(database),
        index: Arc::new(index)
    };

    let router = routes::create_router(app_state);

    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    eprintln!("Server running on: http://{}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}
