use crate::prelude::*;
use anyhow::Result;
use futures::{
  sink::{Sink, SinkExt},
  stream::StreamExt,
};
use nix::sys::stat::Mode;
use sink::RestoreSink;
use std::{error::Error, os::unix::fs::MetadataExt, path::Path};
use tokio::fs;

mod sink;

// if this is a type alias, it causes a compilation failure lol
pub struct PathFilter(Option<Box<dyn Fn(&Path) -> bool + Send + Sync>>);

impl PathFilter {
  pub fn always() -> Self {
    Self(None)
  }
}

impl FnOnce<(&Path,)> for PathFilter {
  type Output = bool;

  extern "rust-call" fn call_once(self, args: (&Path,)) -> Self::Output {
    if let Some(x) = self.0 {
      x.call_once(args)
    } else {
      true
    }
  }
}

impl FnMut<(&Path,)> for PathFilter {
  extern "rust-call" fn call_mut(&mut self, args: (&Path,)) -> Self::Output {
    if let Some(x) = &self.0 {
      x.call_once(args)
    } else {
      true
    }
  }
}

impl Fn<(&Path,)> for PathFilter {
  extern "rust-call" fn call(&self, args: (&Path,)) -> Self::Output {
    if let Some(x) = &self.0 {
      x.call_once(args)
    } else {
      true
    }
  }
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct ArchiveSink<S> {
  sink: S,
}

impl<S> ArchiveSink<S> {
  pub fn new(thing: S) -> Self {
    Self { sink: thing }
  }

  pub fn into_inner(self) -> S {
    self.sink
  }
}

impl<S: Sink<Bytes> + Unpin> ArchiveSink<S>
where
  S::Error: std::error::Error + Send + Sync + 'static,
{
  pub async fn write_str<B: AsRef<str>>(&mut self, string: B) -> Result<()> {
    let string = string.as_ref();
    self.write_usize(string.len()).await?;
    self.write_bytes(string.as_bytes()).await?;
    self.pad(string.len()).await?;
    Ok(())
  }

  pub async fn write_usize(&mut self, len: usize) -> Result<()> {
    let mut buf = [0u8; 8];
    buf[0] = len as u8;
    buf[1] = (len >> 8) as u8;
    buf[2] = (len >> 16) as u8;
    buf[3] = (len >> 24) as u8;
    buf[4] = (len >> 32) as u8;
    buf[5] = (len >> 40) as u8;
    buf[6] = (len >> 48) as u8;
    buf[7] = (len >> 56) as u8;
    self.write_bytes(&buf).await
  }

  pub async fn write_bytes<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<()> {
    let bs = Bytes::copy_from_slice(bytes.as_ref());
    Ok(self.sink.send(bs).await?)
  }

  async fn pad(&mut self, len: usize) -> Result<()> {
    let rest = (len % 8) as u8;
    if rest > 0 {
      self.write_bytes(&vec![0; 8 - rest as usize]).await?;
    }
    Ok(())
  }
}

#[async_recursion]
pub async fn dump_path<W: Sink<Bytes> + Send + Unpin>(
  path: &Path,
  sink: &mut ArchiveSink<W>,
  filter: &PathFilter,
) -> Result<()>
where
  W::Error: Error + Send + Sync + 'static,
{
  let meta = fs::metadata(path).await?;
  sink.write_str("(").await?;

  if meta.file_type().is_file() {
    sink.write_str("type").await?;
    sink.write_str("regular").await?;
    if Mode::from_bits_truncate(meta.mode()).contains(Mode::S_IXUSR) {
      sink.write_str("executable").await?;
      sink.write_str("").await?;
    }

    dump_file(path, meta.len(), sink).await?;
  } else if meta.file_type().is_dir() {
    sink.write_str("type").await?;
    sink.write_str("directory").await?;

    while let Some(file) = fs::read_dir(path).await?.next_entry().await? {
      if filter(&file.path()) {
        sink.write_str("entry").await?;
        sink.write_str("(").await?;
        sink.write_str("name").await?;

        let name = file.file_name();
        sink
          .write_str(
            name
              .to_str()
              .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?,
          )
          .await?;

        sink.write_str("node").await?;
        dump_path(&file.path(), sink, filter).await?;
        sink.write_str(")").await?;
      }
    }
  } else if meta.file_type().is_symlink() {
    sink.write_str("type").await?;
    sink.write_str("symlink").await?;
    sink.write_str("target").await?;
    sink
      .write_str(fs::canonicalize(path).await?.display().to_string())
      .await?;
  } else {
    bail!("path `{}' has an unsupported type", path.display());
  }

  sink.write_str(")").await?;

  Ok(())
}

async fn dump_file<P: AsRef<Path>, W: Sink<Bytes> + Unpin>(
  path: P,
  size: u64,
  sink: &mut ArchiveSink<W>,
) -> Result<()>
where
  W::Error: Error + Send + Sync + 'static,
{
  sink.write_str("contents").await?;
  sink.write_usize(size as usize).await?;

  let mut file_reader = crate::util::stream_file(path).await?;
  while let Some(bytes) = file_reader.next().await {
    sink.write_bytes(bytes?).await?;
  }

  if size % 8 > 0 {
    sink.write_bytes(vec![0u8; 8 - (size as usize % 8)]).await?;
  }

  Ok(())
}

pub async fn restore_into<P: AsRef<Path>, S: ByteStream + Send + Unpin>(
  path: P,
  mut source: S,
) -> Result<()> {
  sink::parse_dump(&mut RestoreSink::new(path.as_ref()), &mut source).await
}
