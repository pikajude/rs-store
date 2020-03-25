use crate::util::ext::*;
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn root() -> bool {
  nix::unistd::getuid().is_root()
}

#[cfg(not(unix))]
fn root() -> bool {
  false
}

#[derive(serde::Deserialize)]
struct Builtins {
  store_path: &'static str,
  local_state_path: &'static str,
}

impl Builtins {
  fn get() -> Self {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/config.json"))).unwrap()
  }
}

pub struct Settings {
  store_path: PathBuf,
  log_dir: PathBuf,
  state_dir: PathBuf,
  daemon_socket: Option<PathBuf>,
  build_users_group: Option<String>,
  lock_cpu: bool,
  ca_file: PathBuf,
}

impl Settings {
  pub fn get() -> super::error::Result<Self> {
    let bs = Builtins::get();
    let state_dir = lookup(&["NIX_STATE_DIR"], || {
      PathBuf::from(bs.local_state_path).join("nix")
    })?;
    Ok(Self {
      store_path: lookup(&["NIX_STORE_DIR", "NIX_STORE"], || bs.store_path)?,
      log_dir: lookup(&["NIX_LOG_DIR"], || {
        PathBuf::from(bs.local_state_path).join("log/nix")
      })?,
      daemon_socket: lookup(&[], || state_dir.join("daemon-socket/socket")).ok(),
      state_dir,
      build_users_group: if root() { None } else { Some("nixbld".into()) },
      lock_cpu: std::env::var("NIX_AFFINITY_HACK").map_or(false, |y| y == "1"),
      ca_file: lookup(&["NIX_SSL_CERT_FILE", "SSL_CERT_FILE"], || {
        "/etc/ssl/certs/ca-certificates.crt"
      })?,
    })
  }

  /// May be overridden using `NIX_STORE_PATH` or `NIX_STORE`.
  pub fn store_path(&self) -> &Path {
    &self.store_path
  }

  /// May be overridden using `NIX_LOG_DIR`.
  pub fn log_dir(&self) -> &Path {
    &self.log_dir
  }

  /// May be overridden using `NIX_STATE_DIR`.
  pub fn state_dir(&self) -> &Path {
    &self.state_dir
  }

  pub fn daemon_socket(&self) -> Option<&Path> {
    self.daemon_socket.as_deref()
  }

  pub fn build_users_group(&self) -> Option<&str> {
    self.build_users_group.as_deref()
  }

  /// True if `NIX_AFFINITY_HACK` is set to 1.
  pub fn lock_cpu(&self) -> bool {
    self.lock_cpu
  }

  /// May be overridden using `NIX_SSL_CERT_FILE` or `SSL_CERT_FILE`.
  /// Otherwise, the CA file will be searched for in `/etc/ssl`.
  pub fn ca_file(&self) -> &Path {
    &self.ca_file
  }
}

fn lookup<F: FnOnce() -> P, P: AsRef<Path>>(
  vars: &'static [&'static str],
  x: F,
) -> crate::error::Result<PathBuf> {
  for v in vars {
    if let Ok(x) = std::env::var(v) {
      return PathBuf::from(&x).canonicalize().on_path(x);
    }
  }
  let fallback = x();
  PathBuf::from(fallback.as_ref())
    .canonicalize()
    .on_path(fallback)
}
