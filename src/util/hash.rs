use crate::util::base32;
use std::{cmp::Ordering, fmt, str::FromStr};

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum Error {
  #[error("Given hash is too long: got {given} bytes, expected at most {limit}")]
  InvalidLength { given: usize, limit: usize },
  #[error("Invalid length {len} for hash type {ty}")]
  IncorrectLength { ty: HashType, len: usize },
  #[error("Invalid character `{0}` in base32 hash")]
  InvalidChar(char),
  #[error("No hash type provided, and it cannot be inferred from the input")]
  TypeNotProvided,
  #[error("Hash type `{0}` not recognized")]
  UnknownHashType(String),
  #[error("{0}")]
  Base16(#[from] base16::DecodeError),
  #[error("{0}")]
  Base32(#[from] base32::Error),
  #[error("{0}")]
  Base64(#[from] base64::DecodeError),
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy, derive_more::Display)]
pub enum HashType {
  Md5,
  Sha1,
  Sha256,
  Sha512,
}

impl FromStr for HashType {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(match s {
      "md5" => Self::Md5,
      "sha1" => Self::Sha1,
      "sha256" => Self::Sha256,
      "sha512" => Self::Sha512,
      x => return Err(Error::UnknownHashType(x.into())),
    })
  }
}

impl HashType {
  pub fn len(self) -> usize {
    match self {
      Self::Md5 => 16,
      Self::Sha1 => 20,
      Self::Sha256 => 32,
      Self::Sha512 => 64,
    }
  }

  pub fn base16_len(self) -> usize {
    self.len() * 2
  }

  pub fn base32_len(self) -> usize {
    (self.len() * 8 - 1) / 5 + 1
  }

  pub fn base64_len(self) -> usize {
    ((4 * self.len() / 3) + 3) & !3
  }
}

/// A signature computed from some data somewhere. This can be an MD5, SHA1,
/// SHA2, or SHA512 hash.
///
/// In serialized form, hashes are almost always prefixed with the hash type. A
/// notable exception is the hash found in store paths like
/// `/nix/store/vaxhh4bg6smwbrid99g62x54y2hk1ph3-rustc-1.41.0`, which is a
/// sha256 hash of the derivation's contents truncated to 20 bytes and base32
/// encoded.
///
/// Examples:
///
/// * `sha256:1yzhjn8rsvjjsfycdc993ms6jy2j5jh7x3r2ax6g02z5n0anvnbx`
/// * `sha512:+CvDC7ZttU/sSt9rFjix/P05iS43qHCOOGzcr3Ry99bXG7VX953+vFyEuph/
///   tfqoYu8dttBkE86JSKBO2OzcxA==`
/// * `vaxhh4bg6smwbrid99g62x54y2hk1ph3`
pub struct Hash {
  data: [u8; 64],
  len: usize,
}

impl Hash {
  pub(crate) fn from_data(in_: &[u8]) -> Self {
    assert!(in_.len() <= 64);
    let mut data = [0; 64];
    {
      let (l, _) = data.split_at_mut(in_.len());
      l.copy_from_slice(in_);
    }
    Self {
      data,
      len: in_.len(),
    }
  }

  /// Try to parse an unprefixed string as a hash of a certain type. As in other
  /// places, `ty` is only used to compute the expected hash length.
  ///
  /// To parse a string which includes hash type information, use the `FromStr`
  /// instance.
  pub fn parse_typed(s: &str, ty: HashType) -> Result<Self, Error> {
    unimplemented!()
  }

  pub fn base16(&self, with_type: bool) -> String {
    base16::encode_lower(self.as_ref())
  }

  pub fn base32(&self, with_type: bool) -> String {
    base32::encode(self.as_ref())
  }

  pub fn base64(&self, with_type: bool) -> String {
    base64::encode(&self)
  }

  pub fn sri(&self) -> String {
    unimplemented!()
  }

  /// Truncate `self` to a given length by XOR-ing the trailing bytes.
  ///
  /// No-op if `len >= self.len`.
  pub fn truncate(&self, len: usize) -> Self {
    if len >= self.len {
      return self.clone();
    }
    unimplemented!()
  }
}

impl AsRef<[u8]> for Hash {
  fn as_ref(&self) -> &[u8] {
    &self.data[0..self.len]
  }
}

impl AsMut<[u8]> for Hash {
  fn as_mut(&mut self) -> &mut [u8] {
    &mut self.data[0..self.len]
  }
}

impl Clone for Hash {
  fn clone(&self) -> Self {
    let mut this = Self {
      data: [0; 64],
      len: self.len,
    };
    this.data.copy_from_slice(&self.data);
    this
  }
}

impl PartialEq for Hash {
  fn eq(&self, other: &Hash) -> bool {
    self.len == other.len && &self.data[..] == &other.data[..]
  }
}

impl Eq for Hash {}

impl PartialOrd for Hash {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    let cmp = self.len.partial_cmp(&other.len)?;
    if cmp == Ordering::Equal {
      self.data[..].partial_cmp(&other.data[..])
    } else {
      Some(cmp)
    }
  }
}

impl Ord for Hash {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let cmp = self.len.cmp(&other.len);
    if cmp == Ordering::Equal {
      self.data[..].cmp(&other.data[..])
    } else {
      cmp
    }
  }
}

impl fmt::Debug for Hash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Hash").field(&self.as_ref()).finish()
  }
}

impl FromStr for Hash {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if let Some(ix) = s.find(':') {
      let ty = s[..ix].parse::<HashType>()?;
      let rest = &s[ix + 1..];
      // input is base16
      if rest.len() == ty.base16_len() {
        Ok(Self::from_data(&base16::decode(rest)?))
      } else if rest.len() == ty.base32_len() {
        Ok(Self::from_data(&base32::decode(rest)?))
      } else if rest.len() == ty.base64_len() {
        Ok(Self::from_data(&base64::decode(rest)?))
      } else {
        Err(Error::IncorrectLength {
          ty,
          len: rest.len(),
        })
      }
    } else if let Some(ix) = s.find('-') {
      unimplemented!("SRI")
    } else {
      Err(Error::TypeNotProvided)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_ref() {
    let h = Hash::from_data(&[1, 2, 3]);
    assert_eq!(h.as_ref(), &[1, 2, 3]);
  }

  #[test]
  fn test_trunc() {
    let h = Hash::from_data(&[1, 2, 3, 4, 5]);
    assert_eq!(h.truncate(2).as_ref(), &[4, 5]);
  }
}
