use crate::{path::StorePath, util::hash::Hash};
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct PathInfo {
  pub path: StorePath,
  pub deriver: Option<StorePath>,
  pub nar_hash: Hash,
  pub references: BTreeSet<StorePath>,
  pub registration_time: DateTime<Utc>,
  pub nar_size: u32,
  pub id: u32,
  pub ultimate: bool,
  pub signatures: BTreeSet<String>,
  pub ca: Option<String>,
}

impl PartialEq for PathInfo {
  fn eq(&self, other: &Self) -> bool {
    self.path == other.path
      && self.nar_hash == other.nar_hash
      && self.references == other.references
  }
}

impl Eq for PathInfo {}

impl PathInfo {
  pub fn fingerprint(&self) -> String {
    unimplemented!()
  }

  pub fn is_content_addressed(&self) -> bool {
    unimplemented!()
  }
}
