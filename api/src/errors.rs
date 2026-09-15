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
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("Data store error")]
    DataStoreError(#[from] datastore::DataStoreError),
    #[error("Index error")]
    IndexError(#[from] index::IndexError),
    #[error("Database error")]
    DatabaseError(#[from] database::DatabaseError)
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Json error")]
    JsonError(#[from] serde_json::Error),
    #[error("Database error")]
    DatabaseError(#[source] database::DatabaseError),
    // The payload is the message the caller is answered with; it names the rank itself, so the
    // variant adds no prefix of its own.
    #[error("{0}")]
    UnknownRankError(String),
    #[error("Join error")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Not implemented: {0}")]
    NotImplementedError(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String)
}

/// A database failure is a server fault, except where it is the caller's.
///
/// `DatabaseError` answers as a bare 500 with its message withheld, so nothing internal reaches a
/// client. `WindowUnreachable` is the one variant a caller causes rather than suffers: it names a
/// page the cluster can reach from neither end, and its message says what is reachable instead.
impl From<database::DatabaseError> for ApiError {
    fn from(error: database::DatabaseError) -> Self {
        match error {
            error @ database::DatabaseError::WindowUnreachable { .. } => ApiError::InvalidParameter(error.to_string()),
            error => ApiError::DatabaseError(error)
        }
    }
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

impl ApiError {
    /// Whether this service failed, or the caller asked for something it will not do.
    ///
    /// Not derivable from the status. `NotImplementedError` answers 501 for a combination of
    /// parameters the caller chose, which is a refusal rather than a fault, and an error rate
    /// counted from the status alone would page somebody for it.
    fn is_fault(&self) -> bool {
        matches!(self, ApiError::DatabaseError(_) | ApiError::JoinError(_))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Matched by reference, so `self` survives to be logged below. The three owned messages
        // are cloned rather than moved out; an error response is not a path where one `String`
        // matters.
        let (status, message) = match &self {
            ApiError::JsonError(_) => (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
            ApiError::UnknownRankError(message) => (StatusCode::BAD_REQUEST, message.clone()),
            ApiError::NotImplementedError(message) => (StatusCode::NOT_IMPLEMENTED, message.clone()),
            ApiError::InvalidParameter(message) => (StatusCode::BAD_REQUEST, message.clone()),
            ApiError::JoinError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        };

        // Recorded as `dyn Error` rather than as text: the subscriber walks `source()` and prints
        // the causes. The field is written unquoted, so any caller-supplied text a variant carries
        // has to reach its message through `{:?}`, or a newline in it forges a line in the journal.
        if self.is_fault() {
            tracing::error!(status = status.as_u16(), error = &self as &dyn std::error::Error, "request failed");
        } else {
            tracing::warn!(status = status.as_u16(), error = &self as &dyn std::error::Error, "request refused");
        }

        error_response(status, message)
    }
}
