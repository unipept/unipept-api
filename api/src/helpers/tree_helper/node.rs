use serde::Serialize;

#[derive(Serialize)]
pub struct NodeData {
    pub count: usize,
    pub self_count: usize,
    pub rank: String
}

#[derive(Serialize)]
pub struct Node {
    pub id: usize,
    pub name: String,
    pub data: NodeData,
    pub children: Vec<Node>
}

impl NodeData {
    pub fn new(rank: String) -> NodeData {
        NodeData {
            count: 0,
            self_count: 0,
            rank
        }
    }
}

impl Node {
    pub fn new(id: usize, name: String, rank: String) -> Node {
        Node {
            id,
            name,
            data: NodeData::new(rank),
            children: Vec::new()
        }
    }
}
