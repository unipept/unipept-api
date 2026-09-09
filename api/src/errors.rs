use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response}
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
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

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Json error")]
    JsonError(#[from] serde_json::Error),
    #[error("Database error")]
    DatabaseError(#[from] database::DatabaseError),
    #[error("Unknown rank error")]
    UnknownRankError(String),
    #[error("Join error")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Not implemented: {0}")]
    NotImplementedError(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String)
}

/// The body every error answers with.
///
/// A client parses one shape whatever went wrong, rather than a JSON body for a result and bare
/// text for a failure.
#[derive(Serialize)]
pub struct ErrorBody {
    error: String
}

/// A status and a JSON body saying what went wrong.
///
/// Rejections from the extractors answer through this too, so the shape does not depend on whether
/// a request failed before or inside a handler.
pub fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: message.into() })).into_response()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log the full error details
        eprintln!("API Error: {:?}", self);

        let (status, message) = match self {
            ApiError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
            ApiError::UnknownRankError(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::NotImplementedError(message) => (StatusCode::NOT_IMPLEMENTED, message),
            ApiError::InvalidParameter(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::JoinError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        };

        error_response(status, message)
    }
}
