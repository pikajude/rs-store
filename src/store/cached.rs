use crate::{error::*, path::Path as StorePath, path_info::PathInfo, Store};
use futures::lock::Mutex;
use lru_cache::LruCache;
use std::{
  path::{Path, PathBuf},
  sync::Arc,
};

/// Wraps a Store implementation with an in-memory path info cache.
pub struct CachedStore<S> {
  store: S,
  cache: Mutex<LruCache<PathBuf, Option<Arc<dyn PathInfo>>>>,
}

impl<S> CachedStore<S> {
  pub fn new(store: S) -> Self {
    Self {
      store,
      cache: Mutex::new(LruCache::new(8192)),
    }
  }
}

#[async_trait]
impl<S: Store + Send + Sync> Store for CachedStore<S> {
  fn store_path(&self) -> &Path {
    self.store.store_path()
  }

  fn get_uri(&self) -> String {
    self.store.get_uri()
  }

  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>> {
    let mut cache = self.cache.lock().await;
    let path_key = self.print_store_path(path);
    if let Some(x) = cache.get_mut(Path::new(path_key.as_str())) {
      Ok(x.clone())
    } else {
      let new_data = self.store.get_path_info(path).await?;
      cache.insert(path_key.into(), new_data.clone());
      Ok(new_data)
    }
  }
}
