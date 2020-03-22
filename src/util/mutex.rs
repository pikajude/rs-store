use crate::error::*;
use std::sync::{Mutex, MutexGuard};

pub trait NixMutex<T> {
  fn nlock(&self) -> Result<MutexGuard<T>>;
}

impl<T> NixMutex<T> for Mutex<T> {
  fn nlock(&self) -> Result<MutexGuard<T>> {
    self.lock().map_err(|_| Error::Deadlock)
  }
}
