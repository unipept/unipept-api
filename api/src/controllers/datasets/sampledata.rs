use axum::{extract::State, Json};
use datastore::sampledata::SampleData;

pub async fn handler(State(sample_data): State<SampleData>) -> Json<SampleData> {
    Json(sample_data)
}
