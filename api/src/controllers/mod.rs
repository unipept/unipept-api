pub mod api;
pub mod datasets;
pub mod mpa;
pub mod private_api;
pub mod request;
pub mod response;

macro_rules! generate_json_handlers {
    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>
        ) -> Result<$ret, $err> $body

        pub async fn get_json_handler(
            state: State<$state_type>
        ) -> Result<Json<$ret>, $err> {
            Ok(Json($handler_name(state).await?))
        }

        pub async fn post_json_handler(
            state: State<$state_type>
        ) -> Result<Json<$ret>, $err> {
            Ok(Json($handler_name(state).await?))
        }
    };

    (
        async fn $handler_name:ident(
            State($state_pattern:pat): State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        async fn $handler_name(
            State($state_pattern): State<$state_type>,
            $params_pattern: $params_type
        ) -> Result<$ret, $err> $body

        pub async fn get_json_handler(
            state: State<$state_type>,
            $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
        ) -> Result<Json<$ret>, $err> {
            Ok(Json($handler_name(state, params).await?))
        }

        pub async fn post_json_handler(
            state: State<$state_type>,
            $crate::controllers::request::PostContent(params): $crate::controllers::request::PostContent<$params_type>
        ) -> Result<Json<$ret>, $err> {
            Ok(Json($handler_name(state, params).await?))
        }
    };

    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty,
            $version_param:ident : LineageVersion
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        async fn $handler_name(
            $state_pattern: State<$state_type>,
            $params_pattern: $params_type,
            $version_param: LineageVersion
        ) -> Result<$ret, $err> $body

        $(
            paste::paste! {
                pub async fn [<get_json_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
                ) -> Result<Json<$ret>, $err> {
                    Ok(Json($handler_name(state, params, $version).await?))
                }

                pub async fn [<post_json_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::PostContent(params): $crate::controllers::request::PostContent<$params_type>
                ) -> Result<Json<$ret>, $err> {
                    Ok(Json($handler_name(state, params, $version).await?))
                }
            }
        )*
    };
}

pub(crate) use generate_json_handlers;
