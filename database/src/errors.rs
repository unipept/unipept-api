use opensearch::{Error as ClientError, http::transport::BuildError};
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to build OpenSearch pool: {0}")]
    BuildPoolError(#[from] BuildError),
    #[error("Failed to parse OpenSearch URL: {0}")]
    ParsePoolUrlError(#[from] ParseError),
    #[error("Failed to retrieve documents from OpenSearch URL: {0}")]
    RetrieveError(#[from] ClientError),
    #[error("{0}")]
    GeneralError(String)
}
