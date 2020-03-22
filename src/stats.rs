pub use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Stats {
  pub info_read: AtomicU64,
  pub info_read_averted: AtomicU64,
  pub info_missing: AtomicU64,
  pub info_write: AtomicU64,
  pub path_info_cache_size: AtomicU64,
  pub read: AtomicU64,
  pub read_bytes: AtomicU64,
  pub read_compressed_bytes: AtomicU64,
  pub write: AtomicU64,
  pub write_averted: AtomicU64,
  pub write_bytes: AtomicU64,
  pub write_compressed_bytes: AtomicU64,
  pub write_compression_time_ms: AtomicU64,
}
