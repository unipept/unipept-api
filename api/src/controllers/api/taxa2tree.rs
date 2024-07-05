use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::State,
    Json
};
use serde::{
    Deserialize,
    Serialize
};

use crate::{
    controllers::{
        api::default_link,
        request::{
            GetContent,
            PostContent
        },
        response::HtmlTemplate
    }, errors::ApiError, helpers::{
        lineage_helper::LineageVersion,
        tree_helper::{
            build_tree,
            frequency::FrequencyTable,
            node::Node
        }
    }, AppState
};

#[derive(Deserialize)]
pub struct GetParameters {
    input: Vec<u32>,
    #[serde(default = "default_link")]
    link:  bool
}

#[derive(Deserialize)]
pub struct PostParameters {
    counts: HashMap<u32, usize>,
    #[serde(default = "default_link")]
    link:   bool
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

pub async fn get_handler_v1(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<Json<TreeInformation>, ()> {
    Ok(Json(create_tree_information(state, Parameters::Get(params), LineageVersion::V1)))
}

pub async fn post_handler_v1(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<Json<TreeInformation>, ()> {
    Ok(Json(create_tree_information(state, Parameters::Post(params), LineageVersion::V1)))
}

pub async fn get_handler_v2(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<Json<TreeInformation>, ()> {
    Ok(Json(create_tree_information(state, Parameters::Get(params), LineageVersion::V2)))
}

pub async fn post_handler_v2(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<Json<TreeInformation>, ()> {
    Ok(Json(create_tree_information(state, Parameters::Post(params), LineageVersion::V2)))
}

fn html_handler(
    state: State<AppState>,
    params: Parameters,
    version: LineageVersion
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    match create_tree_information(state, params, version) {
        TreeInformation::Tree { root } => Ok(HtmlTemplate(TreeTemplate {
            json_data: serde_json::to_string(&root)?
        })),
        TreeInformation::Link { .. } => Err(ApiError::NotImplementedError(
            "HTML output is not implemented when using the link option".to_string()
        ))
    }
}

pub async fn get_html_handler_v1(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    html_handler(state, Parameters::Get(params), LineageVersion::V1)
}

pub async fn post_html_handler_v1(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    html_handler(state, Parameters::Post(params), LineageVersion::V1)
}

pub async fn get_html_handler_v2(
    state: State<AppState>,
    GetContent(params): GetContent<GetParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    html_handler(state, Parameters::Get(params), LineageVersion::V2)
}

pub async fn post_html_handler_v2(
    state: State<AppState>,
    PostContent(params): PostContent<PostParameters>
) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
    html_handler(state, Parameters::Post(params), LineageVersion::V2)
}

fn create_tree_information(
    State(AppState {
        datastore, ..
    }): State<AppState>,
    params: Parameters,
    version: LineageVersion
) -> TreeInformation {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let (frequencies, link) = match params {
        Parameters::Get(GetParameters {
            input,
            link
        }) => (FrequencyTable::from_data(&input), link),
        Parameters::Post(PostParameters {
            counts,
            link
        }) => (FrequencyTable::from_counts(counts), link)
    };

    let root = build_tree(frequencies, version, lineage_store, taxon_store);

    if link {
        return TreeInformation::Link {
            gist: "test".to_string()
        };
    }

    TreeInformation::Tree {
        root
    }
}
