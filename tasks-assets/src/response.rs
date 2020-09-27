use super::{AssetRequest, Extensions, Node};

pub struct AssetResponse {
    pub(crate) request: AssetRequest,
    pub(crate) node: Node,
}

impl AssetResponse {
    pub fn node(&self) -> &Node {
        &self.node
    }

    pub fn into_node(self) -> Node {
        self.node
    }
}
