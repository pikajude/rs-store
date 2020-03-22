use crate::util::hash::Hash;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NarInfo {
  url: String,
  compression: String,
  file_hash: Hash,
  file_size: usize,
  system: String,
}
