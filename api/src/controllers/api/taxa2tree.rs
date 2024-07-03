use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{controllers::{api::default_link, PostContent, Query}, helpers::{lineage_helper::LineageVersion, tree_helper::{build_tree, frequency::FrequencyTable, node::Node}}, AppState};

#[derive(Deserialize)]
pub struct PostParameters {
    counts: HashMap<u32, usize>,
    #[serde(default = "default_link")]
    link: bool
}

#[derive(Deserialize)]
pub struct GetParameters {
    input: Vec<u32>,
    #[serde(default = "default_link")]
    link: bool
}

#[derive(Deserialize)]
pub enum Parameters {
    Get(GetParameters),
    Post(PostParameters)
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum TreeInformation {
    Tree {
        #[serde(flatten)]
        root: Node
    },
    Link {
        gist: String
    }
}

pub async fn get_handler_v1(
    state: State<AppState>,
    Query(params): Query<GetParameters>
) -> Json<TreeInformation> {
    handler(state, Parameters::Get(params), LineageVersion::V1)
}

pub async fn post_handler_v1(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Json<TreeInformation> {
    handler(state, Parameters::Post(params), LineageVersion::V1)
}

pub async fn get_handler_v2(
    state: State<AppState>,
    Query(params): Query<GetParameters>
) -> Json<TreeInformation> {
    handler(state, Parameters::Get(params), LineageVersion::V2)
}

pub async fn post_handler_v2(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Json<TreeInformation> {
    handler(state, Parameters::Post(params), LineageVersion::V2)
}

fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    params: Parameters,
    version: LineageVersion
) -> Json<TreeInformation> {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let (frequencies, link) = match params {
        Parameters::Get(GetParameters { input, link }) => {
            (FrequencyTable::from_data(&input), link)
        },
        Parameters::Post(PostParameters { counts, link }) => {
            (FrequencyTable::from_counts(counts), link)
        }
    };

    let root = build_tree(frequencies, version, lineage_store, taxon_store);

    if link {
        return Json(TreeInformation::Link { gist: "test".to_string() });
    }

    Json(TreeInformation::Tree { root })
}
