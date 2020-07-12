use crate::{
  hash::{Encoding, Hash},
  prelude::*,
};

#[derive(Debug, Error)]
pub enum Error {
  #[error(
    "hash mismatch while importing path `{}':\n  wanted: {}\n  got:    {}",
    path.display(),
    expected.encode_with_type(Encoding::Base32),
    actual.encode_with_type(Encoding::Base32)
  )]
  NarHashMismatch {
    path: PathBuf,
    expected: Hash,
    actual: Hash,
  },
  #[error("size mismatch while importing path `{}':\n  wanted: {expected}\n  got:    {actual}", path.display())]
  NarSizeMismatch {
    path: PathBuf,
    expected: usize,
    actual: usize,
  },
}
