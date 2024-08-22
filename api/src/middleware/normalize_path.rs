use http::{Request, Response, Uri};
use std::{
    task::{Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;

#[allow(unused_macros)]
macro_rules! define_inner_service_accessors {
    () => {
        /// Gets a reference to the underlying service.
        pub fn get_ref(&self) -> &S {
            &self.inner
        }

        /// Gets a mutable reference to the underlying service.
        pub fn get_mut(&mut self) -> &mut S {
            &mut self.inner
        }

        /// Consumes `self`, returning the underlying service.
        pub fn into_inner(self) -> S {
            self.inner
        }
    };
}

/// Layer that applies [`NormalizePath`] which normalizes paths.
///
/// See the [module docs](self) for more details.
#[derive(Debug, Copy, Clone)]
pub struct NormalizePathLayer {}

impl NormalizePathLayer {
    /// Create a new [`NormalizePathLayer`].
    ///
    /// Any trailing slashes from request paths will be removed. For example, a request with `/foo/`
    /// will be changed to `/foo` before reaching the inner service.
    pub fn normalize_uris() -> Self {
        NormalizePathLayer {}
    }
}

impl<S> Layer<S> for NormalizePathLayer {
    type Service = NormalizePath<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NormalizePath::normalize_uris(inner)
    }
}

/// Middleware that normalizes paths.
///
/// See the [module docs](self) for more details.
#[derive(Debug, Copy, Clone)]
pub struct NormalizePath<S> {
    inner: S,
}

impl<S> NormalizePath<S> {
    /// Create a new [`NormalizePath`].
    ///
    /// Any trailing slashes from request paths will be removed. For example, a request with `/foo/`
    /// will be changed to `/foo` before reaching the inner service.
    pub fn normalize_uris(inner: S) -> Self {
        Self { inner }
    }

    define_inner_service_accessors!();
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for NormalizePath<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        normalize_uris(req.uri_mut());
        self.inner.call(req)
    }
}

fn normalize_uris(uri: &mut Uri) {
    println!("{}", uri);

    // Normalize the path by removing consecutive slashes
    let normalized_path = uri
        .path()
        .split('/')
        .filter(|&segment| !segment.is_empty())
        .collect::<Vec<&str>>()
        .join("/");

    // Reconstruct the URI with the normalized path
    let new_uri = if normalized_path.is_empty() {
        Uri::builder()
            .path_and_query("/")
            .build()
            .unwrap()
    } else if let Some(path) = uri.query() {
        Uri::builder()
            .path_and_query(format!("/{}?{}", normalized_path, path))
            .build()
            .unwrap()

    } else {
        Uri::builder()
            .path_and_query(format!("/{}", normalized_path))
            .build()
            .unwrap()
    };

    *uri = new_uri;
}
