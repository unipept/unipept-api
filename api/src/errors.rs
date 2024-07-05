use axum::{
    http::StatusCode,
    response::{
        IntoResponse,
        Response
    }
};
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
    #[error("Database error")]
    DatabaseInteractionError(#[from] database::InteractError),
    #[error("Not implemented: {0}")]
    NotImplementedError(String)
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()),
            ApiError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            ApiError::DatabaseInteractionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            ApiError::NotImplementedError(message) => (StatusCode::NOT_IMPLEMENTED, message)
        };

        Response::builder()
            .status(status)
            .body(message.into())
            .unwrap()
    }
}

// impl IntoResponse for AppError {
//     fn into_response(self) -> Response {
//         let (status, message) = match self {
//             AppError::IoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
//             AppError::DataStoreError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server
// error"),             AppError::IndexError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal
// server error"),             AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR,
// "Internal server error")         };
//     }
// }
