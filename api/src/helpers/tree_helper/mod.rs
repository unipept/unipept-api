use frequency::FrequencyTable;
use node::Node;

pub mod node;
pub mod frequency;

pub fn build_tree(lineages: &Vec<&Vec<Option<i32>>>, frequencies: FrequencyTable<usize>) {
    let root = Node::new(1, "root".to_string(), "no rank".to_string());
}
