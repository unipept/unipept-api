pub mod hybrid;
pub mod iterative;
pub mod lca;
pub mod lca_star;
pub mod mrtl;

use datastore::{LineageStore, TaxonStore};
use thiserror::Error;

use crate::helpers::lineage_helper::LineageVersion;

#[derive(Debug, Error)]
pub enum AggregationError {
    #[error("Unknown aggregation method '{0}'. Valid methods: lca, lca_star, mrtl, hybrid, iterative")]
    UnknownMethod(String),
    #[error("Method '{0}' requires a threshold (integer percentage 0-100), e.g. taxa_aggregation_threshold=75")]
    ThresholdRequired(String),
    #[error("Method '{0}' does not accept a threshold")]
    ThresholdNotSupported(String),
    #[error("Threshold {0} is out of range: expected an integer between 0 and 100")]
    ThresholdOutOfRange(u32),
}

pub trait TaxaAggregation {
    fn aggregate(
        &self,
        taxa: Vec<u32>,
        version: LineageVersion,
        taxon_store: &TaxonStore,
        lineage_store: &LineageStore,
        only_valid_taxa: bool,
    ) -> i32;
}

pub enum Aggregation {
    Lca,
    LcaStar,
    Mrtl,
    Hybrid    { threshold: f64 },
    Iterative { threshold: f64 },
}

impl TaxaAggregation for Aggregation {
    fn aggregate(
        &self,
        taxa: Vec<u32>,
        version: LineageVersion,
        taxon_store: &TaxonStore,
        lineage_store: &LineageStore,
        only_valid_taxa: bool,
    ) -> i32 {
        match self {
            Aggregation::Lca =>
                lca::calculate_lca(taxa, version, taxon_store, lineage_store, only_valid_taxa),
            Aggregation::LcaStar =>
                lca_star::calculate_lca_star(taxa, version, taxon_store, lineage_store, only_valid_taxa),
            Aggregation::Mrtl =>
                mrtl::calculate_mrtl(taxa, version, taxon_store, lineage_store, only_valid_taxa),
            Aggregation::Hybrid { threshold } =>
                hybrid::calculate_hybrid(taxa, version, taxon_store, lineage_store, only_valid_taxa, *threshold),
            Aggregation::Iterative { threshold } =>
                iterative::calculate_iterative(taxa, version, taxon_store, lineage_store, only_valid_taxa, *threshold),
        }
    }
}

pub fn parse_aggregation(method: &str, threshold: Option<u32>) -> Result<Aggregation, AggregationError> {
    let lower = method.to_lowercase();
    match lower.as_str() {
        "lca"               => no_threshold(method, threshold).map(|_| Aggregation::Lca),
        "lca_star" | "lca*" => no_threshold(method, threshold).map(|_| Aggregation::LcaStar),
        "mrtl"              => no_threshold(method, threshold).map(|_| Aggregation::Mrtl),
        "hybrid"            => with_threshold(method, threshold).map(|t| Aggregation::Hybrid { threshold: t }),
        "iterative"         => with_threshold(method, threshold).map(|t| Aggregation::Iterative { threshold: t }),
        _                   => Err(AggregationError::UnknownMethod(method.to_string())),
    }
}

fn no_threshold(method: &str, threshold: Option<u32>) -> Result<(), AggregationError> {
    if threshold.is_some() {
        Err(AggregationError::ThresholdNotSupported(method.to_string()))
    } else {
        Ok(())
    }
}

fn with_threshold(method: &str, threshold: Option<u32>) -> Result<f64, AggregationError> {
    let pct = threshold.ok_or_else(|| AggregationError::ThresholdRequired(method.to_string()))?;
    if pct > 100 {
        return Err(AggregationError::ThresholdOutOfRange(pct));
    }
    Ok(pct as f64 / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lca_parses() {
        assert!(parse_aggregation("lca", None).is_ok());
        assert!(parse_aggregation("LCA", None).is_ok());
    }

    #[test]
    fn lca_star_parses() {
        assert!(parse_aggregation("lca_star", None).is_ok());
        assert!(parse_aggregation("lca*", None).is_ok());
        assert!(parse_aggregation("LCA*", None).is_ok());
    }

    #[test]
    fn mrtl_parses() {
        assert!(parse_aggregation("mrtl", None).is_ok());
        assert!(parse_aggregation("MRTL", None).is_ok());
    }

    #[test]
    fn hybrid_valid_thresholds() {
        assert!(parse_aggregation("hybrid", Some(0)).is_ok());
        assert!(parse_aggregation("hybrid", Some(50)).is_ok());
        assert!(parse_aggregation("hybrid", Some(100)).is_ok());
    }

    #[test]
    fn hybrid_threshold_out_of_range() {
        let err = parse_aggregation("hybrid", Some(101)).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdOutOfRange(_)));
    }

    #[test]
    fn hybrid_without_threshold() {
        let err = parse_aggregation("hybrid", None).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdRequired(_)));
    }

    #[test]
    fn iterative_valid_thresholds() {
        assert!(parse_aggregation("iterative", Some(0)).is_ok());
        assert!(parse_aggregation("iterative", Some(50)).is_ok());
        assert!(parse_aggregation("iterative", Some(100)).is_ok());
    }

    #[test]
    fn iterative_threshold_out_of_range() {
        let err = parse_aggregation("iterative", Some(200)).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdOutOfRange(_)));
    }

    #[test]
    fn iterative_without_threshold() {
        let err = parse_aggregation("iterative", None).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdRequired(_)));
    }

    #[test]
    fn threshold_not_supported_for_lca() {
        let err = parse_aggregation("lca", Some(50)).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdNotSupported(_)));
    }

    #[test]
    fn threshold_not_supported_for_mrtl() {
        let err = parse_aggregation("mrtl", Some(50)).err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdNotSupported(_)));
    }

    #[test]
    fn unknown_method() {
        let err = parse_aggregation("unknown", None).err().unwrap();
        assert!(matches!(err, AggregationError::UnknownMethod(_)));
    }

    #[test]
    fn empty_string() {
        let err = parse_aggregation("", None).err().unwrap();
        assert!(matches!(err, AggregationError::UnknownMethod(_)));
    }

    #[test]
    fn default_method_is_lca() {
        // The default taxa_aggregation_method is "lca" with no threshold.
        // This test documents that the default remains backward-compatible.
        assert!(parse_aggregation("lca", None).is_ok());
    }
}
