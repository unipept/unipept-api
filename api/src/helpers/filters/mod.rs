use index::ProteinInfo;

pub mod crap_filter;
pub mod empty_filter;
pub mod protein_filter;
pub mod proteome_filter;
pub mod taxa_filter;

pub trait UniprotFilter {
    fn filter(&self, protein: &ProteinInfo) -> bool;
}
