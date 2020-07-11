use anyhow::Result;
use futures::Future;
use libc::{c_int, flock, EINTR, EWOULDBLOCK, LOCK_EX, LOCK_NB, LOCK_SH, LOCK_UN};
use std::{
  io::Error,
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
  fn flag(self) -> c_int {
    match self {
      Self::Read => LOCK_SH,
      Self::Write => LOCK_EX,
      Self::Unlock => LOCK_UN,
    }
  }
}

pub trait FsExt2 {
  fn try_lock(&self, ty: LockType) -> Result<bool>;
  fn lock(&self, ty: LockType) -> FileLock;
}

impl FsExt2 for fs::File {
  fn try_lock(&self, ty: LockType) -> Result<bool> {
    if unsafe { flock(self.as_raw_fd(), ty.flag() | LOCK_NB) } != 0 {
      let errno = nix::errno::errno();
      if errno == EWOULDBLOCK {
        return Ok(false);
      }
      if errno != EINTR {
        return Err(Error::from_raw_os_error(errno).into());
      }
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
    if unsafe { flock(self.file.as_raw_fd(), self.ty.flag() | LOCK_NB) } != 0 {
      let errno = nix::errno::errno();
      if errno == EWOULDBLOCK {
        return Poll::Pending;
      }
      if errno != EINTR {
        return Poll::Ready(Err(Error::from_raw_os_error(errno).into()));
      }
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
    self.0.clear()
  }
}
