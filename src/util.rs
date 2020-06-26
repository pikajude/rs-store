use crate::error::*;
use std::path::Path;
use tokio::fs::File;

pub async fn open_file<P: AsRef<Path>>(path: P) -> Result<File> {
  let path = path.as_ref();
  Ok(File::open(path).await.somewhere(path)?)
}
