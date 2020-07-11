use super::{dirs::Dirs, lock::*};
use anyhow::Result;
use tokio::fs::File;

pub async fn open_gc_lock(d: &Dirs, l: LockType) -> Result<File> {
  let gc_lock = d.state_dir().join("gc.lock");
  debug!("acquiring global GC lock at `{}'", gc_lock.display());
  let f = File::create(&gc_lock).await?;
  if !f.try_lock(l)? {
    info!("waiting for the big GC lock at `{}'...", gc_lock.display());
    f.lock(l).await?;
  }
  Ok(f)
}
