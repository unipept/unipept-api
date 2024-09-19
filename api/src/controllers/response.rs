use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response}
};

pub struct HtmlTemplate<T>(pub T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(body) => Html(body).into_response(),
            Err(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to render template. Error: {err}")).into_response()
            }
        }
    }
}
