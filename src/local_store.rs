use crate::{error::*, path::Path as StorePath, path_info::ValidPathInfo, Store};
use std::path::Path;

pub struct LocalStore;

#[async_trait]
impl Store for LocalStore {
  fn store_path(&self) -> &Path {
    Path::new("/nix/store")
  }

  fn get_uri(&self) -> String {
    String::from("local")
  }

  async fn get_path_info_uncached(&self, path: &StorePath) -> Result<ValidPathInfo> {
    todo!()
  }
}

#[tokio::test]
async fn test_local_store() {
  let path = Path::new("/nix/store/83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4/bin/nix");
  let spath = LocalStore.store_path_of(&path).expect("no parse");
  let path_info = LocalStore
    .get_path_info_uncached(&spath)
    .await
    .expect("no good");
  assert!(path_info.deriver().is_some());
}
