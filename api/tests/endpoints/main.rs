//! Endpoint tests, laid out as `src/controllers/` is — a handler's tests sit at the matching path.
//!
//! Each module covers its own endpoint across the parameter combinations that change the answer;
//! `routing` holds what is true of every route. Tests need `flavor = "multi_thread"`, since the
//! handlers call `block_in_place`.

#[path = "../common.rs"]
mod common;

mod api;
mod database;
mod datasets;
mod health;
mod mpa;
mod private_api;
mod routing;
mod search;

// ── the suite's own scaffolding ──────────────────────────────────────────────────────────────────

/// The corpus files must outlive the request: a memory-mapped index reads them until it is
/// dropped. Fails if a helper stops holding its `TempDir`.
#[tokio::test(flavor = "multi_thread")]
async fn corpus_files_outlive_the_request() {
    use axum::{
        body::Body,
        http::{Request, StatusCode}
    };

    use crate::common::{offline_state, request_json};

    let (dir, state) = offline_state();
    let path = dir.path().to_path_buf();

    let (status, _) =
        request_json(state, Request::get("/api/v2/taxa2lca?input[]=8501").body(Body::empty()).unwrap()).await;

    assert_eq!(status, StatusCode::OK);
    assert!(path.join("sa.bin").exists(), "the index files were deleted before the request finished");
    assert!(path.join("taxons.tsv").exists(), "the datastore files were deleted before the request finished");

    drop(dir);
    assert!(!path.exists(), "and they are cleaned up once the guard is dropped");
}

/// Every test file under `tests/` must be reachable, or it silently never runs.
///
/// Cargo turns `tests/foo.rs` and `tests/foo/main.rs` into targets by itself, but anything deeper
/// only compiles if some `mod` statement names it — and a file nobody names sits there looking
/// like coverage while contributing nothing. Not hypothetical: four of these modules were orphaned
/// while being moved into this layout, and the suite passed at 103 tests instead of 117 without
/// anything looking wrong.
#[test]
fn every_test_file_is_reachable() {
    use std::{
        fs,
        path::{Path, PathBuf}
    };

    fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("tests/ should be readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut files = Vec::new();
    rust_files(&tests, &mut files);

    let sources: Vec<String> = files.iter().map(|file| fs::read_to_string(file).expect("a test file")).collect();

    for file in &files {
        let relative = file.strip_prefix(&tests).expect("under tests/").to_string_lossy().replace('\\', "/");

        // What cargo discovers on its own: a file directly in tests/, or a main.rs one level down.
        let depth = relative.matches('/').count();
        if depth == 0 || (depth == 1 && file.file_name().is_some_and(|name| name == "main.rs")) {
            continue;
        }

        // Everything deeper has to be named. A `foo/mod.rs` is named by its directory.
        let module = if file.file_name().is_some_and(|name| name == "mod.rs") {
            file.parent().and_then(Path::file_name)
        } else {
            file.file_stem()
        }
        .expect("a module name")
        .to_string_lossy()
        .into_owned();

        let declaration = format!("mod {module};");
        assert!(
            sources.iter().any(|source| source.contains(&declaration)),
            "tests/{relative} is a module nobody declares, so it never runs. Add `{declaration}` to its parent."
        );
    }
}
