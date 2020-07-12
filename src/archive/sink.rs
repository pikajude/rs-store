use crate::prelude::*;
use async_recursion::async_recursion;
use futures::{
  io::{AsyncRead, AsyncReadExt},
  stream::TryStreamExt,
};
use nix::sys::stat::*;
use std::{
  borrow::Cow,
  os::unix::io::AsRawFd,
  path::{Path, PathBuf},
};
use tokio::fs::File;

#[derive(Debug, Error)]
pub enum ParseError {
  #[error("input is not a Nix archive")]
  InvalidNar,
  #[error("an entry in the archive has multiple `type' fields, which is not allowed")]
  MultipleTypeFields,
  #[error("unknown archive type `{0}'")]
  UnknownArchiveType(String),
  #[error("nar contains invalid filename `{0}'")]
  InvalidFilename(String),
  #[error("invalid executable marker")]
  ExecutableMarker,
  #[error("expected {0}")]
  Expected(&'static str),
  #[error("unknown field `{0}'")]
  UnknownField(String),
  #[error("string is longer ({0} bytes) than allowed max {1}")]
  StringTooLong(usize, usize),
  #[error("non-zero padding in NAR")]
  NonzeroPadding,
}

fn utf8_bytes(b: impl AsRef<[u8]>) -> String {
  String::from_utf8_lossy(b.as_ref()).into()
}

#[async_trait]
pub trait ParseSink {
  async fn create_directory(&mut self, path: Option<&Path>) -> Result<()>;
  async fn create_file(&mut self, path: Option<&Path>) -> Result<()>;
  async fn create_symlink(&mut self, path: Option<&Path>, target: PathBuf) -> Result<()>;
  async fn set_executable(&mut self) -> Result<()>;
  async fn allocate_contents(&mut self, size: usize) -> Result<()>;
  async fn receive_contents<S: AsyncRead + Send + Unpin>(&mut self, bytes: S) -> Result<()>;
}

pub async fn parse_dump<S: ParseSink + Send, R: ByteStream + Send + Unpin>(
  sink: &mut S,
  source: &mut R,
) -> Result<()> {
  static NAR_MAGIC: &str = "nix-archive-1";
  let mut source = source.into_async_read();
  let vers = read_bytes_len(&mut source, NAR_MAGIC.len()).await?;
  if vers != NAR_MAGIC.as_bytes() {
    bail!(ParseError::InvalidNar);
  }
  parse(sink, &mut source, None).await
}

#[async_recursion]
async fn parse<S: ParseSink + Send, R: AsyncRead + Send + Unpin>(
  sink: &mut S,
  source: &mut R,
  path: Option<PathBuf>,
) -> Result<()> {
  let s = read_bytes(source).await?;
  if s != b"(" {
    bail!(ParseError::InvalidNar);
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
          bail!(ParseError::MultipleTypeFields);
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
          x => bail!(ParseError::UnknownArchiveType(
            String::from_utf8_lossy(x).into()
          )),
        }
      }
      b"contents" if ty == Some(EntryType::File) => {
        let filesize = read_num(source).await?;
        sink.allocate_contents(filesize).await?;
        sink.receive_contents(source.take(filesize as u64)).await?;
        read_padding(source, filesize).await?;
      }
      b"executable" if ty == Some(EntryType::File) => {
        let marker = read_bytes(source).await?;
        if marker != b"" {
          bail!(ParseError::ExecutableMarker);
        }
        sink.set_executable().await?;
      }
      b"entry" if ty == Some(EntryType::Dir) => {
        if read_bytes(source).await? != b"(" {
          bail!(ParseError::Expected("open tag"));
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
                bail!(ParseError::InvalidFilename(utf8_bytes(&name)));
              }
            }
            b"node" => {
              if name.is_empty() {
                bail!(ParseError::Expected("entry name"));
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
            x => bail!(ParseError::UnknownField(utf8_bytes(x))),
          }
        }
      }
      b"target" if ty == Some(EntryType::Symlink) => {
        let target = read_bytes(source).await?;
        sink
          .create_symlink(path.as_deref(), String::from_utf8(target)?.into())
          .await?;
      }
      x => bail!(ParseError::UnknownField(utf8_bytes(x))),
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
    bail!(ParseError::StringTooLong(len, max));
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
      bail!(ParseError::NonzeroPadding);
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

  fn file(&mut self) -> Result<&mut File> {
    self
      .last_file
      .as_mut()
      .ok_or_else(|| anyhow!("nar sink is in an invalid state"))
  }
}

#[async_trait]
impl ParseSink for RestoreSink {
  async fn create_directory(&mut self, path: Option<&Path>) -> Result<()> {
    trace!("creating directory {}", self.get_path(path).display());
    Ok(tokio::fs::create_dir(self.get_path(path)).await?)
  }

  async fn create_file(&mut self, path: Option<&Path>) -> Result<()> {
    trace!("importing file {}", self.get_path(path).display());
    self.last_file = Some(tokio::fs::File::create(self.get_path(path)).await?);
    Ok(())
  }

  async fn create_symlink(&mut self, path: Option<&Path>, target: PathBuf) -> Result<()> {
    todo!()
  }

  async fn set_executable(&mut self) -> Result<()> {
    let x = self.file()?;
    let stat = fstat(x.as_raw_fd())?;
    fchmod(
      x.as_raw_fd(),
      unsafe { Mode::from_bits_unchecked(stat.st_mode) }
        | (Mode::S_IXUSR | Mode::S_IXGRP | Mode::S_IXOTH),
    )?;
    Ok(())
  }

  async fn allocate_contents(&mut self, size: usize) -> Result<()> {
    // copied block from nix crate src
    #[cfg(any(
      target_os = "linux",
      target_os = "android",
      target_os = "emscripten",
      target_os = "fuchsia",
      any(target_os = "wasi", target_env = "wasi"),
      target_os = "freebsd"
    ))]
    nix::fcntl::posix_fallocate(self.file()?.as_raw_fd(), 0, size as i64)?;
    Ok(())
  }

  async fn receive_contents<S: AsyncRead + Send + Unpin>(&mut self, mut bytes: S) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let x = self.file()?;
    let mut buf = [0; 65536];
    let mut total = 0;
    loop {
      let len = bytes.read(&mut buf).await?;
      total += len;
      if len > 0 {
        x.write_all(&buf[..len]).await?;
      } else {
        break;
      }
    }
    trace!("imported {} bytes", total);

    Ok(())
  }
}
