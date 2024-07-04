pub mod api;
pub mod datasets;
pub mod mpa;
pub mod private_api;
pub mod request;
pub mod response;

macro_rules! generate_handlers {
    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>
        ) -> $ret:ty $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>
        ) -> $ret $body

        pub async fn get_handler(
            state: State<$state_type>
        ) -> Json<$ret> {
            Json($handler_name(state).await)
        }

        pub async fn post_handler(
            state: State<$state_type>
        ) -> Json<$ret> {
            Json($handler_name(state).await)
        }
    };

    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty
        ) -> $ret:ty $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>,
            $params_pattern: $params_type
        ) -> $ret $body

        pub async fn get_handler(
            state: State<$state_type>,
            $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
        ) -> Json<$ret> {
            Json($handler_name(state, params).await)
        }

        pub async fn post_handler(
            state: State<$state_type>,
            $crate::controllers::request::PostContent(params): $crate::controllers::request::PostContent<$params_type>
        ) -> Json<$ret> {
            Json($handler_name(state, params).await)
        }
    };

    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty,
            $version_param:ident : LineageVersion
        ) -> impl IntoResponse $body:block
    ) => {
        async fn $handler_name(
            $state_pattern: State<$state_type>,
            $params_pattern: $params_type,
            $version_param: LineageVersion
        ) -> impl axum::response::IntoResponse $body

        $(
            paste::paste! {
                pub async fn [<get_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
                ) -> impl axum::response::IntoResponse {
                    $handler_name(state, params, $version).await
                }

                pub async fn [<post_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::PostContent(params): $crate::controllers::request::PostContent<$params_type>
                ) -> impl axum::response::IntoResponse {
                    $handler_name(state, params, $version).await
                }
            }
        )*
    };
}

pub(crate) use generate_handlers;
