use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{
    controllers::generate_handlers,
    AppState
};

fn default_filter() -> String {
    String::from("")
}

#[derive(Deserialize)]
pub struct ReferenceProteomeCountParameters {
    #[serde(default = "default_filter")]
    filter: String
}

#[derive(Deserialize)]
pub struct ReferenceProteomeFilterParameters {
    #[serde(default = "default_filter")]
    filter: String,
    start: usize,
    end: usize,
    #[serde(default)]
    sort_by: String, // Can be "id", "name", or "rank"
    #[serde(default)]
    sort_descending: bool
}

#[derive(Serialize)]
pub struct ReferenceProteomeCountResult {
    count: u32
}

async fn count_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeCountParameters { filter }: ReferenceProteomeCountParameters
) -> Result<ReferenceProteomeCountResult, ()> {
    let proteome_store = datastore.reference_proteome_store();
    let taxon_store = datastore.taxon_store();
    
    if filter.is_empty() {
        Ok(ReferenceProteomeCountResult {
            count: proteome_store.mapper
                .values()
                .count() as u32
        })
    } else {
        Ok(ReferenceProteomeCountResult {
            count: proteome_store.mapper
                .iter()
                .filter(|(key, (taxon_id, _))| {
                    let taxon_name = datastore
                        .taxon_store()
                        .get_name(*taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string());

                    key.to_lowercase().contains(&filter.to_lowercase()) ||
                        taxon_id.to_string().contains(&filter) || 
                        taxon_name.to_lowercase().contains(&filter.to_lowercase())
                })
                .count() as u32
        })
    }
}

async fn filter_handler(
    State(AppState { datastore, .. }): State<AppState>,
    ReferenceProteomeFilterParameters {
        filter,
        start,
        end,
        sort_by,
        sort_descending
    }: ReferenceProteomeFilterParameters
) -> Result<Vec<String>, ()> {
    let proteome_store = datastore.reference_proteome_store();

    let mut filtered_proteomes: Vec<(&String, &(u32, u32))> = proteome_store.mapper
        .iter()
        .filter(|(key, (taxon_id, _))| {
            let taxon_name = datastore
                .taxon_store()
                .get_name(*taxon_id).cloned() // Clone the &String to String
                .unwrap_or_else(|| "Unknown".to_string());

            key.to_lowercase().contains(&filter.to_lowercase()) ||
                taxon_id.to_string().contains(&filter) ||
                taxon_name.to_lowercase().contains(&filter.to_lowercase())
        })
        .collect();

    // Sort based on the `sort_by` field
    match sort_by.as_str() {
        "taxon_name" => {
            if sort_descending {
                filtered_proteomes.sort_by(|(_, &(a_taxon_id, _)), (_, &(b_taxon_id,_))| {
                    let taxon_name_a = datastore
                        .taxon_store()
                        .get_name(a_taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string());

                    let taxon_name_b = datastore
                        .taxon_store()
                        .get_name(b_taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string());
                    taxon_name_b.cmp(&taxon_name_a)
                });
            } else {
                filtered_proteomes.sort_by(|(_, &(a_taxon_id, _)), (_, &(b_taxon_id,_))| {
                    let taxon_name_a = datastore
                        .taxon_store()
                        .get_name(a_taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string());

                    let taxon_name_b = datastore
                        .taxon_store()
                        .get_name(b_taxon_id).cloned() // Clone the &String to String
                        .unwrap_or_else(|| "Unknown".to_string());
                    taxon_name_a.cmp(&taxon_name_b)
                });
            }
        },
        "protein_count" => {
            if sort_descending {
                filtered_proteomes.sort_by(|(_, (_, a_protein_count)), (_, &(_, b_protein_count))| {
                    b_protein_count.cmp(a_protein_count)
                });
            } else {
                filtered_proteomes.sort_by(|(_, &(_, a_protein_count)), (_, (_, b_protein_count))| {
                    a_protein_count.cmp(b_protein_count)
                });
            }
        },
        _ => {
            if sort_descending {
                filtered_proteomes.sort_by(|(a_proteome_id, _), (b_proteome_id, _)| {
                    b_proteome_id.cmp(a_proteome_id)
                });
            } else {
                filtered_proteomes.sort_by(|(a_proteome_id, _), (b_proteome_id, _)| {
                    a_proteome_id.cmp(b_proteome_id)
                });
            }
        },
    }

    Ok(filtered_proteomes.into_iter().skip(start).take(end - start).map(|(key, _)| key.to_string()).collect())
}

generate_handlers!(
    async fn json_count_handler(
        state => State<AppState>,
        params => ReferenceProteomeCountParameters
    ) -> Result<Json<ReferenceProteomeCountResult>, ()> {
        Ok(Json(count_handler(state, params).await?))
    }
);

generate_handlers!(
    async fn json_filter_handler(
        state => State<AppState>,
        params => ReferenceProteomeFilterParameters
    ) -> Result<Json<Vec<String>>, ()> {
        Ok(Json(filter_handler(state, params).await?))
    }
);
