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

pub async fn start(index_location: &str, database_address: &str, port: u32) -> Result<(), errors::AppError> {
    let version = format!("{}/datastore/.version", index_location);
    let sampledata = format!("{}/datastore/sampledata.json", index_location);
    let ec_numbers = format!("{}/datastore/ec_numbers.tsv", index_location);
    let go_terms = format!("{}/datastore/go_terms.tsv", index_location);
    let interpro_entries = format!("{}/datastore/interpro_entries.tsv", index_location);
    let lineages = format!("{}/datastore/lineages.tsv", index_location);
    let taxons = format!("{}/datastore/taxons.tsv", index_location);

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

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    eprintln!("Server running on: http://{}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}
