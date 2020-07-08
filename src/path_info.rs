use crate::{hash::Hash, path::Path};
use std::{collections::BTreeSet, time::SystemTime};

pub trait PathInfo: Send + Sync {
  fn store_path(&self) -> &Path;
}

#[derive(Clone, Debug)]
pub struct ValidPathInfo {
  pub store_path: Path,
  pub deriver: Option<Path>,
  pub nar_hash: Hash,
  pub references: BTreeSet<Path>,
  pub registration_time: SystemTime,
  pub nar_size: Option<u64>,
  pub id: u64,
  pub signatures: BTreeSet<String>,
  pub content_addressed: Option<String>,
}

impl PartialEq for ValidPathInfo {
  fn eq(&self, other: &Self) -> bool {
    self.store_path == other.store_path
      && self.nar_hash == other.nar_hash
      && self.references == other.references
  }
}

impl Eq for ValidPathInfo {}

impl PathInfo for ValidPathInfo {
  fn store_path(&self) -> &Path {
    &self.store_path
  }
}
