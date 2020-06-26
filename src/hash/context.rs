use super::HashType;
use crypto::digest::Digest;

pub enum Context {
  Md5(crypto::md5::Md5),
  Sha1(crypto::sha1::Sha1),
  Sha256(crypto::sha2::Sha256),
  Sha512(crypto::sha2::Sha512),
}

macro_rules! do_impl {
  ($x:ident, $($t:tt)+) => {
    match $x {
      Context::Md5(m) => m.$($t)+,
      Context::Sha1(m) => m.$($t)+,
      Context::Sha256(m) => m.$($t)+,
      Context::Sha512(m) => m.$($t)+,
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
  pub fn new(s: HashType) -> Self {
    match s {
      HashType::MD5 => Self::Md5(crypto::md5::Md5::new()),
      HashType::SHA1 => Self::Sha1(crypto::sha1::Sha1::new()),
      HashType::SHA256 => Self::Sha256(crypto::sha2::Sha256::new()),
      HashType::SHA512 => Self::Sha512(crypto::sha2::Sha512::new()),
    }
  }
}
