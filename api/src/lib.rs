use std::sync::Arc;

use axum::{
    ServiceExt, // for `into_make_service`
    extract::Request
};
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
    // First, so that a failure loading the database, the datastore, or the index is loggable too
    // — those are the failures that matter most.
    middleware::tracing::init_tracing_subscriber();

    let version = format!("{}/.version", index_location);

    let sampledata = format!("{}/datastore/sampledata.json", index_location);
    let ec_numbers = format!("{}/datastore/ec_numbers.tsv", index_location);
    let go_terms = format!("{}/datastore/go_terms.tsv", index_location);
    let interpro_entries = format!("{}/datastore/interpro_entries.tsv", index_location);
    let reference_proteomes = format!("{}/datastore/proteomes.tsv", index_location);
    let lineages = format!("{}/datastore/lineages.tsv", index_location);
    let taxons = format!("{}/datastore/taxons.tsv", index_location);

    let sa = format!("{}/sa.bin", index_location);
    let proteins = format!("{}/proteins.bin", index_location);
    let mappings = format!("{}/mapping.bin", index_location);
    let kmer_table = format!("{}/kmer_table.bin", index_location);

    let database = Database::try_from_url(database_address)?;

    let datastore = DataStore::try_from_files(
        &version,
        &sampledata,
        &ec_numbers,
        &go_terms,
        &interpro_entries,
        &reference_proteomes,
        &lineages,
        &taxons
    )?;

    // Neither the version nor the backend is recorded anywhere else, so this is the only way
    // to tell what is actually running. The configurations differ enough in memory profile to
    // be worth stating.
    tracing::info!(version = env!("CARGO_PKG_VERSION"), backend = %Index::backend_summary(), "starting unipept api");

    let index = Index::try_from_files(&sa, &proteins, &mappings, &kmer_table)?;

    let app_state = AppState {
        datastore: Arc::new(datastore),
        database: Arc::new(database),
        index: Arc::new(index)
    };

    let app = routes::create_app(app_state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();

    tracing::info!(address = %listener.local_addr()?, "listening");

    axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}
