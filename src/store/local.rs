use crate::{
  error::*,
  path::Path as StorePath,
  path_info::{PathInfo, ValidPathInfo},
  Store,
};
use bytes::Bytes;
use db::Db;
use dirs::Dirs;
use futures::{lock::Mutex, Stream, TryStreamExt};
use gc::{FsExt2, LockType};
use std::{borrow::Cow, collections::BTreeSet, path::Path, process, sync::Arc};
use tokio::{fs, io::AsyncWriteExt};

mod db;
mod dirs;
mod gc;

pub struct LocalStore {
  dirs: Dirs,
  db: Mutex<Db>,
}

#[async_trait]
impl Store for LocalStore {
  fn store_path(&self) -> Cow<Path> {
    Cow::Borrowed(self.dirs.store_dir())
  }

  fn get_uri(&self) -> String {
    String::from("local")
  }

  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>> {
    // i think i have to destructure here because map() requires Sized
    if let Some(x) = self.db.lock().await.get_path_info(self, path)? {
      Ok(Some(Arc::new(x)))
    } else {
      Ok(None)
    }
  }

  async fn get_referrers(&self, path: &StorePath) -> Result<BTreeSet<StorePath>> {
    self.db.lock().await.get_referrers(self, path)
  }

  async fn add_temp_root(&self, path: &StorePath) -> Result<()> {
    let file = self
      .dirs
      .state_dir()
      .join("temproots")
      .join(process::id().to_string());
    let mut temp_file = loop {
      let all_gc_roots = gc::open_gc_lock(&self.dirs, LockType::Read).await?;
      debug!("{:?}", fs::remove_file(&file).await);
      let temproots_file = fs::OpenOptions::new()
        .create_new(true)
        .open(&file)
        .await
        .somewhere(&file)?;
      drop(all_gc_roots);
      debug!("acquiring read lock on `{}'", self.print_store_path(path));
      temproots_file.lock(LockType::Read).await?;
      if temproots_file.metadata().await.nowhere()?.len() == 0 {
        break temproots_file;
      }
    };
    debug!("acquiring write lock on `{}'", file.display());
    temp_file.lock(LockType::Write).await?;
    temp_file
      .write(self.print_store_path(path).as_bytes())
      .await
      .nowhere()?;
    temp_file.lock(LockType::Read).await?;
    Ok(())
  }

  async fn add_to_store<S: Stream<Item = Result<Bytes>> + Send + Unpin>(
    &self,
    info: &ValidPathInfo,
    mut source: S,
  ) -> Result<()> {
    self.add_temp_root(&info.store_path).await?;
    while let Some(bytes) = source.try_next().await? {
      eprintln!("{:?}", bytes);
    }
    Ok(())
    // todo!()
  }
}

impl LocalStore {
  pub fn open(root: &Path) -> Result<Self> {
    let dirs = Dirs::new(root)?;
    Ok(Self {
      db: Mutex::new(Db::open(&dirs.db_dir().join("db.sqlite"))?),
      dirs,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::hash::{Hash, HashType};
  use std::time::SystemTime;

  #[test]
  fn test_local_store() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
      let temp = tempfile::tempdir()?;
      let store = LocalStore::open(temp.as_ref())?;
      store
        .add_to_store(
          &ValidPathInfo {
            store_path: store.parse_store_path(
              &temp
                .as_ref()
                .join("store/83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4"),
            )?,
            deriver: None,
            nar_hash: Hash::hash_bytes(b"foobar", HashType::SHA256),
            references: Default::default(),
            registration_time: SystemTime::now(),
            nar_size: None,
            id: 0,
            signatures: Default::default(),
            content_addressed: None,
          },
          crate::util::stream_file(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).await?,
        )
        .await?;
      let path = temp
        .as_ref()
        .join("store/83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4/bin/nix");
      let spath = store.store_path_of(&path)?;
      let path_info = store.get_path_info(&spath).await?;
      assert!(path_info.is_some());

      let refs = store.get_referrers(&spath).await?;
      assert!(!refs.is_empty());

      Ok(())
    })
  }
}
