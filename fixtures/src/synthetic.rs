//! A generated taxonomy, for measurements that need a realistic working set.
//!
//! The corpus in [`crate`] is twenty-six taxa, so that an expected LCA is checkable by eye. That
//! is the wrong shape for a benchmark of anything that walks lineages: a handful of them stays in
//! L1, and a pointer walk over the lineage store costs nothing measurable. This module builds a
//! taxonomy wide enough for that walk to miss cache, without committing a corpus to the
//! repository.
//!
//! It is generated, not real. Nothing here should be asserted against: the names are `taxon_0`
//! upwards and the ancestry is arithmetic. The corpus is what tests read; this is what benchmarks
//! measure over.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf}
};

use datastore::{LineageRank, LineageStore, TaxonStore};

/// Leaf taxon ids start here, clear of the ancestor ids [`ancestor`] hands out.
pub const LEAF_BASE: u32 = 10_000_000;

/// The number of ranks a lineage row carries, from the store that reads them.
const RANKS: usize = LineageStore::AMOUNT_OF_RANKS;

/// Each rank's ancestor ids live in a block this wide, so no two ranks name the same taxon.
pub const RANK_BLOCK: u32 = 100_000;

/// The widest taxonomy that fits: at the deepest rank every leaf has an ancestor of its own, and
/// those ids have to stay inside one block.
pub const MAX_DISTINCT: u32 = RANK_BLOCK;

/// The paths [`write_taxonomy`] wrote, in the order the stores take them.
pub struct TaxonomyPaths {
    pub taxons: PathBuf,
    pub lineages: PathBuf
}

/// The shift that splits `distinct` leaves into exactly two buckets.
///
/// Derived from the width rather than fixed, so rank 0 yields two domains at every size. A fixed
/// shift only does that over one span of widths, and silently yields one domain below it — which
/// makes a reduction stop at rank 0 instead of walking every rank.
fn top_shift(distinct: u32) -> usize {
    (u32::BITS - (distinct - 1).leading_zeros()).saturating_sub(1) as usize
}

/// The ancestor taxon of leaf `leaf` at rank index `rank`, over a taxonomy of `distinct` leaves,
/// or `None` where nothing is recorded.
///
/// Leaves fall into ever finer buckets as the rank deepens: two at `domain`, one per leaf at
/// `forma`. A rank's ids sit in its own [`RANK_BLOCK`], because a taxon id in a lineage row is a
/// taxon in its own right and no two ranks may name the same one.
pub fn ancestor(leaf: u32, rank: usize, distinct: u32) -> Option<i32> {
    // Every eleventh leaf has a gap at three of the ranks. Real lineages are not fully populated,
    // and a reader that assumes they are has no case here that catches it.
    if leaf.is_multiple_of(11) && (rank == 3 || rank == 8 || rank == 20) {
        return None;
    }

    let top = top_shift(distinct);
    let shift = top - (rank * top) / (RANKS - 1);

    Some((1_000_000 + rank as u32 * RANK_BLOCK + (leaf >> shift)) as i32)
}

