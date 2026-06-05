use axum::{extract::State, Json};
use datastore::LineageRank;
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    controllers::generate_handlers,
    errors::ApiError,
    AppState,
};
use database::get_proteins_for_taxon;

fn default_cleavage_regex() -> String {
    String::from("[KR](?!P)")
}

fn default_min_length() -> usize {
    5
}

#[derive(Deserialize)]
pub struct Parameters {
    taxon_id: u32,
    #[serde(default = "default_cleavage_regex")]
    cleavage_regex: String,
    #[serde(default = "default_min_length")]
    min_length: usize,
}

#[derive(Serialize)]
pub struct UniquePeptidesResult {
    unique_peptides: Vec<String>,
    total_peptides: usize,
    total_unique_peptides: usize,
}

async fn handler(
    State(AppState { index, datastore, database, .. }): State<AppState>,
    Parameters { taxon_id, cleavage_regex, min_length }: Parameters,
) -> Result<UniquePeptidesResult, ApiError> {
    let re = Regex::new(&cleavage_regex)
        .map_err(|e| ApiError::UnknownRankError(format!("Invalid cleavage_regex: {}", e)))?;

    let taxon_info = datastore.taxon_store().get(taxon_id)
        .ok_or_else(|| ApiError::UnknownRankError(format!("Taxon {} not found", taxon_id)))?;

    let rank = &taxon_info.1;
    if *rank != LineageRank::Species && *rank != LineageRank::Strain {
        return Err(ApiError::UnknownRankError(format!(
            "Taxon {} is at rank '{}', but must be at species or strain level",
            taxon_id, rank
        )));
    }

    let proteins = get_proteins_for_taxon(database.get_conn(), taxon_id).await?;

    let mut peptides: Vec<String> = proteins.iter().flat_map(|protein| {
        let sequence = &protein.protein;
        let mut fragments = Vec::new();
        let mut start = 0;
        for m in re.find_iter(sequence).flatten() {
            let end = m.end();
            if end > start {
                fragments.push(sequence[start..end].to_string());
            }
            start = end;
        }
        if start < sequence.len() {
            fragments.push(sequence[start..].to_string());
        }
        fragments
    }).collect();

    peptides.retain(|p| p.len() >= min_length);
    peptides.sort();
    peptides.dedup();

    let total_peptides = peptides.len();

    let (peptides, results) = tokio::task::spawn_blocking(move || {
        let results = index.analyse(&peptides, false, false, Some(10_000));
        (peptides, results)
    }).await?;

    let unique_peptides: Vec<String> = peptides.into_iter()
        .zip(results.into_iter())
        .filter_map(|(peptide, result)| {
            if result.cutoff_used || result.proteins.is_empty() {
                return None;
            }
            if result.proteins.iter().all(|p| p.taxon == taxon_id) {
                Some(peptide)
            } else {
                None
            }
        })
        .collect();

    let total_unique_peptides = unique_peptides.len();

    Ok(UniquePeptidesResult {
        unique_peptides,
        total_peptides,
        total_unique_peptides,
    })
}

generate_handlers!(
    async fn json_handler(
        state => State<AppState>,
        params => Parameters
    ) -> Result<Json<UniquePeptidesResult>, ApiError> {
        Ok(Json(handler(state, params).await?))
    }
);

#[cfg(test)]
mod tests {
    use fancy_regex::Regex;

    fn cleave(sequence: &str, pattern: &str) -> Vec<String> {
        let re = Regex::new(pattern).unwrap();
        let mut fragments = Vec::new();
        let mut start = 0;
        for m in re.find_iter(sequence).flatten() {
            let end = m.end();
            if end > start {
                fragments.push(sequence[start..end].to_string());
            }
            start = end;
        }
        if start < sequence.len() {
            fragments.push(sequence[start..].to_string());
        }
        fragments
    }

    #[test]
    fn tryptic_cleavage_splits_after_k_and_r_not_before_p() {
        // K at pos 1 not followed by P → split after K; R at end of string → split after R
        assert_eq!(cleave("MKVTLPGAR", "[KR](?!P)"), vec!["MK", "VTLPGAR"]);
    }

    #[test]
    fn tryptic_cleavage_skips_k_before_p() {
        // K at pos 3 is followed by P → no split; R at end of string → split after R
        assert_eq!(cleave("ACTKPDEFR", "[KR](?!P)"), vec!["ACTKPDEFR"]);
    }
}
