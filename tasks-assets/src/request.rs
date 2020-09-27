use super::Extensions;
use super::{AssetResponse, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Options(Map<String, Value>);

pub struct AssetRequest {
    pub(crate) path: String,
    pub(crate) args: Options,
    pub(crate) extensions: Extensions,
}

impl AssetRequest {
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }

    pub fn reply(self, node: Node) -> AssetResponse {
        AssetResponse {
            request: self,
            node,
        }
    }
}
