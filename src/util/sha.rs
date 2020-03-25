use crate::util::hash::Hash;
use crypto::{digest::Digest, sha2::Sha256};

pub fn sha256(bytes: &[u8]) -> Hash {
  let mut s = Sha256::new();
  s.input(bytes);
  let mut v = vec![0u8; s.output_bytes()];
  s.result(&mut v);
  Hash::from_data(&v)
}
