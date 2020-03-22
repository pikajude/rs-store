use crate::path_info::PathInfo;
use lru::LruCache;
use std::time::Instant;

mod disk_cache;
pub use disk_cache::*;

pub struct State {
  pub info_cache: LruCache<String, PathInfoCacheValue>,
}

impl Default for State {
  fn default() -> Self {
    Self {
      info_cache: LruCache::unbounded(),
    }
  }
}

pub struct PathInfoCacheValue {
  pub time_point: Instant,
  pub value: Option<PathInfo>,
}

impl PathInfoCacheValue {
  pub fn is_known_now(&self) -> bool {
    unimplemented!()
  }

  pub fn did_exist(&self) -> bool {
    self.value.is_some()
  }
}
