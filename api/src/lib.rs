use std::sync::Arc;

use datastore::DataStore;
use index::Index;
use tokio::net::TcpListener;

pub mod routes;
pub mod errors;
pub mod controllers;
pub mod helpers;

#[derive(Clone)]
pub struct AppState {
    pub datastore: Arc<DataStore>,
    pub index: Arc<Index>
}

const DEBUG_DATA_FOLDER_RICK: &str = "/mnt/data/uniprot-2024-03/suffix-array";

pub async fn start() -> Result<(), errors::AppError> {
    let sampledata = format!("{}/datastore/sampledata.json", DEBUG_DATA_FOLDER_RICK);
    let ec_numbers = format!("{}/datastore/ec_numbers.tsv", DEBUG_DATA_FOLDER_RICK);
    let go_terms = format!("{}/datastore/go_terms.tsv", DEBUG_DATA_FOLDER_RICK);
    let interpro_entries = format!("{}/datastore/interpro_entries.tsv", DEBUG_DATA_FOLDER_RICK);
    let lineages = format!("{}/datastore/lineages.tsv", DEBUG_DATA_FOLDER_RICK);
    let taxons = format!("{}/datastore/taxons.tsv", DEBUG_DATA_FOLDER_RICK);

    let sa = format!("{}/sa.bin", DEBUG_DATA_FOLDER_RICK);
    let proteins = format!("{}/proteins.tsv", DEBUG_DATA_FOLDER_RICK);

    let datastore = DataStore::try_from_files(
        "2024.03", sampledata.as_str(), ec_numbers.as_str(), go_terms.as_str(), interpro_entries.as_str(), lineages.as_str(), taxons.as_str()
    ).map_err(|_| errors::AppError::ServerStartError)?;

    let index = Index::try_from_files(
        sa.as_str(), proteins.as_str(), taxons.as_str()
    ).map_err(|_| errors::AppError::ServerStartError)?;

    let app_state = AppState { 
        datastore: Arc::new(datastore),
        index: Arc::new(index)
    };
        
    let router = routes::create_router(app_state);
    
    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    eprintln!("Server running on: http://{}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}
