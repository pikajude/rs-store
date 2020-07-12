use super::ByteStream;
use crate::{
  archive::PathFilter,
  path::Path as StorePath,
  path_info::{PathInfo, ValidPathInfo},
  Store,
};
use anyhow::Result;
use disk::{CacheEntry, DiskCache};
use futures::lock::Mutex;
use lru_cache::LruCache;
use std::{
  borrow::Cow,
  collections::BTreeSet,
  path::{Path, PathBuf},
  sync::Arc,
};

mod disk;

/// Wraps a Store implementation with a path info cache.
pub struct Cached<S> {
  store: S,
  cache: Mutex<LruCache<PathBuf, Option<Arc<dyn PathInfo>>>>,
  disk_cache: Option<DiskCache>,
}

impl<S> Cached<S> {
  pub fn new(store: S, use_disk_cache: bool) -> Self {
    Self {
      store,
      cache: Mutex::new(LruCache::new(8192)),
      disk_cache: if use_disk_cache {
        Some(DiskCache::new())
      } else {
        None
      },
    }
  }
}

#[async_trait]
impl<S: Store> Store for Cached<S> {
  fn store_path(&self) -> Cow<Path> {
    self.store.store_path()
  }

  fn get_uri(&self) -> String {
    self.store.get_uri()
  }

  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>> {
    let mut cache = self.cache.lock().await;
    let path_key = self.print_store_path(path);

    // two layers of options, None means cache miss, Some(None) means nonexistent
    // path
    if let Some(x) = cache.get_mut(Path::new(path_key.as_str())) {
      return Ok(x.clone());
    }

    if let Some(dc) = &self.disk_cache {
      match dc.lookup_nar(&self.get_uri(), "").await? {
        CacheEntry::Valid(x) => {
          cache.insert(path_key.into(), Some(x.clone()));
          return Ok(Some(x));
        }
        CacheEntry::Invalid => {
          cache.insert(path_key.into(), None);
          return Ok(None);
        }
        CacheEntry::Unknown => {}
      }
    }

    let new_data = self.store.get_path_info(path).await?;
    cache.insert(path_key.into(), new_data.clone());
    Ok(new_data)
  }

  async fn get_referrers(&self, path: &StorePath) -> Result<BTreeSet<StorePath>> {
    self.store.get_referrers(path).await
  }

  async fn add_temp_root(&self, path: &StorePath) -> Result<()> {
    self.store.add_temp_root(path).await
  }

  async fn add_nar_to_store<I: ByteStream + Send + Unpin>(
    &self,
    info: &ValidPathInfo,
    source: I,
  ) -> Result<()> {
    self.store.add_nar_to_store(info, source).await
  }

  async fn add_path_to_store(
    &self,
    name: &str,
    path: &Path,
    algo: crate::hash::HashType,
    filter: PathFilter,
    repair: bool,
  ) -> Result<StorePath> {
    self
      .store
      .add_path_to_store(name, path, algo, filter, repair)
      .await
  }
}
