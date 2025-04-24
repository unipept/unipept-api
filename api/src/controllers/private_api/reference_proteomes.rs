use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{
    controllers::generate_handlers,
    AppState
};

#[derive(Serialize, Deserialize)]
pub struct Parameters {
    #[serde(default)]
    proteomes: Vec<String>
}

#[derive(Serialize)]
pub struct ReferenceProteome {
    id: String,
    taxon_id: u32,
    taxon_name: String,
    protein_count: u32
}

async fn handler(
    State(AppState { datastore, .. }): State<AppState>,
    Parameters { proteomes }: Parameters
) -> Result<Vec<ReferenceProteome>, ()> {
    Ok(
        proteomes
            .iter()
            .map(|proteome| proteome.trim())
            .filter_map(|proteome| {
                datastore.reference_proteome_store().get(proteome).map(|(taxon_id, protein_count, _)| {
                    let taxon_name = datastore
                        .taxon_store()
                        .get_name(*taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string()); // Use `unwrap_or_else` for a default

                    ReferenceProteome {
                        id: proteome.to_string(),
                        taxon_id: *taxon_id,
                        taxon_name,
                        protein_count: *protein_count,
                    }

                })
            })
            .collect()
    )
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<Vec<ReferenceProteome>>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
