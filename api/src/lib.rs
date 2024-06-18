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

pub async fn start() -> Result<(), errors::AppError> {
    let datastore = DataStore::try_from_files(
        "2024.01", "data/sampledata.json", "data/ec_numbers.tsv", "data/go_terms.tsv", "data/interpro_entries.tsv", "data/lineages.tsv", "data/taxons.tsv"
    ).map_err(|_| errors::AppError::ServerStartError)?;

    let index = Index::try_from_files(
        "data/sa.bin", "data/proteins.tsv", "data/taxons.tsv"
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
