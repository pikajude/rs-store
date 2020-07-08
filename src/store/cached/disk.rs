use crate::{error::*, path_info::PathInfo};
use std::{borrow::Borrow, collections::HashMap, hash::Hash, path::PathBuf, sync::Arc};

pub struct DiskCache {
  entries: HashMap<String, SingleSourceCache>,
  want_mass_query: bool,
  priority: usize,
}

pub enum CacheEntry {
  Valid(Arc<dyn PathInfo>),
  Invalid,
  Unknown,
}

struct SingleSourceCache(HashMap<PathBuf, CacheEntry>);

impl DiskCache {
  pub fn new() -> Self {
    Self {
      entries: HashMap::new(),
      want_mass_query: false,
      priority: 0,
    }
  }

  pub fn has_cache<Q>(&self, s: Q) -> bool
  where
    String: Borrow<Q>,
    Q: Hash + Eq,
  {
    self.entries.contains_key(&s)
  }

  pub async fn lookup_nar(&self, uri: &str, hash_part: &str) -> Result<CacheEntry> {
    todo!()
  }
}
