use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{helpers::tree_helper::{frequency::FrequencyTable, node::Node}, AppState};

use super::Query;

#[derive(Serialize, Deserialize)]
pub struct Body {
    counts: HashMap<u32, usize>
}

#[derive(Deserialize)]
pub struct QueryParams {
    input: Vec<u32>
}

pub type TaxonTree = Node;

pub async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Query(QueryParams { input }): Query<QueryParams>,
    Json(Body { counts }): Json<Body>
) -> Json<TaxonTree> {
    let _taxon_store = datastore.taxon_store();
    let _lineage_store = datastore.lineage_store();

    // Taxon counts
    let frequencies = if counts.is_empty() {
        FrequencyTable::from_data(&input)
    } else {
        FrequencyTable::from_counts(counts)
    };
    
    eprintln!("{:?}", frequencies);

    Json(Node::new(1, "root".to_string(), "no rank".to_string()))
}
