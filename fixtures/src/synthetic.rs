//! A generated taxonomy, for measurements that need a realistic working set.
//!
//! The corpus in [`crate`] is twenty-six taxa, so that an expected LCA is checkable by eye. That
//! is the wrong shape for a benchmark of anything that walks lineages: a handful of them stays in
//! L1, and a pointer walk over the lineage store — which is what `calculate_lca` spends its time
//! on — costs nothing measurable. This module builds a taxonomy wide enough for that walk to miss
//! cache, without committing a corpus to the repository.
//!
//! It is generated, not real. Nothing here should be asserted against: the names are `taxon_0`
//! upwards and the ancestry is arithmetic. The corpus is what tests read; this is what benchmarks
//! measure over.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf}
};

/// Leaf taxon ids start here, clear of the ancestor ids [`ancestor`] hands out.
pub const LEAF_BASE: u32 = 10_000_000;

/// The number of ranks a lineage row carries.
const RANKS: usize = 28;

/// The paths [`write_taxonomy`] wrote, in the order the stores take them.
pub struct TaxonomyPaths {
    pub taxons: PathBuf,
    pub lineages: PathBuf
}

/// The ancestor taxon of leaf `leaf` at rank index `rank`, or `None` where nothing is recorded.
///
/// Leaves fall into ever finer buckets as the rank deepens: at `domain` the shift is 14, so up to
/// 32768 leaves fall into two domains and no rank is shared by all of them; at `forma` it is 0, so
/// every leaf has its own. A rank's ids are offset by the rank, because a taxon id in a lineage
/// row is a taxon in its own right and no two ranks name the same one.
pub fn ancestor(leaf: u32, rank: usize) -> Option<i32> {
    // Every eleventh leaf has a gap at three of the ranks. Real lineages are not fully populated,
    // and a reader that assumes they are has no case here that catches it.
    if leaf.is_multiple_of(11) && (rank == 3 || rank == 8 || rank == 20) {
        return None;
    }

    let shift = 14 - (rank * 14) / (RANKS - 1);
    Some((1_000_000 + rank as u32 * 100_000 + (leaf >> shift)) as i32)
}

/// Writes a taxonomy of `distinct` leaves into `dir` and returns the two paths.
///
/// Every leaf is valid. A caller measuring with `only_valid_taxa` on wants the whole input to
/// reach the lineage walk, not to be filtered out ahead of it.
///
/// `distinct` must not exceed 32768, or every leaf falls into one domain and the reduction stops
/// at that rank instead of walking all of them.
pub fn write_taxonomy(dir: &Path, distinct: u32) -> TaxonomyPaths {
    assert!(distinct <= 1 << 15, "more than 32768 leaves no longer split into two domains");

    let mut taxons = String::new();
    let mut lineages = String::new();

    for leaf in 0..distinct {
        let id = LEAF_BASE + leaf;

        // The fifth column is the validity byte, 0x01 for a valid taxon.
        writeln!(taxons, "{id}\ttaxon_{leaf}\tspecies\t1\t\u{1}").expect("writing to a String cannot fail");

        write!(lineages, "{id}").expect("writing to a String cannot fail");
        for rank in 0..RANKS {
            match ancestor(leaf, rank) {
                Some(ancestor) => write!(lineages, "\t{ancestor}"),
                None => write!(lineages, "\t\\N")
            }
            .expect("writing to a String cannot fail");
        }
        lineages.push('\n');
    }

    std::fs::create_dir_all(dir).unwrap_or_else(|err| panic!("could not create {}: {}", dir.display(), err));

    let paths = TaxonomyPaths {
        taxons: dir.join("taxons.tsv"),
        lineages: dir.join("lineages.tsv")
    };
    std::fs::write(&paths.taxons, taxons).expect("could not write the generated taxons");
    std::fs::write(&paths.lineages, lineages).expect("could not write the generated lineages");

    paths
}

/// `count` taxon ids drawn from the `distinct` leaves [`write_taxonomy`] wrote.
///
/// Scattered by a multiplicative hash rather than taken in order, so consecutive draws land far
/// apart in the store and the lookups miss cache the way a request's do. The draw is even, where a
/// real request draws some taxa far more often than others.
pub fn draws(distinct: u32, count: usize) -> Vec<u32> {
    (0..count).map(|i| LEAF_BASE + (i as u32).wrapping_mul(2_654_435_761) % distinct).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The properties a caller relies on, none of which the arithmetic makes obvious.
    #[test]
    fn the_generated_lineages_have_the_shape_callers_measure_against() {
        let distinct: u32 = 26_919;

        let domains: Vec<i32> = (0..distinct).filter_map(|leaf| ancestor(leaf, 0)).collect();
        let mut distinct_domains = domains.clone();
        distinct_domains.sort_unstable();
        distinct_domains.dedup();
        assert_eq!(distinct_domains.len(), 2, "the reduction must find no shared domain");

        let formae: Vec<i32> = (0..distinct).filter_map(|leaf| ancestor(leaf, RANKS - 1)).collect();
        let mut distinct_formae = formae.clone();
        distinct_formae.sort_unstable();
        distinct_formae.dedup();
        assert_eq!(distinct_formae.len(), formae.len(), "the deepest rank must be unique per leaf");

        assert_eq!(ancestor(11, 8), None, "every eleventh leaf has its gaps");
        assert!(ancestor(11, 7).is_some(), "the gaps must not swallow the neighbouring ranks");
        assert!(ancestor(12, 8).is_some(), "the gaps must not apply to every leaf");
    }

    /// No rank may name a taxon another rank also names.
    #[test]
    fn no_two_ranks_share_an_ancestor_id() {
        let mut seen: Vec<(usize, i32)> = Vec::new();
        for rank in 0..RANKS {
            for leaf in 0..4096u32 {
                if let Some(ancestor) = ancestor(leaf, rank) {
                    seen.push((rank, ancestor));
                }
            }
        }
        seen.sort_unstable_by_key(|&(_, ancestor)| ancestor);
        seen.dedup_by_key(|&mut (_, ancestor)| ancestor);

        let ranks_per_id: Vec<usize> = seen.iter().map(|&(rank, _)| rank).collect();
        assert_eq!(ranks_per_id.len(), seen.len());
    }

    #[test]
    fn the_draws_come_from_the_leaves_that_were_written() {
        let distinct: u32 = 1_000;
        let drawn = draws(distinct, 10_000);

        assert_eq!(drawn.len(), 10_000);
        assert!(drawn.iter().all(|&id| (LEAF_BASE..LEAF_BASE + distinct).contains(&id)));

        let mut unique = drawn.clone();
        unique.sort_unstable();
        unique.dedup();
        assert!(unique.len() > 1, "a draw that returns one taxon measures nothing");
    }
}
