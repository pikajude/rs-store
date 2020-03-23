#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Invalid path `{0}`")]
  InvalidPath(String),
  #[error("Path `{0}` is not a store path")]
  BadStorePath(std::path::PathBuf),
  #[error("Path `{0}` is not in the Nix store")]
  NotInStore(std::path::PathBuf),
  #[error(".narinfo file is corrupt")]
  BadNarInfo,
  #[error("{0}")]
  Base32(#[from] crate::util::hash::Error),
  #[error("Store path name is empty")]
  StorePathNameEmpty,
  #[error("Store path name is longer than 211 characters")]
  StorePathNameTooLong,
  #[error("Store path name contains forbidden characters")]
  BadStorePathName,
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
  #[error("DB error: {0}")]
  Db(#[from] rusqlite::Error),
  #[error("Deadlock when attempting to read state")]
  Deadlock,
  #[error("Unsupported operation: {0}")]
  Unsupported(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
