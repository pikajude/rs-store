use super::{Hash, HashType};
use crypto::digest::Digest;

pub struct Context {
  ty: HashType,
  buf: Buf,
}

enum Buf {
  Md5(crypto::md5::Md5),
  Sha1(crypto::sha1::Sha1),
  Sha256(crypto::sha2::Sha256),
  Sha512(crypto::sha2::Sha512),
}

macro_rules! do_impl {
  ($x:ident, $($t:tt)+) => {
    #[allow(unused_mut)]
    match $x.buf {
      Buf::Md5(mut m) => m.$($t)+,
      Buf::Sha1(mut m) => m.$($t)+,
      Buf::Sha256(mut m) => m.$($t)+,
      Buf::Sha512(mut m) => m.$($t)+,
    }
  }
}

impl Digest for Context {
  fn input(&mut self, input: &[u8]) {
    do_impl!(self, input(input))
  }

  fn result(&mut self, out: &mut [u8]) {
    do_impl!(self, result(out))
  }

  fn reset(&mut self) {
    do_impl!(self, reset())
  }

  fn output_bits(&self) -> usize {
    do_impl!(self, output_bits())
  }

  fn block_size(&self) -> usize {
    do_impl!(self, block_size())
  }
}

impl Context {
  pub fn new(ty: HashType) -> Self {
    Self {
      ty,
      buf: match ty {
        HashType::MD5 => Buf::Md5(crypto::md5::Md5::new()),
        HashType::SHA1 => Buf::Sha1(crypto::sha1::Sha1::new()),
        HashType::SHA256 => Buf::Sha256(crypto::sha2::Sha256::new()),
        HashType::SHA512 => Buf::Sha512(crypto::sha2::Sha512::new()),
      },
    }
  }
}

impl From<Context> for Hash {
  fn from(mut c: Context) -> Self {
    let mut bytes = [0; 64];
    c.result(&mut bytes);
    Self {
      ty: c.ty,
      len: c.ty.size(),
      data: bytes,
    }
  }
}
