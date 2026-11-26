use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// An error that knows which HTTP status to send. Handlers return
/// `Result<_, ApiError>`, and axum calls `into_response` on the error for us.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!(message = %self.message, "request failed");
        }
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

/// Implementing `From` lets `?` convert core errors into HTTP errors automatically.
impl From<akarbox_core::Error> for ApiError {
    fn from(err: akarbox_core::Error) -> Self {
        use akarbox_core::Error::*;
        let status = match err {
            UnknownFormat(_) | NoPath { .. } | InvalidOptions(_) | InvalidPages(_) => {
                StatusCode::BAD_REQUEST
            }
            Unsupported(_) => StatusCode::UNPROCESSABLE_ENTITY,
            CommandFailed { .. } => StatusCode::UNPROCESSABLE_ENTITY, // usually a bad input file
            MissingOutput(_) | Io(_) | Zip(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        akarbox_core::Error::from(err).into()
    }
}
