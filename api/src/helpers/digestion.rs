use datastore::{LineageRank, TaxonStore};
use fancy_regex::Regex;

use crate::errors::ApiError;

pub fn default_cleavage_regex() -> String {
    String::from("[KR](?!P)")
}

pub fn default_min_length() -> usize {
    5
}

/// Splits a single amino acid sequence at every regex match end position.
/// The matched character stays in the preceding fragment (trypsin-style).
pub fn cleave_sequence(sequence: &str, re: &Regex) -> Vec<String> {
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

/// Compiles a cleavage regex, returning an appropriate API error on failure.
pub fn compile_cleavage_regex(pattern: &str) -> Result<Regex, ApiError> {
    Regex::new(pattern)
        .map_err(|e| ApiError::InvalidRegexError(format!("Invalid cleavage_regex: {}", e)))
}

/// Validates that a taxon is at species or strain rank.
pub fn validate_taxon_rank(taxon_store: &TaxonStore, taxon_id: u32) -> Result<(), ApiError> {
    let rank = taxon_store
        .get_rank(taxon_id)
        .ok_or_else(|| ApiError::UnknownRankError(format!("Taxon {} not found", taxon_id)))?;
    if *rank != LineageRank::Species && *rank != LineageRank::Strain {
        return Err(ApiError::UnknownRankError(format!(
            "Taxon {} is at rank '{}', but must be at species or strain level",
            taxon_id, rank
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tryptic_cleavage_splits_after_k_and_r_not_before_p() {
        let re = Regex::new("[KR](?!P)").unwrap();
        // K at pos 1 not followed by P → split after K; R at end of string → split after R
        assert_eq!(cleave_sequence("MKVTLPGAR", &re), vec!["MK", "VTLPGAR"]);
    }

    #[test]
    fn tryptic_cleavage_skips_k_before_p() {
        let re = Regex::new("[KR](?!P)").unwrap();
        // K at pos 3 is followed by P → no split; R at end of string → split after R
        assert_eq!(cleave_sequence("ACTKPDEFR", &re), vec!["ACTKPDEFR"]);
    }
}
