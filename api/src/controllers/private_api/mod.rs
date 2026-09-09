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

#[cfg(test)]
mod tests {
    use super::*;

    /// What `page_of` promises: the rows at sorted positions `[start, end)`, in order.
    fn expected(rows: &[u32], start: usize, end: usize, sort_descending: bool) -> Vec<u32> {
        let mut sorted = rows.to_vec();
        sorted.sort_unstable();
        if sort_descending {
            sorted.reverse();
        }

        sorted.into_iter().skip(start).take(end.saturating_sub(start)).collect()
    }

    fn page(rows: &[u32], start: usize, end: usize, sort_descending: bool) -> Vec<u32> {
        page_of(&mut rows.to_vec(), start, end, sort_descending).to_vec()
    }

    /// Every window of a shuffled table, against the sort it stands for.
    #[test]
    fn a_window_is_the_slice_of_the_sorted_rows_at_the_same_offsets() {
        let rows: Vec<u32> = (0..40u32).map(|i| i * 17 % 40).collect();

        for start in 0..rows.len() {
            for size in [1usize, 2, 7, 40] {
                for sort_descending in [false, true] {
                    let end = start + size;
                    assert_eq!(
                        page(&rows, start, end, sort_descending),
                        expected(&rows, start, end, sort_descending),
                        "[{start}, {end}) descending={sort_descending}"
                    );
                }
            }
        }
    }

    /// Consecutive pages are the listing, cut up: nothing repeated and nothing lost.
    #[test]
    fn pages_walked_end_to_end_rebuild_the_listing() {
        let rows: Vec<u32> = (0..37u32).map(|i| i * 11 % 37).collect();

        for sort_descending in [false, true] {
            let walked: Vec<u32> = (0..rows.len())
                .step_by(4)
                .flat_map(|start| page(&rows, start, start + 4, sort_descending))
                .collect();

            assert_eq!(walked, expected(&rows, 0, rows.len(), sort_descending), "descending={sort_descending}");
        }
    }

    /// A window may sit past the end, start at the end, or be empty. None of them may panic, and
    /// `end - 1` inside must never underflow.
    #[test]
    fn a_window_outside_the_rows_is_empty_rather_than_a_panic() {
        let rows: Vec<u32> = vec![3, 1, 2];

        assert_eq!(page(&rows, 0, 100, false), vec![1, 2, 3], "wider than the rows");
        assert_eq!(page(&rows, 2, 100, false), vec![3], "starts inside, ends past");
        assert!(page(&rows, 3, 6, false).is_empty(), "starts at the end");
        assert!(page(&rows, 10, 20, false).is_empty(), "starts past the end");
        assert!(page(&rows, 0, 0, false).is_empty(), "empty window");
        assert!(page(&rows, 2, 1, false).is_empty(), "end below start");
        assert!(page(&[], 0, 5, false).is_empty(), "no rows at all");
    }

    /// The rows this is given are tuples whose last field is a unique id, so no two compare equal.
    /// A table that does tie is what the partition cannot order, and this records that: the ids
    /// come back, but which of the tied rows lands on which page is not defined.
    #[test]
    fn tied_rows_still_yield_the_right_number_of_them() {
        let rows: Vec<u32> = vec![5, 5, 5, 5];

        assert_eq!(page(&rows, 0, 2, false), vec![5, 5]);
        assert_eq!(page(&rows, 2, 4, false), vec![5, 5]);
    }
}
