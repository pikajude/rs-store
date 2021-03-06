use self::dirs::Dirs;
use super::ByteStream;
use crate::{
  archive::{ArchiveSink, PathFilter},
  hash::{self, Hash, HashType},
  path::Path as StorePath,
  path_info::{PathInfo, ValidPathInfo},
  prelude::*,
  Store,
};
use crypto::digest::Digest;
use db::Db;
use error::Error;
use futures::{lock::Mutex, TryStreamExt};
use lock::{FsExt2, LockType, PathLocks};
use nix::{
  sys::{stat::*, time::TimeSpec},
  unistd::{fchownat, getegid, geteuid, getuid, FchownatFlags},
};
use std::{
  borrow::Cow,
  collections::{BTreeSet, HashSet},
  iter,
  path::{Path, PathBuf},
  process,
  sync::Arc,
  time::SystemTime,
};
use tokio::{fs, io::AsyncWriteExt};

mod db;
mod dirs;
mod error;
mod gc;
mod lock;

pub struct LocalStore {
  dirs: Dirs,
  db: Mutex<Db>,
}

#[async_trait]
impl Store for LocalStore {
  fn store_path(&self) -> Cow<Path> {
    Cow::Owned(self.dirs.store_dir())
  }

  fn get_uri(&self) -> String {
    String::from("local")
  }

  async fn get_path_info(&self, path: &StorePath) -> Result<Option<Arc<dyn PathInfo>>> {
    // i think i have to destructure here because map() requires Sized
    if let Some(x) = self.db.lock().await.get_path_info(self, path)? {
      Ok(Some(Arc::new(x)))
    } else {
      Ok(None)
    }
  }

  async fn get_referrers(&self, path: &StorePath) -> Result<BTreeSet<StorePath>> {
    self.db.lock().await.get_referrers(self, path)
  }

  async fn add_temp_root(&self, path: &StorePath) -> Result<()> {
    let file = self.dirs.temproots_dir().join(process::id().to_string());
    let mut temp_file = loop {
      let all_gc_roots = gc::open_gc_lock(&self.dirs, LockType::Read)
        .await
        .context("acquiring GC lock")?;
      let _ = fs::remove_file(&file).await;
      let temproots_file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&file)
        .await
        .with_context(|| format!("while opening temproots file {}", file.display()))?;
      drop(all_gc_roots);
      debug!("acquiring read lock on `{}'", file.display());
      temproots_file.lock(LockType::Read).await?;
      if temproots_file.metadata().await?.len() == 0 {
        break temproots_file;
      }
    };
    debug!("acquiring write lock on `{}'", file.display());
    temp_file.lock(LockType::Write).await?;
    temp_file
      .write(self.print_store_path(path).as_bytes())
      .await?;
    temp_file.lock(LockType::Read).await?;
    Ok(())
  }

  async fn add_nar_to_store<S: ByteStream + Send + Unpin>(
    &self,
    info: &ValidPathInfo,
    source: S,
  ) -> Result<()> {
    self
      .add_temp_root(&info.store_path)
      .await
      .with_context(|| {
        format!(
          "while trying to acquire temproot for path {}",
          info.store_path
        )
      })?;

    if !self.is_valid_path(&info.store_path).await? {
      let mut locks = PathLocks::new();
      let real_path = self
        .store_path()
        .join(PathBuf::from(info.store_path.to_string()));

      locks.lock(Some(real_path.clone()), false, None).await?;

      if !self.is_valid_path(&info.store_path).await? {
        let _ = fs::remove_file(&real_path).await;

        let mut hash_sink = hash::Context::new(HashType::SHA256);

        crate::archive::restore_into(
          &real_path,
          source.and_then(|bytes| {
            hash_sink.input(&bytes);
            futures::future::ok(bytes)
          }),
        )
        .await?;

        let (hash, hash_len) = hash_sink.finish();

        if hash != info.nar_hash {
          bail!(Error::NarHashMismatch {
            path: self.print_store_path(&info.store_path).into(),
            expected: info.nar_hash.clone(),
            actual: hash
          });
        }
        if hash_len != info.nar_size.unwrap_or_default() as usize {
          bail!(Error::NarSizeMismatch {
            path: self.print_store_path(&info.store_path).into(),
            expected: info.nar_size.unwrap_or_default() as usize,
            actual: hash_len
          });
        }

        // self.auto_gc().await?;

        self.canonicalise_path_metadata(&real_path, None).await?;

        // self.optimise(&real_path).await?;

        self.db.lock().await.insert_valid_paths(self, Some(info))?;
      }
    }
    Ok(())
  }

  async fn add_path_to_store(
    &self,
    name: &str,
    path: &Path,
    algo: HashType,
    filter: PathFilter,
    _repair: bool,
  ) -> Result<StorePath> {
    let fpath = fs::canonicalize(path).await?;
    let meta = fs::metadata(&fpath).await?;
    let (contents_hash, _) = Hash::hash_file(&fpath, algo).await?;
    let dest =
      self.make_fixed_output_path(meta.is_dir(), &contents_hash, name, iter::empty(), false)?;
    self.add_temp_root(&dest).await?;
    if !self.is_valid_path(&dest).await? {
      let real_path = self.store_path().join(PathBuf::from(dest.to_string()));
      let _ = fs::remove_file(&real_path).await;
      if meta.is_dir() {
        unimplemented!("recursive add not yet supported");
      } else {
        fs::copy(&fpath, &real_path).await.with_context(|| {
          format!(
            "while copying contents from {} to {}",
            fpath.display(),
            real_path.display()
          )
        })?;
      }

      self.canonicalise_path_metadata(&real_path, None).await?;

      let mut h = ArchiveSink::new(crate::hash::Sink::new(algo));
      crate::archive::dump_path(&real_path, &mut h, &filter).await?;
      let (hash, size) = h.into_inner().finish();

      // self.optimise_path(&real_path).await?;

      let vpi = ValidPathInfo {
        store_path: dest.clone(),
        deriver: None,
        nar_hash: hash,
        references: Default::default(),
        registration_time: SystemTime::now(),
        nar_size: Some(size as u64),
        id: 0,
        signatures: Default::default(),
        content_addressed: Default::default(),
        ultimate: true,
      };

      self.db.lock().await.insert_valid_paths(self, Some(&vpi))?;
    }
    Ok(dest)
  }
}

