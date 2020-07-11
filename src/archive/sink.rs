use crate::store::ByteStream;
use anyhow::Result;
use async_recursion::async_recursion;
use bytes::Bytes;
use futures::{io::AsyncReadExt, sink::Sink, stream::TryStreamExt, AsyncRead};
use libc::{S_IXGRP, S_IXOTH, S_IXUSR};
use nix::sys::stat;
use stat::Mode;
use std::{
  borrow::Cow,
  fs::Permissions,
  os::unix::io::AsRawFd,
  path::{Path, PathBuf},
  pin::Pin,
  task::{Context, Poll},
};
use tokio::fs::File;

#[async_trait]
pub trait ParseSink {
  async fn create_directory(&mut self, path: Option<&Path>) -> Result<()>;
  async fn create_file(&mut self, path: Option<&Path>) -> Result<()>;
  async fn create_symlink(&mut self, path: Option<&Path>, target: PathBuf) -> Result<()>;
  async fn set_executable(&mut self) -> Result<()>;
  async fn allocate_contents(&mut self, size: usize) -> Result<()>;
  async fn receive_contents<S: AsyncRead + Send>(&mut self, bytes: S) -> Result<()>;
}

pub async fn parse_dump<S: ParseSink + Send, R: AsyncRead + Send + Unpin>(
  sink: &mut S,
  source: &mut R,
) -> Result<()> {
  static NAR_MAGIC: &str = "nix-archive-1";
  let vers = read_bytes_len(source, NAR_MAGIC.len()).await?;
  if vers != NAR_MAGIC.as_bytes() {
    return Err(anyhow::anyhow!("mismatch"));
  }
  parse(sink, source, None).await
}

#[async_recursion]
async fn parse<S: ParseSink + Send, R: AsyncRead + Send + Unpin>(
  sink: &mut S,
  source: &mut R,
  path: Option<PathBuf>,
) -> Result<()> {
  let s = read_bytes(source).await?;
  if s != b"(" {
    return Err(anyhow::anyhow!("bad archive"));
  }

  #[derive(PartialEq, Eq)]
  enum EntryType {
    File,
    Dir,
    Symlink,
  };

  let mut ty = None;

  loop {
    let s = read_bytes(source).await?;
    match &s[..] {
      b")" => break,
      b"type" => {
        if ty.is_some() {
          return Err(anyhow::anyhow!("multiple type fields"));
        }
        let tagged_ty = read_bytes(source).await?;
        match &tagged_ty[..] {
          b"regular" => {
            ty = Some(EntryType::File);
            sink.create_file(path.as_deref()).await?;
          }
          b"directory" => {
            ty = Some(EntryType::Dir);
            sink.create_directory(path.as_deref()).await?;
          }
          b"symlink" => {
            ty = Some(EntryType::Symlink);
          }
          x => {
            return Err(anyhow::anyhow!(
              "bad archive type: {}",
              String::from_utf8_lossy(x)
            ))
          }
        }
      }
      b"contents" if ty == Some(EntryType::File) => {
        unimplemented!("parseContents");
      }
      b"executable" if ty == Some(EntryType::File) => {
        let marker = read_bytes(source).await?;
        if marker != b"" {
          return Err(anyhow::anyhow!("executable marker with non-empty value"));
        }
        sink.set_executable().await?;
      }
      b"entry" if ty == Some(EntryType::Dir) => {
        if read_bytes(source).await? != b"(" {
          return Err(anyhow::anyhow!("expected open tag"));
        }
        let mut name = vec![];
        loop {
          let s = read_bytes(source).await?;
          match &s[..] {
            b")" => break,
            b"name" => {
              name = read_bytes(source).await?;
              if name.is_empty()
                || name == b"."
                || name == b".."
                || name.iter().any(|x| *x == b'/' || *x == 0)
              {
                return Err(anyhow::anyhow!(
                  "nar contains bad filename {}",
                  String::from_utf8_lossy(&name)
                ));
              }
            }
            b"node" => {
              if name.is_empty() {
                return Err(anyhow::anyhow!("entry name is missing"));
              }
              parse(
                sink,
                source,
                Some(match path {
                  None => PathBuf::from(String::from_utf8(name.clone())?),
                  Some(ref x) => x.join(String::from_utf8(name.clone())?),
                }),
              )
              .await?;
            }
            x => {
              return Err(anyhow::anyhow!(
                "nar contains bad filename {}",
                String::from_utf8_lossy(x)
              ))
            }
          }
        }
      }
      b"target" if ty == Some(EntryType::Symlink) => {
        let target = read_bytes(source).await?;
        sink
          .create_symlink(path.as_deref(), String::from_utf8(target)?.into())
          .await?;
      }
      x => {
        return Err(anyhow::anyhow!(
          "bad NAR field: {}",
          String::from_utf8_lossy(x)
        ))
      }
    }
  }

  Ok(())
}

