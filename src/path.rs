use crate::{error::*, util::hash::Hash};
use derive_more::Display;
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(fmt = "{}-{}", hash, name)]
pub struct StorePath {
  name: Name,
  hash: Hash,
}

pub type StorePathSet = BTreeSet<StorePath>;

impl StorePath {
  pub fn new<S: Into<String>>(name: S, hash: Hash) -> Self {
    Self {
      name: Name(name.into()),
      hash,
    }
  }

  pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
    let p = path.as_ref();
    Self::from_base_name(
      p.file_name()
        .ok_or_else(|| Error::BadStorePath(p.into()))?
        .to_str()
        .ok_or_else(|| Error::BadStorePath(p.into()))?,
    )
  }

  pub fn from_base_name<P: AsRef<str>>(base_name: P) -> Result<Self> {
    let p = base_name.as_ref();
    if p.len() < Hash::HASH_CHARS + 1 || p.as_bytes()[Hash::HASH_CHARS] != b'-' {
      return Err(Error::BadStorePath(p.into()));
    }
    Ok(StorePath {
      name: Name(p[Hash::HASH_CHARS + 1..].to_string()),
      hash: p[0..Hash::HASH_CHARS].parse()?,
    })
  }

  pub fn name(&self) -> &str {
    &self.name.0
  }

  pub fn hash(&self) -> &Hash {
    &self.hash
  }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
pub struct Name(String);
