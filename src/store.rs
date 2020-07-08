use crate::{
  error::*,
  hash::{Encoding, Hash, HashType},
  path::{Path as StorePath, PathSet},
  path_info::{PathInfo, ValidPathInfo},
};
use bytes::Bytes;
use futures::Stream;
use std::{
  borrow::Cow,
  path::{Path, PathBuf},
  sync::Arc,
};

pub mod cached;
pub mod local;

/// A Nix store, containing a lot of filepaths.
///
/// This store might be read-only, as in the case of a binary cache store or S3
/// bucket store. It may also be read-write, as in the case of a local
/// filestystem store or an SSH store.
#[async_trait]
pub trait Store: Send + Sync {
  fn store_path(&self) -> Cow<Path>;
  fn get_uri(&self) -> String;

  /// Convert some path to a store path, if it's a *direct* descendant of the
  /// store directory. This function does not assume the path exists.
  ///
  /// For arbitrarily deep descendants of a store directory, try
  /// `store_path_of`.
  fn parse_store_path(&self, path: &Path) -> Result<StorePath> {
    StorePath::new(path, self.store_path().as_ref())
  }

  /// If a Nix store path is a parent of `path`, return it. Unlike
  /// `parse_store_path`, this method fails on nonexistent paths, since it calls
  /// `canonicalize`.
  fn store_path_of(&self, path: &Path) -> Result<StorePath> {
    let p = path.canonicalize().somewhere(path)?;
    if !p.starts_with(self.store_path()) {
      return Err(Error::NotInStore(p));
    }
    self.parse_store_path(
      &p.components()
        .take(self.store_path().components().count() + 1)
        .collect::<PathBuf>(),
    )
  }

  fn print_store_path(&self, p: &StorePath) -> String {
    format!("{}/{}", self.store_path().display(), p)
  }

  fn make_store_path(&self, path_type: &str, hash: &Hash, name: &str) -> Result<StorePath> {
    let ident = format!(
      "{}:{}:{}:{}",
      path_type,
      hash.encode(Encoding::Base16),
      self.store_path().display(),
      name
    );
    let hash = Hash::hash_bytes(ident.as_bytes(), HashType::SHA256)
      .truncate(20)
      .into_owned();
    StorePath::from_parts(hash.as_bytes(), name)
  }

  fn make_output_path(&self, id: &str, hash: &Hash, name: &str) -> Result<StorePath> {
    self.make_store_path(
      &format!("output:{}", id),
      hash,
      &format!(
        "{}{}{}",
        name,
        if id == "out" { "" } else { "-" },
        if id == "out" { "" } else { id }
      ),
    )
  }

  fn make_type<'a, I: Iterator<Item = &'a StorePath>>(
    &self,
    mut s: String,
    references: I,
    has_self_reference: bool,
  ) -> String {
    for item in references {
      s.push(':');
      s.push_str(&self.print_store_path(item));
    }
    if has_self_reference {
      s.push_str(":self");
    }
    s
  }

  fn make_fixed_output_path<'a, I: Iterator<Item = &'a StorePath>>(
    &self,
    recursive: bool,
    hash: &Hash,
    name: &str,
    mut references: I,
    has_self_reference: bool,
  ) -> Result<StorePath> {
    if hash.type_() == HashType::SHA256 && recursive {
      self.make_store_path(
        &self.make_type("source".into(), references, has_self_reference),
        hash,
        name,
      )
    } else {
      assert!(references.next().is_none());
      self.make_store_path(
        "output:out",
        &Hash::hash_str(
          &format!(
            "fixed:out:{}{}:",
            if recursive { "r:" } else { "" },
            hash.encode(Encoding::Base16)
          ),
          HashType::SHA256,
        ),
        name,
      )
    }
  }

  fn make_text_path<'a, I: Iterator<Item = &'a StorePath>>(
    &self,
    name: &str,
    hash: &Hash,
    references: I,
  ) -> Result<StorePath> {
    assert!(hash.type_() == HashType::SHA256);
    self.make_store_path(
      &self.make_type("text".into(), references, false),
      hash,
      name,
    )
  }

  fn store_path_for_text<'a, I: Iterator<Item = &'a StorePath>>(
    &self,
    name: &str,
    contents: &str,
    references: I,
  ) -> Result<StorePath> {
    self.make_text_path(
      name,
      &Hash::hash_str(contents, HashType::SHA256),
      references,
    )
  }

  async fn store_path_for_file(
    &self,
    name: &str,
    path: &Path,
    algorithm: HashType,
  ) -> Result<(StorePath, Hash)> {
    let file_hash = Hash::hash_file(path, algorithm).await?;
    Ok((
      self.make_fixed_output_path(false, &file_hash, name, std::iter::empty(), false)?,
      file_hash,
    ))
  }

  async fn store_path_for_dir<F: FnMut(&Path) -> bool + Send>(
    &self,
    name: &str,
    path: &Path,
    algo: HashType,
    filter: F,
  ) -> Result<(StorePath, Hash)> {
    let mut h = crate::hash::Sink::new(algo);
    crate::archive::dump_path(path, &mut h, filter).await?;
    let hash = h.finish();
    Ok((
      self.make_fixed_output_path(true, &hash, name, std::iter::empty(), false)?,
      hash,
    ))
  }

  /// Get info about a valid path. If this method returns `None`, the path is
  /// known not to exist in the store.
  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>>;

  async fn get_referrers(&self, path: &StorePath) -> Result<PathSet>;

  async fn is_valid_path(&self, path: &StorePath) -> Result<bool> {
    self.get_path_info(path).await.map(|x| x.is_some())
  }

  async fn add_to_store<S: Stream<Item = Result<Bytes>> + Send + Unpin>(
    &self,
    info: &ValidPathInfo,
    source: S,
  ) -> Result<()>;

  async fn add_temp_root(&self, path: &StorePath) -> Result<()>;
}
