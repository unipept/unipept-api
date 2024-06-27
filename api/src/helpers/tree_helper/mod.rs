use frequency::FrequencyTable;
use node::Node;

pub mod node;
pub mod frequency;

pub fn build_tree(_lineages: &Vec<&Vec<Option<i32>>>, _frequencies: FrequencyTable<usize>) {
    let _root = Node::new(1, "root".to_string(), "no rank".to_string());
}
