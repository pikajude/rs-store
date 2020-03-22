use crypto::{digest::Digest, sha2::Sha256};

pub fn sha256(bytes: &[u8]) -> Vec<u8> {
  let mut s = Sha256::new();
  s.input(bytes);
  let mut v = vec![0u8; s.output_bytes()];
  s.result(&mut v);
  v
}

#[test]
fn test_digest() {
  let sig = sha256(b"hello world!");
  assert_eq!(sig, b"\x75\x09\xe5\xbd\xa0\xc7\x62\xd2\xba\xc7\xf9\x0d\x75\x8b\x5b\x22\x63\xfa\x01\xcc\xbc\x54\x2a\xb5\xe3\xdf\x16\x3b\xe0\x8e\x6c\xa9");
}
