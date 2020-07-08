use crate::util;
use anyhow::Result;
use crypto::digest::Digest;
use derive_more::Display;
use std::{
  borrow::Cow,
  fmt::{self, Debug},
  hash::Hasher,
  path::Path,
  str::FromStr,
};
use tokio::io::{AsyncRead, AsyncReadExt};

mod context;
mod sink;

pub use context::Context;
pub use sink::HashSink as Sink;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("incorrect length `{0}' for hash")]
  WrongHashLen(usize),
  #[error("attempt to parse untyped hash `{0}'")]
  UntypedHash(String),
  #[error("unknown hash type `{0}'")]
  UnknownHashType(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
pub enum HashType {
  #[display(fmt = "md5")]
  MD5,
  #[display(fmt = "sha1")]
  SHA1,
  #[display(fmt = "sha256")]
  SHA256,
  #[display(fmt = "sha512")]
  SHA512,
}

impl HashType {
  fn size(self) -> usize {
    match self {
      Self::MD5 => 16,
      Self::SHA1 => 20,
      Self::SHA256 => 32,
      Self::SHA512 => 64,
    }
  }
}

impl FromStr for HashType {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(match s {
      "md5" => Self::MD5,
      "sha1" => Self::SHA1,
      "sha256" => Self::SHA256,
      "sha512" => Self::SHA512,
      x => return Err(Error::UnknownHashType(x.into())),
    })
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Encoding {
  Base64,
  Base32,
  Base16,
  SRI,
}

#[derive(Clone)]
pub struct Hash {
  data: [u8; 64],
  len: usize,
  ty: HashType,
}

impl Hash {
  fn len_base16(&self) -> usize {
    len_base16(self.len)
  }

  fn len_base32(&self) -> usize {
    len_base32(self.len)
  }

  fn len_base64(&self) -> usize {
    len_base64(self.len)
  }

  /// Size in bytes.
  pub fn size(&self) -> usize {
    self.len
  }

  /// Which algorithm produced this hash. See [`HashType`]
  pub fn type_(&self) -> HashType {
    self.ty
  }

  /// Encode to serialized representation
  pub fn encode(&self, encoding: Encoding) -> String {
    if encoding == Encoding::SRI {
      return self.encode_with_type(encoding);
    }
    let mut s = String::new();
    self.encode_impl(encoding, &mut s);
    s
  }

  pub fn encode_with_type(&self, encoding: Encoding) -> String {
    let mut s = self.ty.to_string();
    if encoding == Encoding::SRI {
      s.push('-');
    } else {
      s.push(':');
    }
    self.encode_impl(encoding, &mut s);
    s
  }

  fn encode_impl(&self, encoding: Encoding, buf: &mut String) {
    let bytes = match encoding {
      Encoding::Base16 => {
        let mut bytes = vec![0; self.len_base16()];
        binascii::bin2hex(self.as_bytes(), &mut bytes).expect("Incorrect buffer size");
        bytes
      }
      Encoding::Base32 => {
        let mut bytes = vec![0; self.len_base32()];
        crate::base32::encode_into(self.as_bytes(), &mut bytes);
        bytes
      }
      Encoding::Base64 | Encoding::SRI => {
        let mut bytes = vec![0; self.len_base64()];
        binascii::b64encode(self.as_bytes(), &mut bytes).expect("Incorrect buffer size");
        bytes
      }
    };
    buf.push_str(unsafe { std::str::from_utf8_unchecked(&bytes) });
  }

  /// Decode from serialized representation
  pub fn decode(input: &str) -> Result<Self> {
    if let Some((ty, rest)) = util::break_str(input, ':') {
      Ok(Self::decode_with_type(rest, ty.parse()?, false)?)
    } else if let Some((ty, rest)) = util::break_str(input, '-') {
      Ok(Self::decode_with_type(rest, ty.parse()?, true)?)
    } else {
      Err(Error::UntypedHash(input.into()).into())
    }
  }

  pub fn decode_with_type(input: &str, ty: HashType, sri: bool) -> Result<Self> {
    let mut bytes = [0; 64];
    if !sri && input.len() == len_base16(ty.size()) {
      binascii::hex2bin(input.as_bytes(), &mut bytes).map_err(|e| anyhow::anyhow!("{:?}", e))?;
      Ok(Self {
        data: bytes,
        ty,
        len: ty.size(),
      })
    } else if !sri && input.len() == len_base32(ty.size()) {
      crate::base32::decode_into(input.as_bytes(), &mut bytes)?;
      Ok(Self {
        data: bytes,
        ty,
        len: ty.size(),
      })
    } else {
      todo!()
    }
  }

  pub fn hash_str(data: &str, ty: HashType) -> Self {
    Self::hash_bytes(data.as_bytes(), ty)
  }

  pub fn hash_bytes(data: &[u8], ty: HashType) -> Self {
    let mut ctx = Context::new(ty);
    ctx.input(data);
    ctx.into()
  }

  pub async fn hash_file<P: AsRef<Path>>(path: P, ty: HashType) -> Result<Self> {
    let path = path.as_ref();
    Self::hash(&mut crate::util::open_file(path).await?, ty).await
  }

  /// Hash the contents of an arbitrary byte stream.
  pub async fn hash<R: AsyncRead + Unpin>(r: &mut R, ty: HashType) -> Result<Self> {
    let mut ctx = Context::new(ty);
    loop {
      let mut buf = [0; 8192];
      if r.read(&mut buf).await? == 0 {
        break;
      }
      ctx.input(&buf);
    }

    Ok(ctx.into())
  }

  /// Convert `self` to a shorter hash by recursively XOR-ing bytes.
  pub fn truncate(&self, new_size: usize) -> Cow<Self> {
    if new_size >= self.len {
      return Cow::Borrowed(self);
    }
    let mut data = [0; 64];
    for i in 0..self.len {
      data[i % new_size] ^= self.data[i];
    }
    Cow::Owned(Self {
      len: new_size,
      data,
      ty: self.ty,
    })
  }

  #[inline]
  pub fn as_bytes(&self) -> &[u8] {
    &self.data[..self.len]
  }
}

impl PartialEq for Hash {
  fn eq(&self, other: &Self) -> bool {
    self.ty == other.ty && self.as_bytes() == other.as_bytes()
  }
}

impl Eq for Hash {}

impl std::hash::Hash for Hash {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_bytes().hash(state)
  }
}

impl Debug for Hash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Hash")
      .field(&format!("{}:{}", self.ty, self.encode(Encoding::Base64)))
      .finish()
  }
}

