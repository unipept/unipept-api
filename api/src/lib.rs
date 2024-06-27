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

pub async fn start(index_location: &str) -> Result<(), errors::AppError> {
    let sampledata = format!("{}/datastore/sampledata.json", index_location);
    let ec_numbers = format!("{}/datastore/ec_numbers.tsv", index_location);
    let go_terms = format!("{}/datastore/go_terms.tsv", index_location);
    let interpro_entries = format!("{}/datastore/interpro_entries.tsv", index_location);
    let lineages = format!("{}/datastore/lineages.tsv", index_location);
    let taxons = format!("{}/datastore/taxons.tsv", index_location);

    let sa = format!("{}/sa.bin", index_location);
    let proteins = format!("{}/proteins.tsv", index_location);

    let datastore = DataStore::try_from_files(
        "2024.03", sampledata.as_str(), ec_numbers.as_str(), go_terms.as_str(), interpro_entries.as_str(), lineages.as_str(), taxons.as_str()
    )?;

    let index = Index::try_from_files(
        sa.as_str(), proteins.as_str(), taxons.as_str()
    )?;

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
