use super::Extensions;
use serde_json::{Map, Value};
use serde::{Serialize, Deserialize}

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
}
