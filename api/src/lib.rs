use std::sync::Arc;

use datastore::DataStore;
use tokio::net::TcpListener;

pub mod routes;
pub mod errors;
pub mod controllers;

#[derive(Clone)]
pub struct AppState {
    pub datastore: Arc<DataStore>
}

pub async fn start() -> Result<(), errors::AppError> {
    let datastore = DataStore::try_from_files(
        "2024.01", "sampledata.json", "ec_numbers.tsv", "go_terms.tsv", "interpro_entries.tsv"
    ).map_err(|_| errors::AppError::ServerStartError)?;

    let app_state = AppState { datastore: Arc::new(datastore) };
        
    let router = routes::create_routes(app_state);
    
    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}
