use crate::error::*;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

pub struct Dirs(PathBuf);

impl Dirs {
  pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
    let s = Self(root.as_ref().into());
    std::fs::create_dir_all(s.state_dir()).somewhere(s.state_dir())?;
    std::fs::create_dir_all(s.db_dir()).somewhere(s.db_dir())?;
    Ok(s)
  }

  pub fn root(&self) -> &Path {
    &self.0
  }

  pub fn store_dir(&self) -> &Path {
    static DIR: OnceCell<PathBuf> = OnceCell::new();
    DIR.get_or_init(|| self.root().join("store"))
  }

  pub fn state_dir(&self) -> &Path {
    static DIR: OnceCell<PathBuf> = OnceCell::new();
    DIR.get_or_init(|| self.root().join("var").join("nix"))
  }

  pub fn db_dir(&self) -> &Path {
    static DIR: OnceCell<PathBuf> = OnceCell::new();
    DIR.get_or_init(|| self.state_dir().join("db"))
  }
}
