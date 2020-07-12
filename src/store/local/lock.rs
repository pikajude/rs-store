use crate::prelude::*;
use anyhow::Result;
use futures::Future;
use nix::{
  errno::EWOULDBLOCK,
  fcntl::{self, FlockArg},
};
use std::{
  os::unix::io::AsRawFd,
  path::PathBuf,
  pin::Pin,
  task::{Context, Poll},
};
use tokio::fs::{self, File};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LockType {
  Read,
  Write,
  Unlock,
}

impl LockType {
  fn flag(self) -> FlockArg {
    use FlockArg::*;
    match self {
      Self::Read => LockSharedNonblock,
      Self::Write => LockExclusiveNonblock,
      Self::Unlock => UnlockNonblock,
    }
  }
}

pub trait FsExt2 {
  fn try_lock(&self, ty: LockType) -> Result<bool>;
  fn lock(&self, ty: LockType) -> FileLock;
}

impl FsExt2 for fs::File {
  fn try_lock(&self, ty: LockType) -> Result<bool> {
    if let Err(e) = fcntl::flock(self.as_raw_fd(), ty.flag()) {
      if e.as_errno() == Some(EWOULDBLOCK) {
        return Ok(false);
      }
      bail!(e);
    }
    Ok(true)
  }

  fn lock(&self, ty: LockType) -> FileLock {
    FileLock { file: self, ty }
  }
}

pub struct FileLock<'a> {
  file: &'a File,
  ty: LockType,
}

impl<'a> Future for FileLock<'a> {
  type Output = Result<()>;

  fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
    if let Err(e) = fcntl::flock(self.file.as_raw_fd(), self.ty.flag()) {
      if e.as_errno() == Some(EWOULDBLOCK) {
        return Poll::Pending;
      }
      return Poll::Ready(Err(e.into()));
    }
    Poll::Ready(Ok(()))
  }
}

#[derive(Default, Debug)]
pub struct PathLocks(Vec<File>);

impl PathLocks {
  pub fn new() -> Self {
    Self::default()
  }

  pub async fn lock<I: IntoIterator<Item = PathBuf>>(
    &mut self,
    paths: I,
    wait: bool,
    message: Option<&'static str>,
  ) -> Result<bool> {
    assert!(self.0.is_empty());
    for path in paths {
      let lock_path = match path.extension() {
        Some(e) => path.with_extension({
          let mut e = e.to_os_string();
          e.push(".lock");
          e
        }),
        None => path.with_extension(".lock"),
      };
      loop {
        let lockfile = fs::File::create(&lock_path).await?;
        if !lockfile.try_lock(LockType::Write)? {
          if wait {
            if let Some(m) = message {
              error!("{}", m);
            }
            lockfile.lock(LockType::Write).await?;
          } else {
            self.unlock();
            return Ok(false);
          }
        }
        debug!("lock acquired on `{}'", lock_path.display());
        let meta = fs::metadata(&lock_path).await?;
        if meta.len() != 0 {
          debug!("lock file `{}' has become stale", lock_path.display());
        } else {
          self.0.push(lockfile);
          break;
        }
      }
    }
    Ok(true)
  }

  pub fn unlock(&mut self) {
    for it in self.0.drain(..) {
      let _ = it.try_lock(LockType::Unlock);
    }
  }
}
