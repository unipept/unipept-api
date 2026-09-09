use datastore::{LineageStore, TaxonRank, TaxonStore};
use frequency::FrequencyTable;
use node::Node;

use super::lineage_helper::get_lineage_array;

pub mod frequency;
pub mod node;

pub fn build_tree(frequencies: FrequencyTable<u32>, lineage_store: &LineageStore, taxon_store: &TaxonStore) -> Node {
    let mut root: Node = Node::new(1, "Organism".to_string(), TaxonRank::NO_RANK.to_string());
    for taxon_id in frequencies.keys() {
        let mut current_node = &mut root;

        let lineage = get_lineage_array(*taxon_id, lineage_store);

        for lineage_id in lineage.into_iter().flatten() {
            if lineage_id < 0 {
                continue;
            }

            let child = current_node.get_child(lineage_id as usize);
            if child.is_none() {
                // A lineage may name an ancestor the taxon table does not — the two are separate
                // dumps, and a taxon can be dropped from one without the other. This used to
                // unwrap, so an incomplete pair turned every request touching that branch into a
                // panic. The rest of the API renders such a taxon with an empty name, and the node
                // is still added so the branch below it hangs in the right place.
                let (name, rank) = match taxon_store.get(lineage_id as u32) {
                    Some((name, rank, _)) => (name.clone(), rank.to_string()),
                    None => (String::new(), TaxonRank::NO_RANK.to_string())
                };
                current_node.add_child(Node::new(lineage_id as usize, name, rank));
            }

            current_node = current_node.get_child(lineage_id as usize).unwrap();
        }

        current_node.data.self_count += frequencies.get(taxon_id).unwrap_or(&0);
    }

    root.count();
    root.sort();

    root
}
