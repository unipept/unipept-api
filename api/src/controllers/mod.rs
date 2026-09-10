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

        pastey::paste! {
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

    // Generates the GET and POST handlers when both methods take the same parameters
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

        pastey::paste! {
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
}

pub(crate) use generate_handlers;