impl LocalStore {
  pub fn open(root: &Path) -> Result<Self> {
    let dirs = Dirs::new(root)?;
    let this = Self {
      db: Mutex::new(Db::open(&dirs.db_dir().join("db.sqlite"), true)?),
      dirs,
    };
    #[cfg(target_os = "linux")]
    this.make_store_writable()?;
    Ok(this)
  }

  #[cfg(target_os = "linux")]
  fn make_store_writable(&self) -> Result<()> {
    use nix::{mount::*, sched::*, sys::statvfs::*};

    if !getuid().is_root() {
      return Ok(());
    }

    let st = statvfs(&self.dirs.store_dir())?;
    if st.flags().contains(FsFlags::ST_RDONLY) {
      unshare(CloneFlags::CLONE_NEWNS)?;

      mount::<Path, Path, str, Path>(
        None,
        &self.dirs.store_dir(),
        Some("none"),
        MsFlags::MS_REMOUNT | MsFlags::MS_BIND,
        None,
      )
      .with_context(|| {
        format!(
          "while trying to remount `{}' as writable",
          self.dirs.store_dir().display()
        )
      })?;
    }
    Ok(())
  }

  async fn canonicalise_path_metadata<P: AsRef<Path> + Send>(
    &self,
    path: P,
    uid: Option<libc::uid_t>,
  ) -> Result<()> {
    self
      .canonicalise_path_metadata_impl(path, uid, &mut HashSet::new())
      .await
  }

  #[async_recursion]
  async fn canonicalise_path_metadata_impl<P: AsRef<Path> + Send>(
    &self,
    path: P,
    uid: Option<libc::uid_t>,
    inodes: &mut HashSet<Inode>,
  ) -> Result<()> {
    use nix::errno::*;
    use std::ffi::CString;

    let path = path.as_ref();
    let path_str = CString::new(path.to_string_lossy().into_owned())?;

    #[cfg(target_os = "macos")]
    {
      extern "C" {
        fn lchflags(path: *const libc::c_char, flags: libc::c_ulong) -> libc::c_int;
      }

      if unsafe { lchflags(path_str.as_ptr(), 0) } != 0 {
        if errno() != libc::ENOTSUP {
          bail!(std::io::Error::last_os_error());
        }
      }
    }

    let info = lstat(path)?;
    if !(s_isreg(info.st_mode) || s_isdir(info.st_mode) || s_islnk(info.st_mode)) {
      bail!("file `{}' has an unsupported type", path.display());
    }

    #[cfg(target_os = "linux")]
    {
      extern "C" {
        fn llistxattr(
          path: *const libc::c_char,
          list: *mut libc::c_char,
          size: libc::size_t,
        ) -> libc::ssize_t;
        fn lremovexattr(path: *const libc::c_char, name: *const libc::c_char) -> libc::c_int;
      }

      use std::cmp::Ordering;

      let attrsize = unsafe { llistxattr(path_str.as_ptr(), std::ptr::null_mut(), 0) };
      match attrsize.cmp(&0) {
        Ordering::Less => {
          if errno() != libc::ENOTSUP && errno() != libc::ENODATA {
            bail!("querying extended attributes of `{}'", path.display());
          }
        }
        Ordering::Greater => {
          let mut attrbuf = vec![0u8; attrsize as usize];
          unsafe {
            llistxattr(
              path_str.as_ptr(),
              attrbuf.as_mut_ptr() as *mut _,
              attrbuf.len(),
            )
          };
          for attr_name in attrbuf.split(|x| *x == 0) {
            if attr_name == b"security.selinux" {
              continue;
            }
            if unsafe { lremovexattr(path_str.as_ptr(), attr_name.as_ptr() as *const _) } == -1 {
              bail!(
                "removing extended attribute `{}' from `{}'",
                String::from_utf8_lossy(attr_name),
                path.display()
              );
            }
          }
        }
        _ => {}
      }
    }

    if uid.map_or(false, |x| x != info.st_uid) {
      assert!(!s_isdir(info.st_mode));
      if !inodes.contains(&Inode {
        dev: info.st_dev,
        ino: info.st_ino,
      }) {
        bail!("invalid ownership on file `{}'", path.display());
      }

      let mode = info.st_mode & !libc::S_IFMT;
      assert!(
        s_islnk(mode)
          || (info.st_uid == getuid().as_raw()
            && (mode == 0o444 || mode == 0o555)
            && info.st_mtime == 1)
      );
    }

    inodes.insert(Inode {
      dev: info.st_dev,
      ino: info.st_ino,
    });

    self.canonicalize_timestamp_and_permissions(path, info)?;

    if info.st_uid != geteuid().as_raw() {
      fchownat(
        None,
        path,
        Some(geteuid()),
        Some(getegid()),
        FchownatFlags::NoFollowSymlink,
      )
      .with_context(|| format!("while changing ownership of path `{}'", path.display()))?;
    }

    if s_isdir(info.st_mode) {
      let mut dir = fs::read_dir(path).await?;
      while let Some(entry) = dir.next_entry().await? {
        self
          .canonicalise_path_metadata_impl(entry.path(), uid, inodes)
          .await?;
      }
    }

    Ok(())
  }

