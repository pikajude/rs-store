use super::{context::Context as C, Hash, HashType};
// use crate::archive::ArchiveData;
use bytes::Bytes;
use crypto::digest::Digest;
use futures::sink::Sink;
use std::{
  convert::Infallible,
  pin::Pin,
  task::{Context, Poll},
};

pub struct HashSink(C);

impl HashSink {
  pub fn new(ty: HashType) -> Self {
    Self(C::new(ty))
  }

  pub fn finish(self) -> (Hash, usize) {
    self.0.finish()
  }
}

impl Sink<Bytes> for HashSink {
  type Error = Infallible;

  fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn start_send(mut self: Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
    self.0.input(&item);
    Ok(())
  }

  fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }
}
