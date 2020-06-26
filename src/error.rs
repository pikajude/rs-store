use std::{error::Error as StdError, io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("path is not in the nix store: `{}'", _0.display())]
  NotInStore(PathBuf),
  #[error("invalid store path: `{}'", _0.display())]
  InvalidStorePath(PathBuf),
  #[error("invalid store path name: {0:?}")]
  InvalidStorePathName(String),
  #[error("invalid base32 data")]
  InvalidBase32,
  #[error("I/O error: {0}")]
  Io(io::Error),
  #[error("I/O error at {1}: {0}")]
  IoAt(io::Error, PathBuf),
  #[error("incorrect length `{0}` for store path hash")]
  HashSize(usize),
  #[error("{0}")]
  Other(Box<dyn StdError + Send>),
}

impl Error {
  pub fn other<E: StdError + Send + 'static>(err: E) -> Self {
    Self::Other(Box::new(err))
  }
}

impl From<!> for Error {
  fn from(_: !) -> Self {
    unreachable!()
  }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait IoExt<T> {
  fn somewhere<P: Into<PathBuf>>(self, at: P) -> Result<T>;
  fn nowhere(self) -> Result<T>;
}

impl<T> IoExt<T> for io::Result<T> {
  fn somewhere<P: Into<PathBuf>>(self, at: P) -> Result<T> {
    self.map_err(|e| Error::IoAt(e, at.into()))
  }

  fn nowhere(self) -> Result<T> {
    self.map_err(Error::Io)
  }
}