async fn read_num<R: AsyncRead + Unpin>(s: &mut R) -> Result<usize> {
  let mut buf = [0u8; 8];
  s.read_exact(&mut buf).await?;
  let result = buf[0] as usize
    | (buf[1] as usize) << 8
    | (buf[2] as usize) << 16
    | (buf[3] as usize) << 24
    | (buf[4] as usize) << 32
    | (buf[5] as usize) << 40
    | (buf[6] as usize) << 48
    | (buf[7] as usize) << 56;
  Ok(result)
}

async fn read_bytes_len<R: AsyncRead + Unpin>(s: &mut R, max: usize) -> Result<Vec<u8>> {
  let len = read_num(s).await?;
  if len > max {
    return Err(anyhow::anyhow!(
      "string is longer than specified max {}",
      max
    ));
  }
  let mut bytes = vec![0; len];
  s.read_exact(&mut bytes).await?;
  read_padding(s, len).await?;
  Ok(bytes)
}

async fn read_bytes<R: AsyncRead + Unpin>(s: &mut R) -> Result<Vec<u8>> {
  read_bytes_len(s, usize::MAX).await
}

async fn read_padding<R: AsyncRead + Unpin>(s: &mut R, len: usize) -> Result<()> {
  if len % 8 > 0 {
    let mut pad = vec![0; 8 - (len % 8)];
    s.read_exact(&mut pad).await?;
    if pad.iter().any(|x| *x > 0) {
      return Err(anyhow::anyhow!("incorrect padding"));
    }
  }
  Ok(())
}

pub struct RestoreSink {
  root: PathBuf,
  last_file: Option<File>,
}

impl RestoreSink {
  pub fn new<S: Into<PathBuf>>(s: S) -> Self {
    Self {
      root: s.into(),
      last_file: None,
    }
  }

  fn get_path(&self, p: Option<&Path>) -> Cow<Path> {
    p.map_or(Cow::Borrowed(&self.root), |p| Cow::Owned(self.root.join(p)))
  }
}

#[async_trait]
impl ParseSink for RestoreSink {
  async fn create_directory(&mut self, path: Option<&Path>) -> Result<()> {
    Ok(tokio::fs::create_dir(self.get_path(path)).await?)
  }

  async fn create_file(&mut self, path: Option<&Path>) -> Result<()> {
    self.last_file = Some(tokio::fs::File::create(self.get_path(path)).await?);
    Ok(())
  }

  async fn create_symlink(&mut self, path: Option<&Path>, target: PathBuf) -> Result<()> {
    todo!()
  }

  async fn set_executable(&mut self) -> Result<()> {
    if let Some(x) = self.last_file.as_ref() {
      let stat = stat::fstat(x.as_raw_fd())?;
      stat::fchmod(
        x.as_raw_fd(),
        unsafe { stat::Mode::from_bits_unchecked(stat.st_mode) }
          | (stat::Mode::S_IXUSR | stat::Mode::S_IXGRP | stat::Mode::S_IXOTH),
      )?;
    }
    Ok(())
  }

  async fn allocate_contents(&mut self, size: usize) -> Result<()> {
    todo!()
  }

  async fn receive_contents<S: AsyncRead + Send>(&mut self, bytes: S) -> Result<()> {
    todo!()
  }
}
