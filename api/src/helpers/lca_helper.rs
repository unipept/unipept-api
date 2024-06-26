use datastore::LineageStore;

use super::lineage_helper::{get_amount_of_ranks, get_genus_index, get_lineage_array, get_species_index, LineageVersion};

pub fn calculate_lca(taxa: Vec<u32>, version: LineageVersion, lineage_store: &LineageStore) -> i32 {
    let mut lca = 1;

    let lineages = prepare_lineages(taxa, version, lineage_store);

    let amount_of_ranks = get_amount_of_ranks(version);
    let genus_index = get_genus_index(version);
    let species_index = get_species_index(version);

    for rank in 0..amount_of_ranks {
        let final_rank = rank;
        let mut value = -1;

        let iterator = lineages
            .iter()
            .map(|x| x[final_rank as usize])
            .filter(|&x| {
                if final_rank == genus_index || final_rank == species_index {
                    x > 0
                } else {
                    x >= 0
                }
            });

        // Check if all elements in the iterator are the same
        // This was near-impossible to do with the iterators above,
        // so we're using a simplified loop here
        for item in iterator {
            if value == -1 {
                value = item;
            } else if item != value {
                return lca;
            }
        }

        // If we found a new value that matched for all of them, use this as the new best
        if value > 0 {
            lca = value;
        }
    }

    lca
}

fn prepare_lineages(taxa: Vec<u32>, version: LineageVersion, lineage_store: &LineageStore) -> Vec<Vec<i32>> {
    taxa
        .into_iter()
        .map(|taxon_id| {
            get_lineage_array(taxon_id, version, lineage_store)
                .into_iter()
                .map(|x| x.unwrap_or(0))
                .collect()
        })
        .collect()
}
