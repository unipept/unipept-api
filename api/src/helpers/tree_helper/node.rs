use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct NodeData {
    pub count:      usize,
    pub self_count: usize
}

#[derive(Serialize, Clone)]
pub struct Node {
    pub id:       usize,
    pub name:     String,
    pub rank:     String,
    pub data:     NodeData,
    pub children: Vec<Node>
}

impl NodeData {
    pub fn new() -> NodeData {
        NodeData {
            count:      0,
            self_count: 0
        }
    }
}

impl Node {
    pub fn new(id: usize, name: String, rank: String) -> Node {
        Node {
            id,
            name,
            rank,
            data: NodeData::new(),
            children: Vec::new()
        }
    }

    pub fn get_child(&mut self, id: usize) -> Option<&mut Node> {
        self.children.iter_mut().find(|child| child.id == id)
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn count(&mut self) {
        for child in self.children.iter_mut() {
            child.count();
        }

        self.data.count = self
            .children
            .iter()
            .map(|child| child.data.count)
            .sum::<usize>()
            + self.data.self_count;
    }
}
