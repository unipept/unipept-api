use axum::{
    Json, RequestExt,
    extract::{FromRequest, FromRequestParts, Multipart, RawForm, Request},
    http::{StatusCode, header::CONTENT_TYPE, request::Parts},
    response::{IntoResponse, Response}
};
use serde::de::DeserializeOwned;
use serde_qs::{Config, DuplicateKeyBehavior};

/// The query-string parser every body path shares.
///
/// `DuplicateKeyBehavior::Error` is the whole reason this is a `Config` rather than the free
/// functions: the default takes the last of a repeated scalar, so `?tryptic=true&tryptic=false`
/// would silently resolve to one of the two. A request that states a flag both ways states no
/// intention worth guessing at, and is refused.
///
/// `use_form_encoding` decides when a key is decoded. With it on, `input%5B%5D` is decoded before
/// the brackets are read and is therefore the array `input[]`; with it off the brackets are read
/// first, the key is a name with two odd characters in it, it matches no field and the value is
/// dropped. `URLSearchParams`, a browser form and `$.param` all encode brackets, so off means
/// answering those clients with an empty result and a 200. It is spelled out rather than left to
/// `Config::new`, which reads it from a Cargo feature — `default_to_form_encoding`, enabled by any
/// crate in the graph, would otherwise flip all three paths at once.
///
/// Shared by all three paths deliberately. Parsing that differs by content type is a way for the
/// same request to be accepted as a form body and refused as a query string.
const QS: Config = Config::new().duplicate_key_behavior(DuplicateKeyBehavior::Error).use_form_encoding(true);

pub struct GetContent<T>(pub T);

impl<S, T> FromRequestParts<S> for GetContent<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();

        // Handed to `serde_qs` still encoded, deliberately. It percent-decodes each key and value
        // *after* splitting on `&` and `=`, so a separator encoded inside a value stays part of
        // that value. Decoding the whole string first turned `filter=A%26equate_il%3Dtrue` into two
        // parameters and let a request set a flag it never sent. Invalid UTF-8 comes back as a
        // deserialisation error rather than a panic, which is what the unwrap here used to be.
        Ok(Self(QS.deserialize_str(query).map_err(|_| (StatusCode::BAD_REQUEST, "invalid query string"))?))
    }
}

pub struct Form<T>(T);

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

        // Undecoded, for the same reason as `GetContent` above.
        Ok(Self(QS.deserialize_bytes(&form).map_err(|_| StatusCode::BAD_REQUEST.into_response())?))
    }
}

pub struct MultiPart<T>(T);

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
        // truncated body, a field with no name, or a read that fails part-way through a field each
        // panicked the handler rather than returning a status. Note that `text()` decodes lossily,
        // so a stray byte is replaced rather than rejected — it fails on the stream, not on
        // encoding.
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
                .map_err(|_| (StatusCode::BAD_REQUEST, "could not read multipart field").into_response())?;

            querystring.push_str(&format!("{}={}&", name, value));
        }

        Ok(Self(QS.deserialize_str(&querystring).map_err(|_| StatusCode::BAD_REQUEST.into_response())?))
    }
}

pub struct PostContent<T>(pub T);

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
