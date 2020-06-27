use crate::{error::*, path::Path as StorePath, path_info::PathInfo, Store};
use db::Db;
use futures::lock::Mutex;
use std::{path::Path, sync::Arc};

mod db;

pub struct LocalStore(Mutex<Db>);

assert_impl_all!(LocalStore: Sync);

#[async_trait]
impl Store for LocalStore {
  fn store_path(&self) -> &Path {
    Path::new("/nix/store")
  }

  fn get_uri(&self) -> String {
    String::from("local")
  }

  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>> {
    if let Some(x) = self.0.lock().await.get_path_info(self, path)? {
      Ok(Some(Arc::new(x)))
    } else {
      Ok(None)
    }
  }
}

impl LocalStore {
  pub fn open(p: &Path) -> Result<Self> {
    Ok(Self(Mutex::new(Db::open(p)?)))
  }
}

#[tokio::test]
async fn test_local_store() {
  let store = LocalStore::open(Path::new(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/db.sqlite"
  )))
  .expect("no openerino");
  let path = Path::new("/nix/store/83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4/bin/nix");
  let spath = store.store_path_of(&path).expect("no parse");
  let path_info = store.get_path_info(&spath).await.expect("no good");
  assert!(path_info.is_some());
}
