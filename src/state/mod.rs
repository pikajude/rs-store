use crate::{nar_info::NarInfo, path_info::PathInfo};
use lru::LruCache;
use std::time::Instant;

mod disk_cache;
pub use disk_cache::*;

pub struct PathInfoCacheValue {
  pub time_point: Instant,
  pub value: Option<NarInfo>,
}

impl PathInfoCacheValue {
  pub fn is_known_now(&self) -> bool {
    unimplemented!()
  }

  pub fn did_exist(&self) -> bool {
    self.value.is_some()
  }
}
