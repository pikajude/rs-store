use crate::{
  error::*,
  path::StorePath,
  path_info::PathInfo,
  state::{NarInfoDiskCache, PathInfoCacheValue},
  stats::Stats,
  store::Store,
  util::ext::*,
};
use lru::LruCache;
use std::{
  path::Path,
  sync::{atomic::Ordering, Arc, Mutex},
  time::Instant,
};

pub struct NixStore<T> {
  store: T,
  stats: Stats,
  info_cache: Arc<Mutex<LruCache<String, PathInfoCacheValue>>>,
  disk_cache: Option<Arc<Mutex<NarInfoDiskCache>>>,
}

impl<T> NixStore<T> {
  /// Wrap a store impl. Unless the backing store supports disk caching, this
  /// method is guaranted never to return `Err`.
  pub fn new(store: T) -> Result<Self> {
    Ok(Self {
      store,
      stats: Stats::default(),
      info_cache: Arc::new(Mutex::new(LruCache::unbounded())),
      disk_cache: Some(Arc::new(Mutex::new(NarInfoDiskCache::new()?))),
    })
  }

  fn with_cache<F: FnOnce(&mut NarInfoDiskCache) -> Result<()>>(&self, op: F) -> Result<()> {
    if let Some(c) = self.disk_cache.as_ref() {
      op(&mut *c.nlock()?)?;
    }
    Ok(())
  }
}

impl<T: Store> NixStore<T> {
  pub async fn query_path_info(&self, path: &StorePath) -> Result<PathInfo> {
    let hash = path.hash().base32(false);

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

    self.with_cache(|dc| {
      if let Some(out) = dc.lookup(self.get_uri(), &hash) {
        self.stats.info_read_averted.fetch_add(1, Ordering::SeqCst);
        self.info_cache.nlock()?.put(
          hash.clone(),
          PathInfoCacheValue {
            time_point: Instant::now(),
            value: out,
          },
        );
      }
      Ok(())
    })?;

    let result = self.store.query_path_info_uncached(path).await?;

    self.with_cache(|dc| {
      dc.upsert(self.get_uri(), &hash, result.clone());
      Ok(())
    })?;

    self.info_cache.nlock()?.put(
      hash.clone(),
      PathInfoCacheValue {
        time_point: Instant::now(),
        value: None,
      },
    );

    if let Some(r) = result {
      Ok(r)
    } else {
      Err(Error::InvalidPath(path.to_string()))
    }
  }
}

impl NixStore<Box<dyn Store>> {
  /// Try to downcast `self` into a store of a specific type. Avoid this method
  /// if possible.
  pub fn downcast<T: Store>(self) -> std::result::Result<NixStore<T>, Self> {
    unimplemented!()
  }
}

#[async_trait]
impl<T: Store> Store for NixStore<T> {
  fn get_uri(&self) -> String {
    self.store.get_uri()
  }

  fn store_path(&self) -> &Path {
    self.store.store_path()
  }

  async fn query_path_info_uncached(&self, path: &StorePath) -> Result<Option<PathInfo>> {
    self.store.query_path_info_uncached(path).await
  }
}
