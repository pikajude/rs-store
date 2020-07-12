use crate::{path_info::PathInfo, prelude::*, sqlite::Sqlite};
use futures::lock::Mutex;
use rusqlite::Connection;
use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  sync::Arc,
};
use tokio::fs;

pub struct DiskCache {
  db: Mutex<Sqlite>,
  want_mass_query: bool,
  priority: usize,
}

#[derive(Clone)]
pub enum CacheEntry {
  Valid(Arc<dyn PathInfo>),
  Invalid,
  Unknown,
}

impl DiskCache {
  pub async fn open() -> Result<Self> {
    let cache = dirs::cache_dir().ok_or_else(|| anyhow!("unable to open a cache dir"))?;
    fs::create_dir_all(cache.join("nix")).await?;
    let db = Sqlite::open(cache.join("nix").join("binary-cache-v6.sqlite"))?;
    db.set_is_cache().await?;
    Ok(Self {
      db: Mutex::new(db),
      want_mass_query: false,
      priority: 0,
    })
  }

  // pub fn has_cache(&self, uri: &str) -> bool {
  //   self.entries.contains_key(uri)
  // }

  pub async fn lookup_nar(&self, uri: &str, hash_part: &str) -> Result<CacheEntry> {
    todo!()
  }

  pub fn insert(&self, uri: &str, hash_part: &str, entry: Option<Arc<dyn PathInfo>>) {
    todo!()
  }
}
