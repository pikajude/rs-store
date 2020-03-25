use crate::{
  error::*,
  path::StorePath,
  path_info::PathInfo,
  settings::Settings,
  store::Store,
  util::{ext::*, hash::Hash},
};
use chrono::{offset::TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::{
  collections::BTreeSet,
  path::Path,
  str::FromStr,
  sync::{Arc, Mutex},
};

pub struct LocalStore {
  settings: Settings,
  db: Arc<Mutex<Connection>>,
}

static QUERY_PATH_INFO: &str = "SELECT id, hash, registrationTime, deriver, narSize, ultimate, \
                                sigs, ca from ValidPaths where path = ?";
static QUERY_REFERENCES: &str =
  "SELECT path FROM Refs JOIN ValidPaths ON reference = id WHERE referrer = ?";

#[async_trait]
impl Store for LocalStore {
  fn get_uri(&self) -> String {
    "local".into()
  }

  fn store_path(&self) -> &Path {
    self.settings.store_path()
  }

  async fn query_path_info_uncached(&self, path: &StorePath) -> Result<Option<PathInfo>> {
    let db = self.db.nlock()?;
    let mut row = db
      .prepare(QUERY_PATH_INFO)?
      .query_row(params![self.print_path(path)], |r| {
        let id: u32 = r.get(0)?;
        let nar_hash: String = r.get(1)?;
        let timestamp: i64 = r.get(2)?;
        let drv: Option<String> = r.get(3)?;
        let size: Option<u32> = r.get(4)?;
        let ult: Option<i32> = r.get(5)?;
        let sigs: Option<String> = r.get(6)?;
        let ca: Option<String> = r.get(7)?;
        // fake try block
        let pinfo: Result<PathInfo> = (move || {
          Ok(PathInfo {
            path: path.clone(),
            id,
            nar_hash: Hash::from_str(&nar_hash)?,
            registration_time: Utc.timestamp(timestamp, 0),
            deriver: drv.as_deref().map(StorePath::from_path).transpose()?,
            nar_size: size.unwrap_or(0),
            ultimate: ult == Some(1),
            signatures: sigs.map_or(Default::default(), |x| {
              x.split(' ').map(|x| x.to_string()).collect()
            }),
            ca,
            references: Default::default(),
          })
        })();
        Ok(pinfo)
      })
      .optional()?
      .transpose()?;
    if let Some(row_ref) = row.as_mut() {
      let mut stmt = db.prepare(QUERY_REFERENCES)?;
      let all_refs = stmt
        .query_map(params![row_ref.id], |r| r.get::<_, String>(0))?
        .map(|x| x.map_err(Error::Db).and_then(StorePath::from_path))
        .collect::<Result<BTreeSet<_>>>()?;
      row_ref.references = all_refs;
    }
    Ok(row)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[tokio::test]
  async fn test_local_query() -> Result<()> {
    let loc = LocalStore {
      settings: Settings::get()?,
      db: Arc::new(Mutex::new(Connection::open("/nix/var/nix/db/db.sqlite")?)),
    };
    let path = "/nix/store/vaxhh4bg6smwbrid99g62x54y2hk1ph3-rustc-1.41.0";
    let pinfo = loc
      .query_path_info_uncached(&StorePath::from_path(&path)?)
      .await?;
    eprintln!("{:?}", pinfo);
    Ok(())
  }
}
