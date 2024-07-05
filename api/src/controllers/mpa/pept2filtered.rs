use std::collections::HashSet;

use axum::{extract::State, Json};
use sa_mappings::functionality::FunctionalAggregation;
use serde::{Deserialize, Serialize};

use crate::{controllers::{generate_json_handlers, mpa::default_equate_il}, AppState};

#[derive(Deserialize)]
pub struct Parameters {
    peptides: Vec<String>,
    taxa: Vec<usize>,
    #[serde(default = "default_equate_il")]
    equate_il: bool
}

#[derive(Serialize)]
pub struct FilteredDataItem {
    sequence: String,
    taxa: Vec<usize>,
    fa: Option<FunctionalAggregation>
}

#[derive(Serialize)]
pub struct FilteredData {
    peptides: Vec<FilteredDataItem>
}

generate_json_handlers!(
    async fn handler(
        State(AppState { index, .. }): State<AppState>,
        Parameters { mut peptides, taxa, equate_il } => Parameters
    ) -> FilteredData {
        if peptides.is_empty() {
            return FilteredData { peptides: Vec::new() };
        }
    
        peptides.sort();
        peptides.dedup();
        let result = index.analyse(&peptides, equate_il).result;
    
        let taxa_set: HashSet<usize> = HashSet::from_iter(taxa.iter().cloned());
    
        FilteredData {
            peptides: result.into_iter().filter_map(|mut item| {
                item.taxa = HashSet::from_iter(item.taxa.iter().cloned()).intersection(&taxa_set).cloned().collect();
                
                if item.taxa.is_empty() {
                    return None;
                }
                
                Some(FilteredDataItem { 
                    sequence: item.sequence, 
                    taxa: item.taxa,
                    fa: item.fa
                })
            }).collect()
        }
    }
);
