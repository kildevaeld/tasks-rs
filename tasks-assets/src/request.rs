use super::{cache::CacheKey, Asset, AssetResponse, MountPath};
use super::{Error, Extensions};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

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
    pub(crate) args: Options,
    pub(crate) extensions: Extensions,
    pub(crate) cache: bool,
}

impl AssetRequest {
    pub fn new(path: impl AsRef<str>) -> AssetRequest {
        let path = if path.as_ref().is_empty() {
            "/".to_owned()
        } else if path.as_ref().chars().nth(0) != Some('/') {
            format!("/{}", path.as_ref())
        } else {
            path.as_ref().to_string()
        };

        AssetRequest {
            path: path,
            args: Options::default(),
            extensions: Extensions::new(),
            cache: true,
        }
    }

    pub fn with_args<S: Serialize>(mut self, args: S) -> Result<Self, Error> {
        let value = serde_json::to_value(args).unwrap();
        self.args = Options(value);
        Ok(self)
    }

    pub fn args(&self) -> &Options {
        &self.args
    }

    pub fn args_mut(&mut self) -> &mut Options {
        &mut self.args
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

    pub fn reply(self, node: Asset) -> AssetResponse {
        AssetResponse {
            request: self,
            node,
        }
    }

    pub(crate) fn cache_key(&self) -> Result<CacheKey, Box<dyn std::error::Error>> {
        let mut key = self.path.as_bytes().to_vec();
        if self.args.0 != Value::Null {
            cache_key_value(&self.args.0, &mut key)?;
        }
        let key = Sha256::digest(&key);
        Ok(CacheKey(key.as_slice().to_vec()))
    }

    pub fn real_path(&self) -> String {
        match self.extensions.get::<MountPath>() {
            Some(m) => m.real_path(self),
            None => self.path.clone(),
        }
    }
}

fn cache_key_value(
    value: &Value,
    out: &mut dyn std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        Value::Array(a) => cache_key_array(a, out)?,
        Value::Object(b) => cache_key_object(b, out)?,
        _ => {
            serde_json::to_writer(out, value)?;
        }
    }

    Ok(())
}

fn cache_key_array(
    value: &Vec<Value>,
    out: &mut dyn std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut v = Vec::new();
    for i in value.iter() {
        let mut t = Vec::new();
        cache_key_value(i, &mut t)?;
        v.push(t);
    }

    v.sort();

    out.write_all(&mut v.concat())?;

    Ok(())
}

fn cache_key_object(
    value: &Map<String, Value>,
    out: &mut dyn std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut list = Vec::new();
    for (k, v) in value.iter() {
        let mut t = k.as_bytes().to_vec();
        cache_key_value(v, &mut t)?;
        list.push(t);
    }

    list.sort();

    out.write_all(&mut list.concat())?;

    Ok(())
}
