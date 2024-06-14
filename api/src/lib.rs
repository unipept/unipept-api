use datastore::sampledata::SampleData;
use tokio::net::TcpListener;

pub mod routes;
pub mod errors;
pub mod controllers;

pub struct AppState {
    pub sample_data: SampleData
}

pub async fn start() -> Result<(), errors::AppError> {
    let sample_data = SampleData::try_from_json_file("sampledata.json").map_err(|_| errors::AppError::ServerStartError)?;

    let app_state = AppState { sample_data };
    
    let app = routes::create_routes(app_state);
    
    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
