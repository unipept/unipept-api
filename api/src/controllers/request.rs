use axum::{
    Json, RequestExt,
    extract::{FromRequest, FromRequestParts, Multipart, RawForm, Request, rejection::JsonRejection},
    http::{StatusCode, header::CONTENT_TYPE, request::Parts},
    response::{IntoResponse, Response}
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned, Visitor}
};
use serde_qs::{Config, DuplicateKeyBehavior};

use crate::errors::reject;

/// The query-string parser every body path shares.
///
/// Both settings are non-default. `DuplicateKeyBehavior::Error` refuses `?tryptic=true&tryptic=false`
/// rather than taking the last of the two. `use_form_encoding` decodes a key before reading its
/// brackets, so the `input%5B%5D` that `URLSearchParams` and browser forms send is still the array
/// `input[]` — and it is spelled out because `Config::new` otherwise takes it from a Cargo feature
/// that any crate in the graph could enable.
///
/// Shared by all three paths: parsing that differs by content type lets one request be accepted as
/// a body and refused as a query string.
const QS: Config = Config::new().duplicate_key_behavior(DuplicateKeyBehavior::Error).use_form_encoding(true);

/// A boolean request parameter.
///
/// The strictness lives on the type rather than on 45 `deserialize_with` attributes. Every
/// `default_*` function returns a `Flag`, so a parameter field declared `bool` does not compile.
///
/// No `Default` impl, deliberately. A bare `#[serde(default)]` would otherwise accept a `bool`
/// field and lose that check, so every `Flag` field has to name a function that returns one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag(pub bool);

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        strict_bool(deserializer).map(Flag)
    }
}

impl Serialize for Flag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.0)
    }
}

/// Reads a boolean parameter, accepting only `true` and `false`.
///
/// The query-string parser reads `?tryptic=` and a bare `?tryptic` as `true`, which on a flag
/// defaulting to false is an instruction the caller never gave, answered 200.
///
/// Per field rather than in the extractor: `GetContent<T>` is generic and cannot know which of
/// `T`'s fields are booleans, and refusing every empty value would refuse `filter=`, which the
/// private-api filters take as a real request.
fn strict_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    struct StrictBool;

    impl Visitor<'_> for StrictBool {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("`true` or `false`")
        }

        // A JSON body carries a real boolean, and there is nothing to be strict about.
        fn visit_bool<E: de::Error>(self, value: bool) -> Result<bool, E> {
            Ok(value)
        }

        // A query string or a form body carries text, which is where the leniency lives.
        fn visit_str<E: de::Error>(self, value: &str) -> Result<bool, E> {
            match value {
                "true" => Ok(true),
                "false" => Ok(false),
                other => Err(E::custom(format!("expected `true` or `false`, got {other:?}")))
            }
        }
    }

    deserializer.deserialize_any(StrictBool)
}

pub struct GetContent<T>(pub T);

impl<S, T> FromRequestParts<S> for GetContent<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();

        // Handed to `serde_qs` still encoded, deliberately. It percent-decodes each key and value
        // *after* splitting on `&` and `=`, so a separator encoded inside a value stays part of
        // that value. Decoding the whole string first turned `filter=A%26equate_il%3Dtrue` into two
        // parameters and let a request set a flag it never sent. Invalid UTF-8 comes back as a
        // deserialisation error rather than a panic, which is what the unwrap here used to be.
        Ok(Self(
            QS.deserialize_str(query)
                .map_err(|error| reject(StatusCode::BAD_REQUEST, "invalid query string", Some(&error)))?
        ))
    }
}

/// Why a body could not be read.
///
/// `Form` and `MultiPart` carry no status of their own: `PostContent` answers 422 for every one of
/// its three branches, so a status chosen at the point of failure would be neither the one the
/// caller receives nor the one the log should name.
pub struct BodyRejection {
    reason: &'static str,
    cause: Option<Box<dyn std::error::Error + Send + Sync>>
}

impl BodyRejection {
    fn new(reason: &'static str, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self { reason, cause: Some(Box::new(cause)) }
    }

    /// A refusal with no underlying error: nothing failed, the body was not what it claimed.
    fn bare(reason: &'static str) -> Self {
        Self { reason, cause: None }
    }
}

/// Logs this refusal and answers it.
///
/// 422 for every one of them, because `PostContent` answers 422 for every one of them — the status
/// written to the log has to be the status the caller received.
impl IntoResponse for BodyRejection {
    fn into_response(self) -> Response {
        reject(
            StatusCode::UNPROCESSABLE_ENTITY,
            self.reason,
            self.cause.as_ref().map(|cause| cause.as_ref() as &(dyn std::error::Error + 'static))
        )
    }
}

pub struct Form<T>(T);

impl<S, T> FromRequest<S> for Form<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = BodyRejection;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let RawForm(form) = req.extract().await.map_err(|error| BodyRejection::new("Invalid request body", error))?;

        // Undecoded, for the same reason as `GetContent` above.
        Ok(Self(QS.deserialize_bytes(&form).map_err(|error| BodyRejection::new("invalid form body", error))?))
    }
}

pub struct MultiPart<T>(T);

impl<S, T> FromRequest<S> for MultiPart<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned
{
    type Rejection = BodyRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|error| BodyRejection::new("Invalid request body", error))?;

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
                .map_err(|error| BodyRejection::new("malformed multipart body", error))?;

            let Some(field) = field else { break };

            let name = field.name().ok_or_else(|| BodyRejection::bare("multipart field without a name"))?.to_string();

            let value =
                field.text().await.map_err(|error| BodyRejection::new("could not read multipart field", error))?;

            // Encoded rather than concatenated: `text()` has already decoded this part, so
            // appending it raw let a value carrying `&` become parameters of its own and a `+`
            // come back as a space. Same bug as `GetContent` describes above, from the other
            // side — there by decoding too early, here by never encoding.
            //
            // One serializer per field: it is not `Send`, so holding one across the `await` above
            // would make the whole future `!Send`. It appends to what it is given, so nothing is
            // buffered twice.
            form_urlencoded::Serializer::new(&mut querystring).append_pair(&name, &value);
        }

        Ok(Self(
            QS.deserialize_str(&querystring)
                .map_err(|error| BodyRejection::new("invalid multipart body", error))?
        ))
    }
}

pub struct PostContent<T>(pub T);

impl<S, T> FromRequest<S> for PostContent<T>
where
    S: Send + Sync,
    // The rejection types are named rather than left open: `Json`'s carries the reason a body did
    // not parse, and the other two carry a reason and a cause for this extractor to answer with.
    Json<T>: FromRequest<(), Rejection = JsonRejection>,
    Form<T>: FromRequest<(), Rejection = BodyRejection>,
    MultiPart<T>: FromRequest<(), Rejection = BodyRejection>,
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
                    .map_err(|error| reject(StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body", Some(&error)))?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(payload) = req.extract().await.map_err(BodyRejection::into_response)?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("multipart/form-data") {
                let MultiPart(payload) = req.extract().await.map_err(BodyRejection::into_response)?;
                return Ok(Self(payload));
            }
        }

        Err(reject(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported content type", None))
    }
}