  fn canonicalize_timestamp_and_permissions<P: AsRef<Path>>(
    &self,
    path: P,
    info: FileStat,
  ) -> Result<()> {
    if !s_islnk(info.st_mode) {
      let mut mode = info.st_mode & !libc::S_IFMT;
      if mode != 0o444 && mode != 0o555 {
        mode = (info.st_mode & libc::S_IFMT)
          | 0o444
          | (if info.st_mode & libc::S_IXUSR > 0 {
            0o111
          } else {
            0
          });
        fchmodat(
          None,
          path.as_ref(),
          Mode::from_bits_truncate(mode),
          FchmodatFlags::FollowSymlink,
        )?;
      }
    }

    if info.st_mtime != 1 {
      use nix::sys::time::TimeValLike;
      utimensat(
        None,
        path.as_ref(),
        &TimeSpec::seconds(info.st_atime),
        &TimeSpec::seconds(1),
        UtimensatFlags::NoFollowSymlink,
      )?;
    }

    Ok(())
  }
}

fn s_isreg(mode: mode_t) -> bool {
  (mode & SFlag::S_IFMT.bits()) == SFlag::S_IFREG.bits()
}
fn s_isdir(mode: mode_t) -> bool {
  (mode & SFlag::S_IFMT.bits()) == SFlag::S_IFDIR.bits()
}
fn s_islnk(mode: mode_t) -> bool {
  (mode & SFlag::S_IFMT.bits()) == SFlag::S_IFLNK.bits()
}

#[derive(Eq, PartialEq, Hash)]
struct Inode {
  dev: libc::dev_t,
  ino: libc::ino_t,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::hash::HashType;
  use async_compression::stream::LzmaDecoder;
  use std::mem::ManuallyDrop;

  fn get_local_store() -> anyhow::Result<LocalStore> {
    let temp = ManuallyDrop::new(tempfile::tempdir()?);
    LocalStore::open(temp.as_ref())
  }

  #[test]
  fn direct_add_path() -> anyhow::Result<()> {
    crate::util::run_test(async {
      let store = get_local_store()?;
      let spath = store
        .add_path_to_store(
          "Cargo.toml",
          Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")),
          HashType::SHA256,
          PathFilter::always(),
          false,
        )
        .await?;
      let path_info = store.get_path_info(&spath).await?;
      assert!(path_info.is_some());

      Ok(())
    })
  }

  #[test]
  fn add_nar() -> anyhow::Result<()> {
    crate::util::run_test(async {
      let store = get_local_store()?;
      let nar_stream = LzmaDecoder::new(
        crate::util::stream_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/disnix.nar.xz"))
          .await?,
      );

      store
        .add_nar_to_store(
          &ValidPathInfo {
            id: 0,
            store_path: StorePath::from_base_name(
              "mabn5yhm39lr6kaqfp1b98sd4b8qr5cg-DisnixWebService-0.8",
            )?,
            deriver: None,
            nar_hash: Hash::decode("sha256:0zgkbmzgyas2d5bjv3gads7qw5fn6zf18nszrdxrkpyz5ckk8syw")?,
            references: Default::default(),
            registration_time: SystemTime::now(),
            nar_size: Some(20094232),
            signatures: Default::default(),
            content_addressed: Default::default(),
            ultimate: false,
          },
          nar_stream,
        )
        .await?;

      Ok(())
    })
  }
}
