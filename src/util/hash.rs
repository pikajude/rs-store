use lazy_static::lazy_static;
use std::{fmt, str::FromStr};

#[derive(Debug, thiserror::Error, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
  #[error("Invalid length `{0}` for base32 hash")]
  InvalidLength(usize),
  #[error("Invalid character `{0}` in base32 hash")]
  InvalidChar(char),
}

static BASE32_CHARS: &[u8; 32] = b"0123456789abcdfghijklmnpqrsvwxyz";

lazy_static! {
  static ref BASE32_CHARS_REVERSE: [u8; 256] = {
    let mut xs = [0xffu8; 256];
    for (n, c) in BASE32_CHARS.iter().enumerate() {
      xs[*c as usize] = n as u8;
    }
    xs
  };
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Hash([u8; Self::HASH_BYTES]);

impl Hash {
  pub const HASH_BYTES: usize = 20;
  pub const HASH_CHARS: usize = 32;

  pub const fn empty() -> Self {
    Self([0u8; Self::HASH_BYTES])
  }
}

impl fmt::Display for Hash {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut bytes = [b'0'; Self::HASH_CHARS];

    let mut nr_bits_left: usize = 0;
    let mut bits_left: u16 = 0;
    let mut pos = bytes.len();

    for b in &self.0 {
      bits_left |= (*b as u16) << nr_bits_left;
      nr_bits_left += 8;
      while nr_bits_left > 5 {
        bytes[pos - 1] = BASE32_CHARS[(bits_left & 0x1f) as usize];
        pos -= 1;
        bits_left >>= 5;
        nr_bits_left -= 5;
      }
    }

    if nr_bits_left > 0 {
      bytes[pos - 1] = BASE32_CHARS[(bits_left & 0x1f) as usize];
    }

    write!(f, "{}", unsafe { std::str::from_utf8_unchecked(&bytes) })
  }
}

impl FromStr for Hash {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.len() != Self::HASH_CHARS {
      return Err(Error::InvalidLength(s.len()));
    }
    let mut bytes = [0u8; Self::HASH_BYTES];
    let mut ix = 0;

    let mut nr_bits_left: usize = 0;
    let mut bits_left: u16 = 0;

    for c in s.chars().rev() {
      if c > 0xffu8 as char {
        return Err(Error::InvalidChar(c));
      }
      let byte = BASE32_CHARS_REVERSE[c as usize];
      if byte == 0xff {
        return Err(Error::InvalidChar(c));
      }
      bits_left |= (byte as u16) << nr_bits_left;
      nr_bits_left += 5;
      if nr_bits_left >= 8 {
        bytes[ix] = (bits_left & 0xff) as u8;
        ix += 1;
        bits_left >>= 8;
        nr_bits_left -= 8;
      }
    }

    assert!(!(nr_bits_left > 0 && bits_left != 0));

    Ok(Self(bytes))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use hex_literal::hex;
  #[test]
  fn parse() {
    assert_eq!(
      "7rnqb733s45x3x07612wrqjncx6ljp4p".parse::<Hash>(),
      Ok(Hash(hex!(
        "97 5c 49 4d 67 56 e2 cc 45 30 07 f4 d1 0b d1 63 9c 85 6d 3e"
      )))
    )
  }

  #[test]
  fn encode() {
    assert_eq!(
      "7rnqb733s45x3x07612wrqjncx6ljp4p"
        .parse::<Hash>()
        .unwrap()
        .to_string(),
      "7rnqb733s45x3x07612wrqjncx6ljp4p"
    );
  }
}
