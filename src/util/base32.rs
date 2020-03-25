use lazy_static::lazy_static;

#[derive(thiserror::Error, Debug, Eq, PartialEq, Clone, Copy)]
pub enum Error {
  #[error("Character `{0}` is not permitted in a base32 hash")]
  InvalidChar(char),
  #[error("Trailing bits in hash")]
  TrailingBits,
}

static BASE32_CHARS: &'static [u8; 32] = &b"0123456789abcdfghijklmnpqrsvwxyz";

lazy_static! {
  static ref BASE32_CHARS_REVERSE: [u8; 256] = {
    let mut xs = [0xffu8; 256];
    for (n, c) in BASE32_CHARS.iter().enumerate() {
      xs[*c as usize] = n as u8;
    }
    xs
  };
}

pub fn decode(input: &str) -> Result<Vec<u8>, Error> {
  let mut res = Vec::with_capacity(decode_len(input.len()));

  let mut nr_bits_left: usize = 0;
  let mut bits_left: u16 = 0;

  for c in input.chars().rev() {
    let b = BASE32_CHARS_REVERSE[c as usize];
    if b == 0xff {
      return Err(Error::InvalidChar(c));
    }
    bits_left |= (b as u16) << nr_bits_left;
    nr_bits_left += 5;
    if nr_bits_left >= 8 {
      res.push((bits_left & 0xff) as u8);
      bits_left >>= 8;
      nr_bits_left -= 8;
    }
  }

  if nr_bits_left > 0 && bits_left != 0 {
    return Err(Error::TrailingBits);
  }

  Ok(res)
}

pub fn encode(input: &[u8]) -> String {
  let len = encode_len(input.len());
  let mut output = vec![0; len];

  let mut nr_bits_left: usize = 0;
  let mut bits_left: u16 = 0;
  let mut pos = len;

  for b in input {
    bits_left |= (*b as u16) << nr_bits_left;
    nr_bits_left += 8;
    while nr_bits_left > 5 {
      output[pos - 1] = BASE32_CHARS[(bits_left & 0x1f) as usize];
      pos -= 1;
      bits_left >>= 5;
      nr_bits_left -= 5;
    }
  }

  if nr_bits_left > 0 {
    output[pos - 1] = BASE32_CHARS[(bits_left & 0x1f) as usize];
    pos -= 1;
  }

  assert_eq!(pos, 0);

  unsafe { String::from_utf8_unchecked(output) }
}

fn encode_len(len: usize) -> usize {
  if len == 0 {
    0
  } else {
    (len * 8 - 1) / 5 + 1
  }
}

fn decode_len(len: usize) -> usize {
  len * 5 / 8
}
