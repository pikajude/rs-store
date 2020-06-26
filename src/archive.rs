use crate::error::*;
use async_recursion::async_recursion;
use futures::{
  sink::{Sink, SinkExt},
  stream::StreamExt,
};
use std::path::Path;
use tokio_util::codec::{BytesCodec, FramedRead};

pub trait ArchiveSink = Sink<ArchiveData> + Unpin;

pub enum ArchiveData {
  Tag(&'static str),
  Bytes(bytes::Bytes),
  Int(u64),
}

#[async_recursion(?Send)]
pub async fn dump_path<P: AsRef<Path>, W: ArchiveSink, F: FnMut(&Path) -> bool>(
  path: P,
  sink: &mut W,
  mut filter: F,
) -> Result<()>
where
  Error: From<W::Error>,
{
  let path = path.as_ref();
  let meta = tokio::fs::metadata(path).await.somewhere(path)?;
  sink.send(ArchiveData::Tag("(")).await?;

  if meta.file_type().is_file() {
    sink.send(ArchiveData::Tag("type")).await?;
    sink.send(ArchiveData::Tag("regular")).await?;
    #[cfg(unix)]
    {
      use nix::sys::stat::Mode;
      use std::os::unix::fs::MetadataExt;

      if Mode::from_bits_truncate(meta.mode()).contains(Mode::S_IXUSR) {
        sink.send(ArchiveData::Tag("executable")).await?;
        sink.send(ArchiveData::Tag("")).await?;
      }
    }

    dump_file(path, meta.len(), sink).await?;
  } else if meta.file_type().is_dir() {
    sink.send(ArchiveData::Tag("type")).await?;
    sink.send(ArchiveData::Tag("directory")).await?;

    while let Some(file) = tokio::fs::read_dir(path)
      .await
      .somewhere(path)?
      .next_entry()
      .await
      .nowhere()?
    {
      if filter(&file.path()) {
        sink.send(ArchiveData::Tag("entry")).await?;
        sink.send(ArchiveData::Tag("(")).await?;
        sink.send(ArchiveData::Tag("name")).await?;

        #[cfg(unix)]
        {
          use std::os::unix::ffi::OsStrExt;
          let name = file.file_name();
          let filebytes = name.as_bytes().to_vec().into();
          sink.send(ArchiveData::Bytes(filebytes)).await?;
        }

        #[cfg(not(unix))]
        compile_error!("FIXME: implement filename() on Windows");

        sink.send(ArchiveData::Tag("node")).await?;
        dump_path(&file.path(), sink, &mut filter).await?;
        sink.send(ArchiveData::Tag(")")).await?;
      }
    }
  } else if meta.file_type().is_symlink() {
    sink.send(ArchiveData::Tag("type")).await?;
    sink.send(ArchiveData::Tag("symlink")).await?;
    sink.send(ArchiveData::Tag("target")).await?;
  }

  todo!()
}

async fn dump_file<P: AsRef<Path>, W: ArchiveSink>(path: P, size: u64, sink: &mut W) -> Result<()>
where
  Error: From<W::Error>,
{
  sink.send(ArchiveData::Tag("contents")).await?;
  sink.send(ArchiveData::Int(size)).await?;

  let contents = crate::util::open_file(path).await?;
  let mut file_reader = FramedRead::new(contents, BytesCodec::new());
  while let Some(bytes) = file_reader.next().await {
    sink
      .send(ArchiveData::Bytes(bytes.nowhere()?.freeze()))
      .await?;
  }

  if size % 8 > 0 {
    sink
      .send(ArchiveData::Bytes(
        vec![0u8; 8 - (size as usize % 8)].into(),
      ))
      .await?;
  }

  Ok(())
}
