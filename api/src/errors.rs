use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("Data store error: {0}")]
    DataStoreError(#[from] datastore::DataStoreError),
    #[error("Index error: {0}")]
    IndexError(#[from] index::IndexError),
    #[error("Database error: {0}")]
    DatabaseError(#[from] database::DatabaseError)
}
