use datastore::{
    LineageStore,
    TaxonStore
};
use frequency::FrequencyTable;
use node::Node;

use super::lineage_helper::{
    get_amount_of_ranks,
    get_lineage_array,
    LineageVersion
};

pub mod frequency;
pub mod node;

pub fn build_tree(
    frequencies: FrequencyTable<u32>,
    version: LineageVersion,
    lineage_store: &LineageStore,
    taxon_store: &TaxonStore
) -> Node {
    let amount_of_ranks = get_amount_of_ranks(version) as usize;

    let mut root: Node = Node::new(1, "Organism".to_string(), "no rank".to_string());
    for taxon_id in frequencies.keys() {
        let mut current_node = &mut root;

        let lineage = get_lineage_array(*taxon_id, version, lineage_store);

        for rank in 0 .. amount_of_ranks {
            if let Some(lineage_id) = lineage[rank] {
                if lineage_id < 0 {
                    continue;
                }

                let child = current_node.get_child(lineage_id as usize);
                if child.is_none() {
                    let (name, rank) = taxon_store.get(lineage_id as u32).unwrap();
                    current_node.add_child(Node::new(
                        lineage_id as usize,
                        name.clone(),
                        rank.clone().into()
                    ));
                }

                current_node = current_node.get_child(lineage_id as usize).unwrap();
            }
        }

        current_node.data.self_count += frequencies.get(taxon_id).unwrap_or(&0);
    }

    root.count();

    root
}
