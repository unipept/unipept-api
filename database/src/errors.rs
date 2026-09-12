use opensearch::{Error as ClientError, http::transport::BuildError};
use thiserror::Error;
use url::ParseError;

use crate::MAX_RESULT_WINDOW;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to build OpenSearch pool")]
    BuildPoolError(#[from] BuildError),
    #[error("Failed to parse OpenSearch URL")]
    ParsePoolUrlError(#[from] ParseError),
    #[error("Failed to retrieve documents from OpenSearch")]
    RetrieveError(#[from] ClientError),
    #[error(
        "cannot page from {start} to {end} of {total} entries: only the first {window} and the last {window} can be \
         reached",
        window = MAX_RESULT_WINDOW
    )]
    WindowUnreachable { start: usize, end: usize, total: usize },
    #[error("{0}")]
    GeneralError(String)
}
