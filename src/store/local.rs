use crate::{path::StorePath, settings::Settings, store::Store};
use std::path::{Path, PathBuf};

pub struct LocalStore {
  settings: Settings,
}

// impl Store for LocalStore {
//   fn get_uri(&self) -> Option<String> {
//     Some("local".into())
//   }
//
//   fn store_path(&self) -> &Path {
//     self.settings.store_path()
//   }
// }
