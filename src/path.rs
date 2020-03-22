use crate::{error::*, util::hash::Hash};
use derive_more::Display;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(fmt = "{}-{}", hash, name)]
pub struct StorePath {
  name: Name,
  hash: Hash,
}

impl StorePath {
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
