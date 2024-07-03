use axum::{async_trait, extract::{FromRequest, FromRequestParts, Multipart, RawForm, Request}, http::{header::CONTENT_TYPE, request::Parts, StatusCode}, response::{IntoResponse, Response}, Json, RequestExt};
use serde::de::DeserializeOwned;

pub mod api;
pub mod datasets;
pub mod mpa;
pub mod private_api;

pub struct Query<T>(T);

#[async_trait]
impl<S, T> FromRequestParts<S> for Query<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().ok_or((StatusCode::BAD_REQUEST, "missing query string"))?;
        Ok(Self(serde_qs::from_str(&urlencoding::decode(query).unwrap()).map_err(|err| {
            eprintln!("{:?}", err);
            (StatusCode::BAD_REQUEST, "invalid query string")
        })?))
    }
}

pub struct Form<T>(T);

#[async_trait]
impl<S, T> FromRequest<S> for Form<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let RawForm(form) = req.extract().await.map_err(IntoResponse::into_response)?;

        let form_bytes = form.to_vec();
        let decoded_bytes = urlencoding::decode_binary(&form_bytes);

        Ok(Self(serde_qs::from_bytes(&decoded_bytes).map_err(|_| StatusCode::BAD_REQUEST.into_response())?))
    }
}

pub struct PostContent<T>(T);

#[async_trait]
impl<S, T> FromRequest<S> for PostContent<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    Form<T>: FromRequest<()>,
    T: 'static + DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }

            if content_type.starts_with("multipart/form-data") {
                let mut multipart = Multipart::from_request(req, _state).await.map_err(IntoResponse::into_response)?;

                let mut querystring = String::new();
                while let Some(field) = multipart.next_field().await.unwrap() {
                    let name = field.name().unwrap().to_string();
                    let value = field.text().await.unwrap();
                    querystring.push_str(&format!("{}={}&", name, value));
                }

                return Ok(Self(serde_qs::from_str(&querystring).map_err(|_| StatusCode::BAD_REQUEST.into_response())?));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

macro_rules! generate_handlers {
    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>
        ) -> Json<$ret:ty> $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>
        ) -> Json<$ret> $body

        pub async fn get_handler(
            state: State<$state_type>
        ) -> Json<$ret> {
            $handler_name(state).await
        }

        pub async fn post_handler(
            state: State<$state_type>
        ) -> Json<$ret> {
            $handler_name(state).await
        }
    };

    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty
        ) -> Json<$ret:ty> $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>,
            $params_pattern: $params_type
        ) -> Json<$ret> $body

        pub async fn get_handler(
            state: State<$state_type>,
            $crate::controllers::Query(params): $crate::controllers::Query<$params_type>
        ) -> Json<$ret> {
            $handler_name(state, params).await
        }

        pub async fn post_handler(
            state: State<$state_type>,
            $crate::controllers::PostContent(params): $crate::controllers::PostContent<$params_type>
        ) -> Json<$ret> {
            $handler_name(state, params).await
        }
    };

    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty,
            $version_param:ident : LineageVersion
        ) -> Json<$ret:ty> $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>,
            $params_pattern: $params_type,
            $version_param: LineageVersion
        ) -> Json<$ret> $body

        $(
            paste::paste! {
                pub async fn [<get_handler_ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::Query(params): $crate::controllers::Query<$params_type>
                ) -> Json<$ret> {
                    $handler_name(state, params, $version).await
                }

                pub async fn [<post_handler_ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::PostContent(params): $crate::controllers::PostContent<$params_type>
                ) -> Json<$ret> {
                    $handler_name(state, params, $version).await
                }
            }
        )*
    };
}

pub(crate) use generate_handlers;
