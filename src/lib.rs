pub mod error;
mod nar_info;
pub mod path;
pub mod path_info;
pub mod settings;
mod state;
pub mod stats;
mod util;

use error::*;
use path::StorePath;
pub use settings::Settings;
use state::*;
use std::{
  path::Path,
  sync::{Arc, Mutex},
};

pub struct Store {
  state: Arc<Mutex<State>>,
  disk_cache: Option<NarInfoDiskCache>,
  stats: stats::Stats,
  settings: Settings,
}

impl Store {
  pub fn new() -> Result<Self> {
    Ok(Self {
      state: Default::default(),
      disk_cache: None,
      stats: Default::default(),
      settings: Settings::get()?,
    })
  }

  pub fn parse_store_path<P: AsRef<Path>>(&self, path: P) -> Result<StorePath> {
    let base = self.settings.store_path();
    let path = path.as_ref();
    if path.parent() != Some(base) {
      return Err(Error::NotInStore(path.into()));
    }
    StorePath::from_base_name(
      path
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| Error::BadStorePath(path.into()))?,
    )
  }

  pub async fn query_path_info(&self, path: &StorePath) -> Result<path_info::PathInfo> {
    let p = path.hash().to_string();
    if let Some(res) = self
      .state
      .lock()
      .map_err(|_| Error::Deadlock)?
      .info_cache
      .get(&p)
    {
      if res.is_known_now() {
        self
          .stats
          .info_read_averted
          .fetch_add(1, stats::Ordering::SeqCst);
        return res
          .value
          .clone()
          .ok_or_else(|| Error::InvalidPath(path.clone()));
      }
    }
    unimplemented!()
  }

  async fn query_path_info_uncached(&self, path: &StorePath) -> Result<path_info::PathInfo> {
    unimplemented!()
  }
}

#[test]
fn test_parse() {
  let fspath =
    "/nix/store/kqdf7siiaivgbcscfw88vrmf3bp7wzi0-rust-1.43.0-nightly-2020-02-18-e620d0f33";
  let p = Store::new().unwrap();
  let path = p.parse_store_path(fspath).unwrap();
  assert_eq!(format!("/nix/store/{}", path), fspath);
}
