use super::{AssetResponse, Node};
use super::{Error, Extensions};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Options(Value);

impl Options {
    pub fn take<D: DeserializeOwned>(&mut self, prop: &str) -> Option<D> {
        let o = match self.0.as_object_mut() {
            Some(o) => o,
            None => return None,
        };

        match o.remove(prop) {
            Some(s) => match serde_json::from_value(s) {
                Ok(v) => Some(v),
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn get<D: DeserializeOwned>(&self, prop: &str) -> Option<D> {
        let o = match self.0.as_object() {
            Some(o) => o,
            None => return None,
        };

        match o.get(prop) {
            Some(s) => match serde_json::from_value(s.clone()) {
                Ok(v) => Some(v),
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn contains(&self, prop: &str) -> bool {
        self.0
            .as_object()
            .map(|m| m.contains_key(prop))
            .unwrap_or(false)
    }
}

pub struct AssetRequest {
    pub(crate) path: String,
    pub(crate) args: Option<Options>,
    pub(crate) extensions: Extensions,
}

impl AssetRequest {
    pub fn new(path: impl ToString) -> AssetRequest {
        AssetRequest {
            path: path.to_string(),
            args: None,
            extensions: Extensions::new(),
        }
    }

    pub fn with_args<S: Serialize>(mut self, args: S) -> Result<Self, Error> {
        let value = serde_json::to_value(args).unwrap();
        self.args = Some(Options(value));
        Ok(self)
    }

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
