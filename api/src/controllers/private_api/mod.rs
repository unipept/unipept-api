use std::cmp::Ordering;

use crate::controllers::request::Flag;

pub mod ecnumbers;
pub mod goterms;
pub mod interpros;
pub mod metadata;
pub mod proteins;
pub mod proteins_filter;
pub mod reference_proteomes;
pub mod reference_proteomes_filter;
pub mod taxa;
pub mod taxa2rank;
pub mod taxa_filter;

pub fn default_equate_il() -> Flag {
    Flag(false)
}

pub fn default_sort_descending() -> Flag {
    Flag(false)
}

/// Reverses an ordering for a filter sorting descending.
///
/// Taken whole rather than per field, so a descending page is the exact reverse of the ascending
/// one even where the sort field repeats and the id settles the tie.
pub fn reversed_when_descending(ordering: Ordering, sort_descending: bool) -> Ordering {
    if sort_descending { ordering.reverse() } else { ordering }
}
