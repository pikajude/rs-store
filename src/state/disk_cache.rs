use crate::{error::*, nar_info::NarInfo, path_info::PathInfo};
use rusqlite::{Connection, Statement};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct NarInfoDiskCache {
  db: Connection,
  caches: HashMap<String, Cache>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Cache {
  id: usize,
  store_dir: PathBuf,
  want_mass_query: bool,
  priority: usize,
}

pub type Outcome = Option<NarInfo>;

impl NarInfoDiskCache {
  pub fn new() -> Result<Self> {
    unimplemented!()
  }

  pub fn exists<P: AsRef<str>>(&self, uri: P) -> bool {
    unimplemented!()
  }

  pub fn lookup<P: AsRef<str>, Q: AsRef<str>>(&self, uri: P, hash_part: Q) -> Option<Outcome> {
    unimplemented!()
  }

  pub fn upsert<P: AsRef<str>, Q: AsRef<str>>(&self, uri: P, hash_part: Q, info: Option<PathInfo>) {
  }
}
