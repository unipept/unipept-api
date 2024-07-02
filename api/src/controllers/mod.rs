use axum::{async_trait, extract::FromRequestParts, http::{request::Parts, StatusCode}};

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
        Ok(Self(serde_qs::from_str(query).map_err(|_| (StatusCode::BAD_REQUEST, "invalid query string"))?))
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
            Json(params): Json<$params_type>
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
                    Json(params): Json<$params_type>
                ) -> Json<$ret> {
                    $handler_name(state, params, $version).await
                }
            }
        )*
    };
}

pub(crate) use generate_handlers;
