use crate::error::*;
use std::{
  path::Path,
  sync::{Mutex, MutexGuard},
};

pub trait MutexExt<T> {
  fn nlock(&self) -> Result<MutexGuard<T>>;
}

impl<T> MutexExt<T> for Mutex<T> {
  fn nlock(&self) -> Result<MutexGuard<T>> {
    self.lock().map_err(|_| Error::Deadlock)
  }
}

pub trait IoResultExt<T> {
  fn on_path<P: AsRef<Path>>(self, path: P) -> Result<T>;
  fn no_path(self) -> Result<T>;
}

impl<T> IoResultExt<T> for std::io::Result<T> {
  fn on_path<P: AsRef<Path>>(self, path: P) -> Result<T> {
    self.map_err(|io| Error::Io {
      error: io,
      path: Some(path.as_ref().into()),
    })
  }

  fn no_path(self) -> Result<T> {
    self.map_err(|io| Error::Io {
      error: io,
      path: None,
    })
  }
}
