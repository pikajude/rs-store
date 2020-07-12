use crate::{base32, prelude::*};
use anyhow::Result;
use derive_more::{Deref, Display};
use std::{
  cmp::Ordering,
  collections::BTreeSet,
  convert::TryInto,
  fmt,
  path::{Path as StdPath, PathBuf},
  str::FromStr,
};

pub const HASH_BYTES: usize = 20;
pub const HASH_CHARS: usize = 32;

pub type PathSet = BTreeSet<Path>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("path is not in the nix store: `{}'", _0.display())]
  NotInStore(PathBuf),
  #[error("invalid filepath for store: `{}'", _0.display())]
  InvalidFilepath(PathBuf),
  #[error("invalid store path name: {0:?}")]
  InvalidStorePathName(String),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
#[display(fmt = "{}-{}", hash, name)]
pub struct Path {
  pub hash: Hash,
  pub name: Name,
}

impl Path {
  pub fn new(p: &StdPath, store_dir: &StdPath) -> Result<Self> {
    if p.parent() != Some(store_dir) {
      bail!(Error::NotInStore(p.into()));
    }

    Self::from_base_name(
      p.file_name()
        .ok_or_else(|| Error::InvalidFilepath(p.into()))?
        .to_str()
        .ok_or_else(|| Error::InvalidFilepath(p.into()))?,
    )
  }

  pub fn from_parts(bytes: &[u8], name: &str) -> Result<Self> {
    Ok(Path {
      hash: Hash(
        bytes
          .try_into()
          .map_err(|_| crate::hash::Error::WrongHashLen(bytes.len()))?,
      ),
      name: name.parse()?,
    })
  }

  pub fn from_base_name(base_name: &str) -> Result<Self> {
    if base_name.len() < HASH_CHARS + 1 || base_name.as_bytes()[HASH_CHARS] != b'-' {
      bail!(Error::InvalidFilepath(base_name.into()));
    }

    Ok(Path {
      hash: Hash::decode(&base_name[0..HASH_CHARS])?,
      name: base_name[HASH_CHARS + 1..].parse()?,
    })
  }
}

#[derive(Clone, PartialEq, Eq, Debug, Deref)]
pub struct Hash([u8; HASH_BYTES]);

impl Hash {
  pub fn decode<S: AsRef<str>>(s: S) -> Result<Self> {
    let s = s.as_ref();
    assert_eq!(s.len(), HASH_CHARS);
    let v = base32::decode(s.as_bytes())?;
    assert_eq!(v.len(), HASH_BYTES);
    let mut bytes: [u8; 20] = Default::default();
    bytes.copy_from_slice(&v[..HASH_BYTES]);
    Ok(Self(bytes))
  }
}

impl fmt::Display for Hash {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut buf = vec![0; HASH_CHARS];
    base32::encode_into(&self.0, &mut buf);
    f.write_str(unsafe { std::str::from_utf8_unchecked(&buf) })
  }
}

impl Ord for Hash {
  fn cmp(&self, other: &Self) -> Ordering {
    // Historically we've sorted store paths by their base32
    // serialization, but our base32 encodes bytes in reverse
    // order. So compare them in reverse order as well.
    self.0.iter().rev().cmp(other.0.iter().rev())
  }
}

impl PartialOrd for Hash {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deref, Display)]
#[deref(forward)]
pub struct Name(String);

impl FromStr for Name {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Error> {
    fn is_valid_char(c: char) -> bool {
      c.is_ascii_alphabetic()
        || c.is_ascii_digit()
        || c == '+'
        || c == '-'
        || c == '.'
        || c == '_'
        || c == '?'
        || c == '='
    }

    if s.is_empty() {
      return Err(Error::InvalidStorePathName("".into()));
    }
    if s.len() > 211 {
      return Err(Error::InvalidStorePathName(s.into()));
    }
    if s.starts_with('.') || !s.chars().all(is_valid_char) {
      return Err(Error::InvalidStorePathName(s.into()));
    }
    Ok(Self(s.into()))
  }
}

#[test]
fn test_basic() {
  let bytes = [
    6, 95, 221, 239, 70, 175, 129, 185, 229, 36, 165, 29, 129, 142, 252, 186, 86, 169, 222, 64,
  ];
  let basename = "nix-2.4";

  let path = Path::new(
    StdPath::new("/nix/store/83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4"),
    StdPath::new("/nix/store"),
  )
  .unwrap();

  assert_eq!(&*path.name, basename);
  assert_eq!(*path.hash, bytes);

  // roundtrip
  assert_eq!(
    path,
    Path::from_parts(&bytes[..], basename).expect("no parse")
  );

  assert_eq!(path.to_string(), "83gajmmszj7827d54kjvk0dg8vpxspq6-nix-2.4");
}
