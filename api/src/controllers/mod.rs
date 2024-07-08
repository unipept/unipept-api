pub mod api;
pub mod datasets;
pub mod mpa;
pub mod private_api;
pub mod request;
pub mod response;

macro_rules! generate_handlers {
    // Generates the GET and POST handlers when there are no parameters
    (
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        async fn $handler_name(
            $state_pattern: State<$state_type>
        ) -> Result<$ret, $err> $body

        paste::paste! {
            pub async fn [<get_ $handler_name>](
                state: State<$state_type>
            ) -> Result<$ret, $err> {
                $handler_name(state).await
            }

            pub async fn [<post_ $handler_name>](
                state: State<$state_type>
            ) -> Result<$ret, $err> {
                $handler_name(state).await
            }
        }
    };

    // Generates the GET and POST handlers when there are no versions
    (
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        async fn $handler_name(
            $state_pattern: State<$state_type>,
            $params_pattern: $params_type
        ) -> Result<$ret, $err> $body

        paste::paste! {
            pub async fn [<get_ $handler_name>](
                state: State<$state_type>,
                $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
            ) -> Result<$ret, $err> {
                $handler_name(state, params).await
            }

            pub async fn [<post_ $handler_name>](
                state: State<$state_type>,
                $crate::controllers::request::PostContent(params): $crate::controllers::request::PostContent<$params_type>
            ) -> Result<$ret, $err> {
                $handler_name(state, params).await
            }
        }
    };

    // Generates the GET handlers when there are multiple versions
    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            GetContent($params_pattern:pat) => GetContent<$params_type:ty>,
            $version_param:ident : LineageVersion
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        paste::paste! {
            async fn [<get_ $handler_name>](
                $state_pattern: State<$state_type>,
                GetContent($params_pattern): $crate::controllers::request::GetContent<$params_type>,
                $version_param: LineageVersion
            ) -> Result<$ret, $err> $body

            $(
                pub async fn [<get_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    params: $crate::controllers::request::GetContent<$params_type>
                ) -> Result<$ret, $err> {
                    [<get_ $handler_name>](state, params, $version).await
                }
            )*
        }
    };

    // This macro is used to generate the POST handlers when there are multiple versions
    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            PostContent($params_pattern:pat) => PostContent<$params_type:ty>,
            $version_param:ident : LineageVersion
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        paste::paste! {
            async fn [<post_ $handler_name>](
                $state_pattern: State<$state_type>,
                PostContent($params_pattern): crate::controllers::request::PostContent<$params_type>,
                $version_param: LineageVersion
            ) -> Result<$ret, $err> $body

            $(
                pub async fn [<post_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    params: crate::controllers::request::PostContent<$params_type>
                ) -> Result<$ret, $err> {
                    [<post_ $handler_name>](state, params, $version).await
                }
            )*
        }
    };

    // This macro is used to generate the GET and POST handlers when there are multiple versions
    (
        [ $($version:ident),* ]
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            $params_pattern:pat => $params_type:ty,
            $version_param:ident : LineageVersion
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        paste::paste! {
            async fn $handler_name(
                $state_pattern: State<$state_type>,
                $params_pattern: $params_type,
                $version_param: LineageVersion
            ) -> Result<$ret, $err> $body

            $(
                pub async fn [<get_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::GetContent(params): $crate::controllers::request::GetContent<$params_type>
                ) -> Result<$ret, $err> {
                    $handler_name(state, params, $version).await
                }

                pub async fn [<post_ $handler_name _ $version:lower>](
                    state: State<$state_type>,
                    $crate::controllers::request::PostContent(params): crate::controllers::request::PostContent<$params_type>
                ) -> Result<$ret, $err> {
                    $handler_name(state, params, $version).await
                }
            )*
        }
    };
}

pub(crate) use generate_handlers;
