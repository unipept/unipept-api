use std::collections::HashMap;

use askama::Template;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::default_link,
        generate_handlers,
        request::{GetContent, PostContent},
        response::HtmlTemplate
    },
    errors::ApiError,
    helpers::{
        lineage_helper::LineageVersion::{self, *},
        tree_helper::{build_tree, frequency::FrequencyTable, node::Node}
    },
    AppState
};

#[derive(Deserialize)]
pub struct GetParameters {
    #[serde(default)]
    input: Vec<u32>,
    #[serde(default = "default_link")]
    link: bool
}

#[derive(Deserialize)]
pub struct PostParameters {
    #[serde(default)]
    counts: HashMap<u32, usize>,
    #[serde(default = "default_link")]
    link: bool
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

fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    params: Parameters,
    version: LineageVersion
) -> TreeInformation {
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let (frequencies, link) = match params {
        Parameters::Get(GetParameters { input, link }) => (FrequencyTable::from_data(&input), link),
        Parameters::Post(PostParameters { counts, link }) => (FrequencyTable::from_counts(counts), link)
    };

    let root = build_tree(frequencies, version, lineage_store, taxon_store);

    if link {
        return TreeInformation::Link { gist: "test".to_string() };
    }

    TreeInformation::Tree { root }
}

generate_handlers!(
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        GetContent(params) => GetContent<GetParameters>,
        version: LineageVersion
    ) -> Result<Json<TreeInformation>, ()> {
        Ok(Json(handler(state, Parameters::Get(params), version)))
    }
);

generate_handlers!(
    [ V1, V2 ]
    async fn json_handler(
        state => State<AppState>,
        PostContent(params) => PostContent<PostParameters>,
        version: LineageVersion
    ) -> Result<Json<TreeInformation>, ()> {
        Ok(Json(handler(state, Parameters::Post(params), version)))
    }
);

generate_handlers!(
    [ V1, V2 ]
    async fn html_handler(
        state => State<AppState>,
        GetContent(params) => GetContent<GetParameters>,
        version: LineageVersion
    ) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
        match handler(state, Parameters::Get(params), version) {
            TreeInformation::Tree { root } => Ok(HtmlTemplate(TreeTemplate {
                json_data: serde_json::to_string(&root)?
            })),
            TreeInformation::Link { .. } => Err(ApiError::NotImplementedError(
                "HTML output is not supported when using the link option".to_string()
            ))
        }
    }
);

generate_handlers!(
    [ V1, V2 ]
    async fn html_handler(
        state => State<AppState>,
        PostContent(params) => PostContent<PostParameters>,
        version: LineageVersion
    ) -> Result<HtmlTemplate<TreeTemplate>, ApiError> {
        match handler(state, Parameters::Post(params), version) {
            TreeInformation::Tree { root } => Ok(HtmlTemplate(TreeTemplate {
                json_data: serde_json::to_string(&root)?
            })),
            TreeInformation::Link { .. } => Err(ApiError::NotImplementedError(
                "HTML output is not supported when using the link option".to_string()
            ))
        }
    }
);
