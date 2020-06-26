use super::{context::Context as C, Hash, HashType};
use crate::archive::ArchiveData;
use crypto::digest::Digest;
use futures::sink::Sink;
use std::{
  pin::Pin,
  task::{Context, Poll},
};

pub struct HashSink(C);

impl HashSink {
  pub fn new(ty: HashType) -> Self {
    Self(C::new(ty))
  }

  pub fn finish(self) -> Hash {
    self.0.into()
  }
}

impl Sink<ArchiveData> for HashSink {
  type Error = !;

  fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn start_send(mut self: Pin<&mut Self>, item: ArchiveData) -> Result<(), Self::Error> {
    match item {
      ArchiveData::Bytes(b) => self.0.input(&b),
      ArchiveData::Tag(s) => self.0.input(s.as_bytes()),
      ArchiveData::Int(i) => {
        let mut buf = [0u8; 8];
        #[allow(clippy::needless_range_loop)] // easiest way to write this
        for ix in 0..8 {
          buf[ix] = (i >> (8 * ix)) as u8;
        }
        self.0.input(&buf);
      }
    };
    Ok(())
  }

  fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }
}
