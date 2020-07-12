use anyhow::Result;
use std::{
  fs,
  path::{Path, PathBuf},
};

pub struct Dirs(PathBuf);

impl Dirs {
  pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
    let s = Self(root.as_ref().into());
    fs::create_dir_all(s.temproots_dir())?;
    fs::create_dir_all(s.db_dir())?;
    fs::create_dir_all(s.store_dir())?;
    Ok(s)
  }

  pub fn root(&self) -> &Path {
    &self.0
  }

  pub fn store_dir(&self) -> PathBuf {
    self.root().join("store")
  }

  pub fn state_dir(&self) -> PathBuf {
    self.root().join("var").join("nix")
  }

  pub fn db_dir(&self) -> PathBuf {
    self.state_dir().join("db")
  }

  pub fn temproots_dir(&self) -> PathBuf {
    self.state_dir().join("temproots")
  }
}
