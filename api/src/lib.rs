use std::sync::Arc;

use axum::{extract::FromRef, Router};
use datastore::{ecnumbers::EcNumbers, goterms::GoTerms, sampledata::SampleData};
use tokio::net::TcpListener;

pub mod routes;
pub mod errors;
pub mod controllers;

#[derive(Clone)]
pub struct AppState {
    pub sample_state: SampleState,
    pub ec_state: EcState,
    pub go_state: GoState
}

#[derive(Clone)]
pub struct SampleState {
    pub samples: Arc<SampleData>
}

#[derive(Clone)]
pub struct EcState {
    pub ec_numbers: Arc<EcNumbers>
}

#[derive(Clone)]
pub struct GoState {
    pub go_terms: Arc<GoTerms>
}

impl FromRef<AppState> for SampleState {
    fn from_ref(state: &AppState) -> Self {
        state.sample_state.clone()
    }
}

impl FromRef<AppState> for EcState {
    fn from_ref(state: &AppState) -> Self {
        state.ec_state.clone()
    }
}

impl FromRef<AppState> for GoState {
    fn from_ref(state: &AppState) -> Self {
        state.go_state.clone()
    }
}

pub async fn start() -> Result<(), errors::AppError> {
    let sample_data = SampleData::try_from_file("sampledata.json").map_err(|_| errors::AppError::ServerStartError)?;
    let ec_numbers = EcNumbers::try_from_file("ec_numbers.tsv").map_err(|_| errors::AppError::ServerStartError)?;
    let go_terms = GoTerms::try_from_file("go_terms.tsv").map_err(|_| errors::AppError::ServerStartError)?;

    let app_state = AppState { 
        sample_state: SampleState { samples: Arc::new(sample_data) }, 
        ec_state: EcState { ec_numbers: Arc::new(ec_numbers) }, 
        go_state: GoState { go_terms: Arc::new(go_terms) } 
    };
        
    let router = routes::create_routes(app_state);
    
    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    axum::serve(listener, router).await?;

    Ok(())
}
