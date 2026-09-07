//! The endpoints that read only the datastore, one module per controller.
//!
//! A controller's parameters are its contract, and most of them take several — so each module
//! covers its own endpoint across the combinations that change the answer, rather than sampling
//! one call per endpoint. A flag whose two values are never both exercised is a flag no test
//! distinguishes from a constant.
//!
//! Split by controller rather than by concern so that the tests for an endpoint sit where someone
//! changing that endpoint will find them. `routing` holds what is true of every route at once and
//! belongs to none of them.
//!
//! Every test runs on a multi-threaded runtime: nine controllers call `tokio::task::block_in_place`,
//! which panics on the current-thread runtime `#[tokio::test]` builds by default.

#[path = "../common/mod.rs"]
mod common;

mod ecnumbers;
mod goterms;
mod interpros;
mod metadata;
mod reference_proteomes;
mod reference_proteomes_filter;
mod routing;
mod sampledata;
mod taxa;
mod taxa2lca;
mod taxa2rank;
mod taxa2tree;
mod taxa_filter;
mod taxonomy;
