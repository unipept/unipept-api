use std::{collections::HashMap, convert::Infallible};

use askama::Template;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    controllers::{
        api::default_link,
        request::{Flag, GetContent, PostContent},
        response::HtmlTemplate
    },
    errors::ApiError,
    helpers::tree_helper::{build_tree, frequency::FrequencyTable, node::Node}
};

#[derive(Deserialize)]
pub struct GetParameters {
    #[serde(default)]
    input: Vec<u32>,
    #[serde(default = "default_link")]
    link: Flag
}

#[derive(Deserialize)]
pub struct PostParameters {
    #[serde(default)]
    counts: HashMap<u32, usize>,
    #[serde(default = "default_link")]
    link: Flag
}

#[derive(Deserialize)]
#[serde(untagged)]
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

#[derive(Template)]
#[template(path = "taxa2tree.html", escape = "none")]
pub struct TreeTemplate {
    json_data: String
}

fn handler(State(AppState { datastore, .. }): State<AppState>, params: Parameters) -> TreeInformation {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let (frequencies, link) = match params {
        Parameters::Get(GetParameters { input, link: Flag(link) }) => (FrequencyTable::from_data(&input), link),
        Parameters::Post(PostParameters { counts, link: Flag(link) }) => (FrequencyTable::from_counts(counts), link)
    };

    let root = build_tree(frequencies, lineage_store, taxon_store);

    if link {
        return TreeInformation::Link { gist: "test".to_string() };
    }

    TreeInformation::Tree { root }
}

pub async fn get_json_handler(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<Json<TreeInformation>, Infallible> {
    Ok(Json(handler(state, Parameters::Get(params))))
}

pub async fn post_json_handler(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<Json<TreeInformation>, Infallible> {
    Ok(Json(handler(state, Parameters::Post(params))))
}

/// The tree as a page, or a refusal: the page draws a tree, and `link` answers a reference instead.
fn render(information: TreeInformation) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    match information {
        TreeInformation::Tree { root } => Ok(HtmlTemplate(TreeTemplate { json_data: serde_json::to_string(&root)? })),
        TreeInformation::Link { .. } => {
            Err(ApiError::NotImplementedError("HTML output is not supported when using the link option".to_string()))
        }
    }
}

pub async fn get_html_handler(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    render(handler(state, Parameters::Get(params)))
}

pub async fn post_html_handler(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    render(handler(state, Parameters::Post(params)))
}
