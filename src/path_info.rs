use crate::{path::StorePath, util::hash::Hash};
use std::{collections::BTreeSet, time::SystemTime};

#[derive(Clone, Debug)]
pub struct PathInfo {
  path: StorePath,
  deriver: Option<StorePath>,
  nar_hash: Option<Hash>,
  references: BTreeSet<StorePath>,
  registration_time: Option<SystemTime>,
  nar_size: Option<u64>,
  id: u64,
  ultimate: bool,
  signatures: BTreeSet<String>,
  ca: Option<String>,
}

impl PartialEq for PathInfo {
  fn eq(&self, other: &Self) -> bool {
    self.path == other.path
      && self.nar_hash == other.nar_hash
      && self.references == other.references
  }
}

impl Eq for PathInfo {}

impl From<StorePath> for PathInfo {
  fn from(path: StorePath) -> Self {
    Self {
      path,
      deriver: None,
      nar_hash: None,
      references: Default::default(),
      registration_time: None,
      nar_size: None,
      id: 0,
      ultimate: false,
      signatures: Default::default(),
      ca: None,
    }
  }
}

impl PathInfo {
  pub fn fingerprint(&self) -> String {
    unimplemented!()
  }

  pub fn is_content_addressed(&self) -> bool {
    unimplemented!()
  }
}
