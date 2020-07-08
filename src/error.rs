use std::{
  convert::Infallible, error::Error as StdError, io, num::TryFromIntError, path::PathBuf, result,
  sync::PoisonError,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  // store path parsing errors
  #[error("path is not in the nix store: `{}'", _0.display())]
  NotInStore(PathBuf),
  #[error("invalid filepath for store: `{}'", _0.display())]
  InvalidFilepath(PathBuf),
  #[error("invalid store path name: {0:?}")]
  InvalidStorePathName(String),
  #[error("invalid base32 data")]
  InvalidBase32,

  // IO stuff
  #[error("I/O error: {0}")]
  Io(io::Error),
  #[error("I/O error at {1}: {0}")]
  IoAt(io::Error, PathBuf),

  // hash manipulation
  #[error("incorrect length `{0}' for hash")]
  WrongHashLen(usize),
  #[error("attempt to parse untyped hash `{0}'")]
  UntypedHash(String),
  #[error("unknown hash type `{0}'")]
  UnknownHashType(String),
  #[error("decoding error: {0:?}")]
  ConvertError(binascii::ConvertError),

  // sql errors
  #[error("{0}")]
  Sql(#[from] rusqlite::Error),
  #[error("{0}")]
  NumericConversion(#[from] TryFromIntError),

  // misc
  #[error("deadlock")]
  Deadlock,

  #[error("{0}")]
  Other(Box<dyn StdError + Send>),
}

impl Error {
  pub fn other<E: StdError + Send + 'static>(err: E) -> Self {
    Self::Other(Box::new(err))
  }
}

impl From<Infallible> for Error {
  fn from(_: Infallible) -> Self {
    unreachable!()
  }
}

impl<G> From<PoisonError<G>> for Error {
  fn from(_: PoisonError<G>) -> Self {
    Self::Deadlock
  }
}

impl From<binascii::ConvertError> for Error {
  fn from(c: binascii::ConvertError) -> Self {
    Self::ConvertError(c)
  }
}

pub type Result<T> = result::Result<T, Error>;

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
