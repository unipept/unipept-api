use index::ProteinInfo;
use crate::helpers::filters::UniprotFilter;

pub struct EmptyFilter;

impl UniprotFilter for EmptyFilter {
    fn filter(&self, _protein: &ProteinInfo) -> bool {
        true
    }
}

impl EmptyFilter {
    pub fn new() -> Self {
        EmptyFilter
    }
}