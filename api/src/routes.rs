use axum::{
    extract::{DefaultBodyLimit},
    routing::get,
    Router
};
use tower::{ServiceBuilder};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{
    controllers::{
        api::{
            pept2ec, pept2funct, pept2go, pept2interpro, pept2lca, pept2prot, pept2taxa, peptinfo, protinfo, taxa2lca,
            taxa2tree, taxonomy
        },
        datasets::sampledata,
        mpa::{pept2data, pept2filtered},
        private_api::{ecnumbers, goterms, interpros, metadata, proteins, taxa}
    },
    middleware::{
        cors::create_cors_layer,
        tracing::{create_tracing_layer, init_tracing_subscriber},
    },
    AppState
};

pub fn create_router(state: AppState) -> Router {
    init_tracing_subscriber();

    Router::new()
        .route("/", get(|| async { "Unipept API server" }))
        .nest("/api", create_api_routes())
        .nest("/datasets", create_datasets_routes())
        .nest("/mpa", create_mpa_routes())
        .nest("/private_api", create_private_api_routes())
        .layer(
            ServiceBuilder::new()
                // Set max request size to 50MiB (default is 2MiB)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
                .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024))
                .layer(create_tracing_layer())
                .layer(create_cors_layer())
        )
        .with_state(state)
}

fn create_api_routes() -> Router<AppState> {
    Router::new().nest("/v1", create_api_v1_routes()).nest("/v2", create_api_v2_routes())
}

fn create_api_v1_routes() -> Router<AppState> {
    define_routes!(
        "/pept2ec",
        get(pept2ec::get_json_handler).post(pept2ec::post_json_handler),
        "/pept2funct",
        get(pept2funct::get_json_handler).post(pept2funct::post_json_handler),
        "/pept2go",
        get(pept2go::get_json_handler).post(pept2go::post_json_handler),
        "/pept2interpro",
        get(pept2interpro::get_json_handler).post(pept2interpro::post_json_handler),
        "/pept2lca",
        get(pept2lca::get_json_handler_v1).post(pept2lca::post_json_handler_v1),
        "/pept2prot",
        get(pept2prot::get_json_handler).post(pept2prot::post_json_handler),
        "/pept2taxa",
        get(pept2taxa::get_json_handler_v1).post(pept2taxa::post_json_handler_v1),
        "/peptinfo",
        get(peptinfo::get_json_handler_v1).post(peptinfo::post_json_handler_v1),
        "/protinfo",
        get(protinfo::get_json_handler_v1).post(protinfo::post_json_handler_v1),
        "/taxa2lca",
        get(taxa2lca::get_json_handler_v1).post(taxa2lca::post_json_handler_v1),
        "/taxa2tree",
        get(taxa2tree::get_json_handler_v1).post(taxa2tree::post_json_handler_v1),
        "/taxonomy",
        get(taxonomy::get_json_handler_v1).post(taxonomy::post_json_handler_v1)
    )
    .route("/taxa2tree.html", get(taxa2tree::get_html_handler_v1).post(taxa2tree::post_html_handler_v1))
}

fn create_api_v2_routes() -> Router<AppState> {
    define_routes!(
        "/pept2ec",
        get(pept2ec::get_json_handler).post(pept2ec::post_json_handler),
        "/pept2funct",
        get(pept2funct::get_json_handler).post(pept2funct::post_json_handler),
        "/pept2go",
        get(pept2go::get_json_handler).post(pept2go::post_json_handler),
        "/pept2interpro",
        get(pept2interpro::get_json_handler).post(pept2interpro::post_json_handler),
        "/pept2lca",
        get(pept2lca::get_json_handler_v2).post(pept2lca::post_json_handler_v2),
        "/pept2prot",
        get(pept2prot::get_json_handler).post(pept2prot::post_json_handler),
        "/pept2taxa",
        get(pept2taxa::get_json_handler_v2).post(pept2taxa::post_json_handler_v2),
        "/peptinfo",
        get(peptinfo::get_json_handler_v2).post(peptinfo::post_json_handler_v2),
        "/protinfo",
        get(protinfo::get_json_handler_v2).post(protinfo::post_json_handler_v2),
        "/taxa2lca",
        get(taxa2lca::get_json_handler_v2).post(taxa2lca::post_json_handler_v2),
        "/taxa2tree",
        get(taxa2tree::get_json_handler_v2).post(taxa2tree::post_json_handler_v2),
        "/taxonomy",
        get(taxonomy::get_json_handler_v2).post(taxonomy::post_json_handler_v2)
    )
    .route("/taxa2tree.html", get(taxa2tree::get_html_handler_v2).post(taxa2tree::post_html_handler_v2))
}

fn create_datasets_routes() -> Router<AppState> {
    define_routes!("/sampledata", get(sampledata::get_json_handler).post(sampledata::post_json_handler))
}

fn create_mpa_routes() -> Router<AppState> {
    define_routes!(
        "/pept2data",
        get(pept2data::get_json_handler).post(pept2data::post_json_handler),
        "/pept2filtered",
        get(pept2filtered::get_json_handler).post(pept2filtered::post_json_handler)
    )
}

fn create_private_api_routes() -> Router<AppState> {
    define_routes!(
        "/ecnumbers",
        get(ecnumbers::get_json_handler).post(ecnumbers::post_json_handler),
        "/goterms",
        get(goterms::get_json_handler).post(goterms::post_json_handler),
        "/interpros",
        get(interpros::get_json_handler).post(interpros::post_json_handler),
        "/metadata",
        get(metadata::get_json_handler).post(metadata::post_json_handler),
        "/proteins",
        get(proteins::get_json_handler).post(proteins::post_json_handler),
        "/taxa",
        get(taxa::get_json_handler).post(taxa::post_json_handler)
    )
}

macro_rules! define_routes {
    (
        $( $path:tt, $handlers:expr),*
    ) => {{
        let mut router = Router::new();

        $(
            router = router
                .route($path, $handlers)
                .route(concat!($path, ".json"), $handlers);
        )*

        router
    }};
}

pub(crate) use define_routes;