/// Writes a taxonomy of `distinct` leaves into `dir` and returns the two paths.
///
/// `taxons.tsv` carries a row for every leaf, every ancestor those leaves name, and root, so that
/// an id the reduction answers with resolves through `TaxonStore`. Every one of them is valid: a
/// caller measuring with `only_valid_taxa` on wants the whole input to reach the lineage walk
/// rather than to be filtered out ahead of it.
///
/// # Panics
///
/// If `distinct` is below 2, which cannot split into two domains, or above [`MAX_DISTINCT`], where
/// the deepest rank's ids would run out of their block and collide with the next rank's.
pub fn write_taxonomy(dir: &Path, distinct: u32) -> TaxonomyPaths {
    assert!(distinct >= 2, "a taxonomy of {distinct} leaves cannot split into two domains");
    assert!(distinct <= MAX_DISTINCT, "a taxonomy of {distinct} leaves overruns the {RANK_BLOCK}-wide rank blocks");

    // Ancestor id to the rank it belongs to. Shallow ranks are named by many leaves, so the map
    // collapses those to one row each.
    let mut ancestors: BTreeMap<i32, usize> = BTreeMap::new();
    let mut lineages = String::new();

    for leaf in 0..distinct {
        write!(lineages, "{}", LEAF_BASE + leaf).expect("writing to a String cannot fail");

        for rank in 0..RANKS {
            match ancestor(leaf, rank, distinct) {
                Some(ancestor) => {
                    ancestors.insert(ancestor, rank);
                    write!(lineages, "\t{ancestor}")
                }
                None => write!(lineages, "\t\\N")
            }
            .expect("writing to a String cannot fail");
        }

        lineages.push('\n');
    }

    // The fifth column is the validity byte, 0x01 for a valid taxon.
    let no_rank: String = LineageRank::NoRank.into();
    let mut taxons = format!("1\troot\t{no_rank}\t1\t\u{1}\n");

    for (id, rank) in &ancestors {
        // `LineageRank::LINEAGE_ORDER` names the columns in the order they are read, and its
        // string form is the spelling the taxon table's rank column takes.
        let name: String = LineageRank::LINEAGE_ORDER[*rank].clone().into();
        writeln!(taxons, "{id}\tancestor_{id}\t{name}\t1\t\u{1}").expect("writing cannot fail");
    }

    for leaf in 0..distinct {
        let id = LEAF_BASE + leaf;
        let species: String = LineageRank::Species.into();
        writeln!(taxons, "{id}\ttaxon_{leaf}\t{species}\t1\t\u{1}").expect("writing to a String cannot fail");
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

/// Writes a taxonomy of `distinct` leaves into `dir` and loads it into the two stores.
///
/// What a caller almost always wants: the files exist only to be parsed. [`write_taxonomy`] is
/// still there for a caller that needs the paths themselves.
///
/// The stores hold everything they read, so `dir` may be deleted as soon as this returns.
///
/// # Panics
///
/// If `distinct` is outside the range [`write_taxonomy`] accepts, or if either store rejects what
/// was written, which is a broken generator rather than a condition to handle.
pub fn load_taxonomy(dir: &Path, distinct: u32) -> (TaxonStore, LineageStore) {
    let paths = write_taxonomy(dir, distinct);

    let taxons = TaxonStore::try_from_file(&paths.taxons.to_string_lossy())
        .unwrap_or_else(|err| panic!("the generated taxons should load: {err:?}"));
    let lineages = LineageStore::try_from_file(&paths.lineages.to_string_lossy())
        .unwrap_or_else(|err| panic!("the generated lineages should load: {err:?}"));

    (taxons, lineages)
}

/// `count` taxon ids drawn from the `distinct` leaves [`write_taxonomy`] wrote.
///
/// Scattered by a multiplicative hash rather than taken in order, so consecutive draws land far
/// apart in the store and the lookups miss cache the way a request's do. The draw is even, where a
/// real request draws some taxa far more often than others.
///
/// # Panics
///
/// If `distinct` is 0. Pass the same width [`write_taxonomy`] was given.
pub fn draws(distinct: u32, count: usize) -> Vec<u32> {
    assert!(distinct > 0, "there is nothing to draw from");

    (0..count).map(|i| LEAF_BASE + (i as u32).wrapping_mul(2_654_435_761) % distinct).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::*;

    fn distinct_at(rank: usize, distinct: u32) -> BTreeSet<i32> {
        (0..distinct).filter_map(|leaf| ancestor(leaf, rank, distinct)).collect()
    }

    /// Two domains at every width, not only the one the benchmark happens to use.
    ///
    /// A fixed top shift splits into two over one span of widths and yields a single domain below
    /// it, which makes a reduction stop at rank 0 rather than walking every rank — silently, since
    /// the answer is still a valid taxon.
    #[test]
    fn every_width_splits_into_two_domains() {
        for distinct in [2, 3, 17, 1_000, 16_384, 16_385, 26_919, MAX_DISTINCT] {
            assert_eq!(distinct_at(0, distinct).len(), 2, "{distinct} leaves");
        }
    }

    /// The deepest rank names one ancestor per leaf, so the reduction cannot stop early there.
    #[test]
    fn the_deepest_rank_is_unique_per_leaf() {
        for distinct in [2, 17, 1_000, 26_919] {
            assert_eq!(distinct_at(RANKS - 1, distinct).len(), distinct as usize, "{distinct} leaves");
        }
    }

    /// Ancestry narrows monotonically, which is what makes this a tree rather than noise.
    #[test]
    fn each_rank_is_at_least_as_fine_as_the_one_above() {
        let distinct = 26_919;

        // Rank 8 and 20 have gaps, which drop leaves rather than merge them, so compare across
        // them rather than through them.
        let widths: Vec<usize> = (0..RANKS)
            .filter(|rank| ![3, 8, 20].contains(rank))
            .map(|rank| distinct_at(rank, distinct).len())
            .collect();

        for pair in widths.windows(2) {
            assert!(pair[1] >= pair[0], "a rank must not be coarser than the one above it: {pair:?}");
        }
    }

    /// No id may be handed out at two different ranks.
    ///
    /// Within one rank many leaves share an ancestor, which is the point, so the distinct
    /// `(rank, id)` pairs are collected first and the id column is then checked for duplicates.
    #[test]
    fn no_two_ranks_share_an_ancestor_id() {
        let distinct = 4_096;

        let pairs: BTreeSet<(usize, i32)> = (0..RANKS)
            .flat_map(|rank| (0..distinct).filter_map(move |leaf| ancestor(leaf, rank, distinct).map(|id| (rank, id))))
            .collect();

        let mut seen: HashSet<i32> = HashSet::new();
        for (rank, id) in &pairs {
            assert!(seen.insert(*id), "id {id} is handed out at rank {rank} and at another rank");
        }
    }

    /// Ancestor ids must stay clear of the leaf ids, or a lineage would name a leaf as an ancestor.
    #[test]
    fn ancestor_ids_stay_below_the_leaves() {
        for distinct in [2, 1_000, MAX_DISTINCT] {
            for rank in 0..RANKS {
                for id in distinct_at(rank, distinct) {
                    assert!((id as u32) < LEAF_BASE, "{id} at rank {rank} collides with the leaves");
                }
            }
        }
    }

    #[test]
    fn the_gaps_fall_where_they_are_meant_to() {
        let distinct = 1_000;

        assert_eq!(ancestor(11, 8, distinct), None, "every eleventh leaf has its gaps");
        assert!(ancestor(11, 7, distinct).is_some(), "the gaps must not swallow the neighbouring ranks");
        assert!(ancestor(12, 8, distinct).is_some(), "the gaps must not apply to every leaf");
    }

    #[test]
    fn the_draws_come_from_the_leaves_that_were_written() {
        let distinct: u32 = 1_000;
        let drawn = draws(distinct, 10_000);

        assert_eq!(drawn.len(), 10_000);
        assert!(drawn.iter().all(|&id| (LEAF_BASE..LEAF_BASE + distinct).contains(&id)));

        let unique: BTreeSet<u32> = drawn.iter().copied().collect();
        assert!(unique.len() > 1, "a draw that returns one taxon measures nothing");
    }
}
