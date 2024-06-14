use tokio::net::TcpListener;

pub mod routes;
pub mod errors;

pub async fn start() -> Result<(), errors::AppError> {
    let app = routes::create_routes();
    
    let listener = TcpListener::bind("0.0.0.0:4000").await?;

    axum::serve(listener, app).await?;

    Ok(())
}
