use std::collections::HashSet;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use index::{ProteinInfo, SearchResult};
use crate::{
    controllers::{generate_handlers, mpa::default_equate_il, mpa::default_tryptic},
    helpers::fa_helper::{calculate_fa, FunctionalAggregation},
    AppState
};
use crate::helpers::filters::empty_filter::EmptyFilter;
use crate::helpers::lca_helper::calculate_lca;
use crate::helpers::filters::protein_filter::ProteinFilter;
use crate::helpers::filters::proteome_filter::ProteomeFilter;
use crate::helpers::filters::taxa_filter::TaxaFilter;
use crate::helpers::filters::UniprotFilter;
use crate::helpers::lineage_helper::{get_lineage_array, LineageVersion};
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    peptides: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_tryptic")]
    tryptic: bool,
    filter: Option<Filter>,
}

#[derive(Deserialize)]
pub enum Filter {
    #[serde(rename = "taxa")]
    Taxa(HashSet<u32>),
    #[serde(rename = "proteomes")]
    Proteomes(HashSet<String>),
    #[serde(rename = "proteins")]
    Proteins(HashSet<String>)
}

#[derive(Serialize)]
pub struct FilteredDataItem {
    sequence: String,
    lca: Option<u32>,
    lineage: Vec<Option<i32>>,
    fa: FunctionalAggregation
}

#[derive(Serialize)]
pub struct FilteredData {
    peptides: Vec<FilteredDataItem>
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { mut peptides, equate_il, tryptic, filter }: Parameters
) -> Result<FilteredData, ()> {
    if peptides.is_empty() {
        return Ok(FilteredData { peptides: Vec::new() });
    }

    peptides.sort();
    peptides.dedup();

    let peptides = sanitize_peptides(peptides);
    let result = index.analyse(&peptides, equate_il, tryptic, Some(10_000));

    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    let filter_proteins: Box<dyn UniprotFilter> = match filter {
        Some(Filter::Taxa(taxa)) => {
            if taxa.contains(&1) {
                Box::new(EmptyFilter::new())
            } else {
                Box::new(TaxaFilter::new(taxa, lineage_store))
            }
        },
        Some(Filter::Proteomes(proteomes)) => {
            Box::new(ProteomeFilter::new(proteomes).await.unwrap())
        },
        Some(Filter::Proteins(proteins)) => {
            Box::new(ProteinFilter::new(proteins))
        },
        None => Box::new(EmptyFilter::new())
    };

    Ok(FilteredData {
        peptides: result
            .into_iter()
            .filter_map(|SearchResult { proteins, sequence, .. }| {
                let filtered_proteins: Vec<ProteinInfo> = proteins
                    .into_iter()
                    .filter(|protein| filter_proteins.filter(protein))
                    .collect();

                if filtered_proteins.is_empty() {
                    return None;
                }

                let lca = calculate_lca(
                    filtered_proteins.iter().map(|protein| protein.taxon).collect(),
                    LineageVersion::V2,
                    taxon_store,
                    lineage_store
                );
                let lineage = get_lineage_array(lca as u32, LineageVersion::V2, lineage_store);

                Some(FilteredDataItem {
                    sequence,
                    lca: Some(lca as u32),
                    lineage,
                    fa: calculate_fa(&filtered_proteins)
                })
            })
            .collect()
    })
}

generate_handlers!(
    async fn json_handler(
        state=> State<AppState>,
        params => Parameters
    ) -> Result<Json<FilteredData>, ()> {
        Ok(Json(handler(state, params).await?))
    }
);
