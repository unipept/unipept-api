use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("{0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("Failed to build database pool: {0}")]
    BuildPoolError(String),
    #[error("Failed to get database connection: {0}")]
    GetConnectionError(#[from] deadpool_diesel::PoolError)
}
