use axum::{
    Json, RequestExt, async_trait,
    extract::{FromRequest, FromRequestParts, Multipart, RawForm, Request},
    http::{StatusCode, header::CONTENT_TYPE, request::Parts},
    response::{IntoResponse, Response}
};
use serde::de::DeserializeOwned;

pub struct GetContent<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for GetContent<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();

        // A percent escape that does not decode to UTF-8 — `%FF`, say — is client input, not an
        // impossible state: this used to unwrap, so the request took the handler down with it.
        let decoded =
            urlencoding::decode(query).map_err(|_| (StatusCode::BAD_REQUEST, "invalid query string encoding"))?;

        Ok(Self(serde_qs::from_str(&decoded).map_err(|_| (StatusCode::BAD_REQUEST, "invalid query string"))?))
    }
}

pub struct Form<T>(T);

#[async_trait]
impl<S, T> FromRequest<S> for Form<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let RawForm(form) = req
            .extract()
            .await
            .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body").into_response())?;

        let form_bytes = form.to_vec();
        let decoded_bytes = urlencoding::decode_binary(&form_bytes);

        Ok(Self(serde_qs::from_bytes(&decoded_bytes).map_err(|_| StatusCode::BAD_REQUEST.into_response())?))
    }
}

pub struct MultiPart<T>(T);

#[async_trait]
impl<S, T> FromRequest<S> for MultiPart<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body").into_response())?;

        // Every step here reads client-supplied bytes and every one of them used to unwrap: a
        // truncated body, a field with no name, or a field whose bytes are not UTF-8 each panicked
        // the handler rather than returning a status.
        let mut querystring = String::new();
        loop {
            let field = multipart
                .next_field()
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "malformed multipart body").into_response())?;

            let Some(field) = field else { break };

            let name = field
                .name()
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "multipart field without a name").into_response())?
                .to_string();

            let value = field
                .text()
                .await
                .map_err(|_| (StatusCode::BAD_REQUEST, "multipart field is not valid text").into_response())?;

            querystring.push_str(&format!("{}={}&", name, value));
        }

        Ok(Self(serde_qs::from_str(&querystring).map_err(|_| StatusCode::BAD_REQUEST.into_response())?))
    }
}

pub struct PostContent<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for PostContent<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    Form<T>: FromRequest<()>,
    MultiPart<T>: FromRequest<()>,
    T: 'static + DeserializeOwned
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(payload) = req
                    .extract()
                    .await
                    .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body").into_response())?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(payload) = req
                    .extract()
                    .await
                    .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body").into_response())?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("multipart/form-data") {
                let MultiPart(payload) = req
                    .extract()
                    .await
                    .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body").into_response())?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}
