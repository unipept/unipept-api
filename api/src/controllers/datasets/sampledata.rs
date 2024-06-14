use axum::{extract::State, Json};
use datastore::sampledata::SampleData;

use crate::SampleState;

pub async fn handler(State(SampleState { samples }): State<SampleState>) -> Json<SampleData> {
    Json(samples.as_ref().to_owned())
}
