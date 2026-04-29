use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    controllers::{
        api::{default_taxa_aggregation_method, default_taxa_aggregation_threshold, default_domains, default_equate_il, default_extra, default_names, default_validate_taxa},
        generate_handlers
    },
    helpers::{
        aggregation::{parse_aggregation, TaxaAggregation},
        ec_helper::{ec_numbers_from_map, EcNumber},
        fa_helper::calculate_fa,
        go_helper::{go_terms_from_map, GoTerms},
        interpro_helper::{interpro_entries_from_map, InterproEntries},
        lineage_helper::{
            get_lineage, get_lineage_with_names, Lineage,
            LineageVersion::{self, *}
        }
    },
    AppState
};
use crate::errors::ApiError;
use crate::helpers::sanitize_peptides;

#[derive(Deserialize)]
pub struct Parameters {
    #[serde(default)]
    input: Vec<String>,
    #[serde(default = "default_equate_il")]
    equate_il: bool,
    #[serde(default = "default_extra")]
    extra: bool,
    #[serde(default = "default_domains")]
    domains: bool,
    #[serde(default = "default_names")]
    names: bool,
    #[serde(default = "default_validate_taxa")]
    validate_taxa: bool,
    #[serde(default = "default_taxa_aggregation_method")]
    taxa_aggregation_method: String,
    #[serde(default = "default_taxa_aggregation_threshold")]
    taxa_aggregation_threshold: Option<u32>
}

#[derive(Serialize)]
pub struct PeptInformation {
    peptide: String,
    total_protein_count: usize,
    ec: Vec<EcNumber>,
    go: GoTerms,
    ipr: InterproEntries,
    #[serde(flatten)]
    taxon: Taxon,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    lineage: Option<Lineage>
}

#[derive(Serialize)]
pub struct Taxon {
    taxon_id: u32,
    taxon_name: String,
    taxon_rank: String
}

async fn handler(
    State(AppState { index, datastore, .. }): State<AppState>,
    Parameters { input, equate_il, extra, domains, names, validate_taxa, taxa_aggregation_method, taxa_aggregation_threshold }: Parameters,
    version: LineageVersion
) -> Result<Vec<PeptInformation>, ApiError> {
    let aggregator = parse_aggregation(&taxa_aggregation_method, taxa_aggregation_threshold)?;

    let input = sanitize_peptides(input);
    let result = tokio::task::spawn_blocking(move || {
        index.analyse(&input, equate_il, false, None)
    }).await?;

    let ec_store = datastore.ec_store();
    let go_store = datastore.go_store();
    let interpro_store = datastore.interpro_store();
    let taxon_store = datastore.taxon_store();
    let lineage_store = datastore.lineage_store();

    Ok(result
        .into_iter()
        .filter_map(|item| {
            let fa = calculate_fa(&item.proteins);

            let total_protein_count = item.proteins.len();
            // let total_protein_count = *fa.counts.get("all").unwrap_or(&0);
            let ecs = ec_numbers_from_map(&fa.data, ec_store, extra);
            let gos = go_terms_from_map(&fa.data, go_store, extra, domains);
            let iprs = interpro_entries_from_map(&fa.data, interpro_store, extra, domains);

            let lca = aggregator.aggregate(
                item.proteins.iter().map(|protein| protein.taxon).collect(),
                version,
                taxon_store,
                lineage_store,
                validate_taxa
            );
            let (name, rank, _) = taxon_store.get(lca as u32)?;
            let lineage = match (extra, names) {
                (true, true) => get_lineage_with_names(lca as u32, version, lineage_store, taxon_store),
                (true, false) => get_lineage(lca as u32, version, lineage_store),
                (false, _) => None
            };

            Some(PeptInformation {
                peptide: item.sequence,
                total_protein_count,
                ec: ecs,
                go: gos,
                ipr: iprs,
                taxon: Taxon {
                    taxon_id: lca as u32,
                    taxon_name: name.to_string(),
                    taxon_rank: rank.clone().into()
                },
                lineage
            })
        })
        .collect())
}

generate_handlers! (
    [ V2 ]
    async fn json_handler(
        state => State<AppState>,
        params => Parameters,
        version: LineageVersion
    ) -> Result<Json<Vec<PeptInformation>>, ApiError> {
        Ok(Json(handler(state, params, version).await?))
    }
);
