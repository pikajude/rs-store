use crate::{
  error::*,
  path::StorePath,
  path_info::PathInfo,
  state::{NarInfoDiskCache, PathInfoCacheValue},
  stats::Stats,
  store::Store,
  util::mutex::*,
};
use lru::LruCache;
use std::{
  path::Path,
  sync::{atomic::Ordering, Arc, Mutex},
  time::Instant,
};

pub struct GenericStore<T> {
  store: T,
  stats: Stats,
  info_cache: Arc<Mutex<LruCache<String, PathInfoCacheValue>>>,
  disk_cache: Arc<Mutex<NarInfoDiskCache>>,
}

#[async_trait]
impl<T: Store> Store for GenericStore<T> {
  fn get_uri(&self) -> String {
    self.store.get_uri()
  }

  fn store_path(&self) -> &Path {
    self.store.store_path()
  }

  async fn query_path_info(&self, path: &StorePath) -> Result<Option<PathInfo>> {
    let hash = path.hash().to_string();

    if let Some(x) = self.info_cache.nlock()?.get(&hash) {
      if x.is_known_now() {
        self.stats.info_read_averted.fetch_add(1, Ordering::SeqCst);
        if !x.did_exist() || x.value.is_none() {
          return Err(Error::InvalidPath(self.print_path(path)));
        }
        unimplemented!()
        // return Ok(x.value.clone().unwrap());
      }
    }

    if let Some(out) = self.disk_cache.nlock()?.lookup(self.get_uri(), &hash) {
      self.stats.info_read_averted.fetch_add(1, Ordering::SeqCst);
      self.info_cache.nlock()?.put(
        hash.clone(),
        PathInfoCacheValue {
          time_point: Instant::now(),
          value: out,
        },
      );
    }

    self.store.query_path_info(path).await
  }
}
