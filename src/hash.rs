use crate::error::*;
use crypto::digest::Digest;
use derive_more::Display;
use std::{borrow::Cow, path::Path};
use tokio::io::{AsyncRead, AsyncReadExt};

mod context;
mod sink;

use context::Context;
pub use sink::HashSink as Sink;

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
  pub fn parse_with_type(_s: &str, _ty: HashType) -> Result<Self> {
    todo!()
  }

  fn len_base16(&self) -> usize {
    self.len * 2
  }

  fn len_base32(&self) -> usize {
    (self.len * 8 - 1) / 5 + 1
  }

  fn len_base64(&self) -> usize {
    ((4 * self.len / 3) + 3) & !3
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
  pub fn decode(_s: &str) -> Result<Self> {
    todo!()
  }

  pub fn decode_with_type(&self, _encoding: Encoding) -> Result<Self> {
    todo!()
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
      if r.read(&mut buf).await.nowhere()? == 0 {
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

  pub fn as_bytes(&self) -> &[u8] {
    &self.data[..self.len]
  }
}

impl PartialEq for Hash {
  fn eq(&self, other: &Self) -> bool {
    self.as_bytes() == other.as_bytes() && self.ty == other.ty
  }
}

impl Eq for Hash {}

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
