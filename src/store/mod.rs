mod generic;
pub mod local;
pub use generic::NixStore;

use crate::{
  error::*,
  path::{StorePath, StorePathSet},
  path_info::PathInfo,
  state::{NarInfoDiskCache, PathInfoCacheValue},
  util::{ext::*, hash::Hash, sha::sha256},
};
use async_trait::async_trait;
use local::LocalStore;
use lru::LruCache;
use std::{
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

/// Open a store. The store type will be inferred based on the given URI.
pub async fn open<U: AsRef<str>>(uri: U) -> Result<NixStore<Box<dyn Store>>> {
  unimplemented!()
}

pub async fn open_local() -> Result<NixStore<LocalStore>> {
  Ok(
    open("local")
      .await?
      .downcast()
      .unwrap_or_else(|_| unreachable!()),
  )
}

#[async_trait]
pub trait Store: Send + Sync {
  fn get_uri(&self) -> String;
  fn store_path(&self) -> &Path;
  fn parse_path(&self, path: &Path) -> Result<StorePath> {
    StorePath::from_path(path)
  }
  fn print_path(&self, path: &StorePath) -> String {
    format!("{}/{}", self.store_path().display(), path)
  }
  fn is_in_store(&self, path: &Path) -> bool {
    path.strip_prefix(self.store_path()).is_ok()
  }
  fn is_store_path(&self, path: &Path) -> bool {
    self.parse_path(path).is_ok()
  }
  fn to_store_path(&self, path: &Path) -> Result<PathBuf> {
    let rest = path
      .strip_prefix(self.store_path())
      .map_err(|_| Error::NotInStore(path.into()))?;
    Ok(self.store_path().join(rest.components().next().unwrap()))
  }
  fn follow_links_to_store(&self, path: &Path) -> Result<PathBuf> {
    let mut path: PathBuf = path.into();
    while !self.is_in_store(&path) {
      path = path.read_link().on_path(&path)?;
    }
    Ok(path)
  }
  fn follow_links_to_store_path(&self, path: &Path) -> Result<StorePath> {
    StorePath::from_path(self.follow_links_to_store(path)?)
  }
  fn mk_store_path(&self, output_type: &str, hash: &Hash, name: &str) -> Result<StorePath> {
    let s = format!(
      "{}:{}:{}:{}",
      output_type,
      hash.base16(false),
      self.store_path().display(),
      name
    );
    let h = sha256(s.as_bytes()).truncate(20);
    Ok(StorePath::new(name, h))
  }
  fn mk_output_path(&self, id: &str, hash: &Hash, name: &str) -> Result<StorePath> {
    self.mk_store_path(&format!("output:{}", id), hash, name)
  }
  fn mk_fixed_output_path(
    &self,
    recursive: bool,
    hash: &Hash,
    name: &str,
    references: &StorePathSet,
    has_self_reference: bool,
  ) -> Result<StorePath> {
    unimplemented!()
  }
  fn mk_text_path(&self, name: &str, hash: &Hash, references: &StorePathSet) -> Result<StorePath> {
    unimplemented!()
  }
  fn query_valid_paths(&self, paths: &StorePathSet, substitute: bool) -> Result<StorePathSet> {
    unimplemented!()
  }
  fn query_all_valid_paths(&self) -> Result<StorePathSet> {
    Err(Error::Unsupported("query_all_valid_paths"))
  }
  async fn query_path_info_uncached(&self, path: &StorePath) -> Result<Option<PathInfo>>;
}

#[test]
fn test_parse() {
  let fspath =
    "/nix/store/kqdf7siiaivgbcscfw88vrmf3bp7wzi0-rust-1.43.0-nightly-2020-02-18-e620d0f33";
  // let p = Store::new().unwrap();
  // let path = p.parse_store_path(fspath).unwrap();
  // assert_eq!(format!("/nix/store/{}", path), fspath);
}
