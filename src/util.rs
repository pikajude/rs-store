use crate::error::*;
use std::path::Path;
use tokio::fs::File;

pub async fn open_file<P: AsRef<Path>>(path: P) -> Result<File> {
  let path = path.as_ref();
  Ok(File::open(path).await.somewhere(path)?)
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
