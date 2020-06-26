use super::{context::Context as C, HashType};
use crate::archive::ArchiveData;
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
}

impl Sink<ArchiveData> for HashSink {
  type Error = !;

  fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn start_send(self: Pin<&mut Self>, item: ArchiveData) -> Result<(), Self::Error> {
    todo!()
  }

  fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    todo!()
  }

  fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    todo!()
  }
}
