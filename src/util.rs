use crate::error::*;
use bytes::Bytes;
use futures::{stream::TryStreamExt, Stream};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

pub async fn open_file<P: AsRef<Path>>(path: P) -> Result<File> {
  let path = path.as_ref();
  Ok(File::open(path).await.somewhere(path)?)
}

pub async fn stream_file<P: AsRef<Path>>(path: P) -> Result<impl Stream<Item = Result<Bytes>>> {
  let buf: PathBuf = path.as_ref().into();
  let f = open_file(&buf).await?;
  Ok(
    FramedRead::new(f, BytesCodec::new())
      .map_ok(|b| b.freeze())
      .map_err(move |i| Error::IoAt(i, buf.clone())),
  )
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
