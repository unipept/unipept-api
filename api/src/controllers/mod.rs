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

    // Generates the GET handler alone, for a route whose two methods take different parameters
    (
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            GetContent($params_pattern:pat) => GetContent<$params_type:ty>
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        pastey::paste! {
            pub async fn [<get_ $handler_name>](
                $state_pattern: State<$state_type>,
                GetContent($params_pattern): $crate::controllers::request::GetContent<$params_type>
            ) -> Result<$ret, $err> $body
        }
    };

    // Generates the POST handler alone, for the same reason
    (
        async fn $handler_name:ident(
            $state_pattern:pat => State<$state_type:ty>,
            PostContent($params_pattern:pat) => PostContent<$params_type:ty>
        ) -> Result<$ret:ty, $err:ty> $body:block
    ) => {
        pastey::paste! {
            pub async fn [<post_ $handler_name>](
                $state_pattern: State<$state_type>,
                PostContent($params_pattern): $crate::controllers::request::PostContent<$params_type>
            ) -> Result<$ret, $err> $body
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
