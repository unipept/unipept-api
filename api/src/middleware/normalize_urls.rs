use axum::{
    body::Body,
    http::{Request, Uri},
    response::Response,
    Router,
};
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceBuilder};
use hyper::Request as HyperRequest;

// Create a layer that normalizes URIs
pub fn create_normalize_uri_layer<S>() -> impl Layer<S> + Clone {
    tower::layer::layer_fn(|inner: S| {
        tower::service_fn(move |req: HyperRequest<Body>| {
            let (mut parts, body) = req.into_parts();
            let path = parts.uri.path().replace("//", "/");
            let new_uri = Uri::builder()
                .scheme(parts.uri.scheme().cloned().unwrap_or_else(|| http::uri::Scheme::HTTP))
                .authority(parts.uri.authority().cloned())
                .path_and_query(path)
                .build()
                .unwrap();

            parts.uri = new_uri;
            let req = HyperRequest::from_parts(parts, body);

            inner.call(req)
        })
    })
}