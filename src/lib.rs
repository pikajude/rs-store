#![feature(trait_alias, never_type)]

use async_trait::async_trait;
use error::*;
use hash::{Encoding, Hash, HashType};
use path::Path as StorePath;
use std::path::{Path, PathBuf};

pub mod archive;
pub mod base32;
pub mod error;
pub mod hash;
pub mod path;
pub mod util;

#[async_trait]
pub trait Store {
  fn store_path(&self) -> &Path;
  fn get_uri(&self) -> String;

  /// Convert some path to a store path, if it's a *direct* descendant of the
  /// store directory. This function does not assume the path exists.
  ///
  /// For arbitrarily deep descendants of a store directory, try
  /// `store_path_of`.
  fn parse_store_path(&self, path: &Path) -> Result<StorePath> {
    StorePath::new(path, self.store_path())
  }

  /// If a Nix store path is a parent of `path`, return it. This function will
  /// fail on invalid paths, since it calls `canonicalize`.
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
    let mut h = hash::Sink::new(algo);
    archive::dump_path(path, &mut h, filter).await?;
    let hash = h.finish();
    Ok((
      self.make_fixed_output_path(true, &hash, name, std::iter::empty(), false)?,
      hash,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct TestStore;

  impl Store for TestStore {
    fn store_path(&self) -> &Path {
      Path::new("/local/nix")
    }

    fn get_uri(&self) -> String {
      "local".into()
    }
  }

  #[test]
  fn parse_path() {
    assert_eq!(
      TestStore
        .parse_store_path(Path::new(
          "/local/nix/5c9a1g1jdqv2jk9k4nbxs9y2445l6jja-foo.txt"
        ))
        .expect("failure"),
      StorePath::from_parts(
        &[74, 74, 67, 11, 33, 194, 39, 221, 151, 37, 51, 77, 41, 54, 110, 50, 188, 160, 18, 43],
        "foo.txt"
      )
      .expect("failure")
    );

    assert!(TestStore
      .parse_store_path(Path::new("/not/in/store"))
      .is_err());
  }

  #[test]
  #[ignore] // runs path.canonicalize()
  fn to_path() {
    assert_eq!(
      &*TestStore
        .store_path_of(Path::new("/local/nix/foo/bar/baz"))
        .expect("no parse")
        .name,
      "foo"
    );
  }

  #[test]
  fn mk_path() {
    let h = Hash::hash_str("Hello, world!", HashType::SHA256);
    let spath = TestStore
      .make_store_path("source", &h, "foo.txt")
      .expect("No good");
    assert_eq!(
      spath.to_string(),
      "5c9a1g1jdqv2jk9k4nbxs9y2445l6jja-foo.txt"
    );
  }
}
