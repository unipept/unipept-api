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

/// The rows of one page, in order, without putting the rest of the table in order.
///
/// A window of a few dozen rows out of a filtered taxonomy is the usual request, and sorting the
/// whole table to answer it is most of the work. `select_nth_unstable_by` partitions instead, in
/// linear time, and only the window itself is sorted.
///
/// Selecting unstably is safe here because `compare` is a total order: every tie is broken on an id
/// no two rows share, so no two rows compare equal and the partition cannot vary between calls. A
/// comparison that left ties equal would hand back arbitrary rows for a page.
pub fn page_of<T: Ord>(rows: &mut [T], start: usize, end: usize, sort_descending: bool) -> &[T] {
    // Reversed whole rather than per field, so a descending page is the exact reverse of the
    // ascending one even where the sort field repeats and the id settles the tie.
    let compare = |a: &T, b: &T| {
        let ordering = a.cmp(b);
        if sort_descending { ordering.reverse() } else { ordering }
    };

    let end = end.min(rows.len());
    if start >= end {
        return &[];
    }

    // Everything below the window to the left of it, so `rows[..end]` holds the first `end` rows.
    if end < rows.len() {
        rows.select_nth_unstable_by(end - 1, compare);
    }

    // And everything below the window's start to the left of that, leaving the window itself.
    let head = &mut rows[..end];
    if start > 0 {
        head.select_nth_unstable_by(start, compare);
    }

    head[start..].sort_unstable_by(compare);

    &rows[start..end]
}
