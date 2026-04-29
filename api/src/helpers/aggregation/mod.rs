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
    #[error("Unknown aggregation method: '{0}'")]
    UnknownMethod(String),
    #[error("Invalid threshold for '{0}': expected integer 0-100")]
    InvalidThreshold(String),
    #[error("Threshold out of range for '{method}': expected 0-100, got {value}")]
    ThresholdOutOfRange { method: String, value: u32 },
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

pub fn parse_aggregation(s: &str) -> Result<Box<dyn TaxaAggregation + Send>, AggregationError> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "lca"               => Ok(Box::new(lca::Lca) as Box<dyn TaxaAggregation + Send>),
        "lca_star" | "lca*" => Ok(Box::new(lca_star::LcaStar)),
        "mrtl"              => Ok(Box::new(mrtl::Mrtl)),
        _ if lower.starts_with("hybrid_") => {
            let pct_str = &lower["hybrid_".len()..];
            let pct: u32 = pct_str.parse().map_err(|_| {
                AggregationError::InvalidThreshold(s.to_string())
            })?;
            if pct > 100 {
                return Err(AggregationError::ThresholdOutOfRange {
                    method: s.to_string(),
                    value: pct,
                });
            }
            Ok(Box::new(hybrid::Hybrid { threshold: pct as f64 / 100.0 }))
        }
        _ if lower.starts_with("iterative_") => {
            let pct_str = &lower["iterative_".len()..];
            let pct: u32 = pct_str.parse().map_err(|_| {
                AggregationError::InvalidThreshold(s.to_string())
            })?;
            if pct > 100 {
                return Err(AggregationError::ThresholdOutOfRange {
                    method: s.to_string(),
                    value: pct,
                });
            }
            Ok(Box::new(iterative::Iterative { threshold: pct as f64 / 100.0 }))
        }
        _ => Err(AggregationError::UnknownMethod(s.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lca_parses() {
        assert!(parse_aggregation("lca").is_ok());
        assert!(parse_aggregation("LCA").is_ok());
    }

    #[test]
    fn lca_star_parses() {
        assert!(parse_aggregation("lca_star").is_ok());
        assert!(parse_aggregation("lca*").is_ok());
        assert!(parse_aggregation("LCA*").is_ok());
    }

    #[test]
    fn mrtl_parses() {
        assert!(parse_aggregation("mrtl").is_ok());
        assert!(parse_aggregation("MRTL").is_ok());
    }

    #[test]
    fn hybrid_valid_thresholds() {
        assert!(parse_aggregation("hybrid_0").is_ok());
        assert!(parse_aggregation("hybrid_50").is_ok());
        assert!(parse_aggregation("hybrid_100").is_ok());
    }

    #[test]
    fn hybrid_threshold_out_of_range() {
        let err = parse_aggregation("hybrid_101").err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdOutOfRange { .. }));
    }

    #[test]
    fn hybrid_non_integer_threshold() {
        let err = parse_aggregation("hybrid_abc").err().unwrap();
        assert!(matches!(err, AggregationError::InvalidThreshold(_)));
    }

    #[test]
    fn iterative_valid_thresholds() {
        assert!(parse_aggregation("iterative_0").is_ok());
        assert!(parse_aggregation("iterative_50").is_ok());
        assert!(parse_aggregation("iterative_100").is_ok());
    }

    #[test]
    fn iterative_threshold_out_of_range() {
        let err = parse_aggregation("iterative_200").err().unwrap();
        assert!(matches!(err, AggregationError::ThresholdOutOfRange { .. }));
    }

    #[test]
    fn iterative_non_integer_threshold() {
        let err = parse_aggregation("iterative_0.5").err().unwrap();
        assert!(matches!(err, AggregationError::InvalidThreshold(_)));
    }

    #[test]
    fn unknown_method() {
        let err = parse_aggregation("unknown").err().unwrap();
        assert!(matches!(err, AggregationError::UnknownMethod(_)));
    }

    #[test]
    fn empty_string() {
        let err = parse_aggregation("").err().unwrap();
        assert!(matches!(err, AggregationError::UnknownMethod(_)));
    }
}