fn len_base16(size: usize) -> usize {
  size * 2
}

fn len_base32(size: usize) -> usize {
  (size * 8 - 1) / 5 + 1
}

fn len_base64(size: usize) -> usize {
  ((4 * size / 3) + 3) & !3
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_md5() {
    assert_eq!(
      Hash::hash_str("foobar", HashType::MD5).encode(Encoding::Base16),
      {
        let mut m = crypto::md5::Md5::new();
        m.input_str("foobar");
        m.result_str()
      },
    );
  }

  #[test]
  fn test_sha1() {
    assert_eq!(
      Hash::hash_str("foobar", HashType::SHA1).encode(Encoding::Base16),
      {
        let mut m = crypto::sha1::Sha1::new();
        m.input_str("foobar");
        m.result_str()
      },
    );
  }

  #[test]
  fn test_sha256() {
    assert_eq!(
      Hash::hash_str("foobar", HashType::SHA256).encode(Encoding::Base16),
      {
        let mut m = crypto::sha2::Sha256::new();
        m.input_str("foobar");
        m.result_str()
      },
    );
  }

  #[test]
  fn test_sha512() {
    assert_eq!(
      Hash::hash_str("foobar", HashType::SHA512).encode(Encoding::Base16),
      {
        let mut m = crypto::sha2::Sha512::new();
        m.input_str("foobar");
        m.result_str()
      },
    );
  }
}
