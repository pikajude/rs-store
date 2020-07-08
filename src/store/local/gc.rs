use super::dirs::Dirs;
use anyhow::{Context as _, Result};
use futures::Future;
use libc::{c_int, flock, EINTR, EWOULDBLOCK, LOCK_EX, LOCK_NB, LOCK_SH, LOCK_UN};
use std::{
  io,
  os::unix::io::AsRawFd,
  pin::Pin,
  task::{Context, Poll},
};
use tokio::fs::{File, OpenOptions};

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

pub async fn open_gc_lock(d: &Dirs, l: LockType) -> Result<File> {
  let gc_lock = d.state_dir().join("gc.lock");
  debug!("acquiring global GC lock at `{}'", gc_lock.display());
  let f = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&gc_lock)
    .await?;
  if !f.try_lock(l)? {
    info!("waiting for the big GC lock at `{}'...", gc_lock.display());
    f.lock(l).await?;
  }
  Ok(f)
}

pub trait FsExt2 {
  fn try_lock(&self, ty: LockType) -> Result<bool>;
  fn lock(&self, ty: LockType) -> FileLock;
}

impl FsExt2 for tokio::fs::File {
  fn try_lock(&self, ty: LockType) -> Result<bool> {
    if unsafe { flock(self.as_raw_fd(), ty.flag() | LOCK_NB) } != 0 {
      let errno = nix::errno::errno();
      if errno == EWOULDBLOCK {
        return Ok(false);
      }
      if errno != EINTR {
        return Err(io::Error::from_raw_os_error(errno).into());
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
        return Poll::Ready(Err(io::Error::from_raw_os_error(errno).into()));
      }
    }
    Poll::Ready(Ok(()))
  }
}
