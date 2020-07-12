use anyhow::Result;
use bytes::Bytes;
use futures::{stream::TryStreamExt, Stream};
use std::{
  io,
  path::{Path, PathBuf},
};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub async fn open_file<P: AsRef<Path>>(path: P) -> Result<File> {
  let path = path.as_ref();
  Ok(File::open(path).await?)
}

pub async fn stream_file<P: AsRef<Path>>(path: P) -> Result<impl Stream<Item = io::Result<Bytes>>> {
  let buf: PathBuf = path.as_ref().into();
  Ok(FramedRead::new(open_file(&buf).await?, BytesCodec::new()).map_ok(|b| b.freeze()))
}

pub fn break_str(s: &str, pattern: char) -> Option<(&str, &str)> {
  let indexof = s.find(pattern)?;

  Some((&s[..indexof], &s[indexof + pattern.len_utf8()..]))
}

#[test]
fn test_break() {
  assert_eq!(break_str("foo:bar", ':'), Some(("foo", "bar")));
  assert_eq!(break_str("foo:", ':'), Some(("foo", "")));
  assert_eq!(break_str("foo", ':'), None);
}

#[cfg(test)]
pub fn run_test<F: futures::Future<Output = anyhow::Result<()>>>(test: F) -> anyhow::Result<()> {
  if std::env::var_os("RUST_LOG").is_some() {
    let _ = pretty_env_logger::try_init();
  } else {
    let _ = pretty_env_logger::formatted_builder()
      .filter_module("nix_store", log::LevelFilter::Trace)
      .try_init();
  }
  tokio::runtime::Runtime::new().unwrap().block_on(test)
}
