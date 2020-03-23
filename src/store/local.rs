use crate::{
  error::*,
  path::StorePath,
  path_info::PathInfo,
  settings::Settings,
  store::Store,
  util::{hash::Hash, mutex::*},
};
use chrono::{offset::TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::{
  path::{Path, PathBuf},
  str::FromStr,
  sync::{Arc, Mutex},
  time::SystemTime,
};

pub struct LocalStore {
  settings: Settings,
  db: Arc<Mutex<Connection>>,
}

#[async_trait]
impl Store for LocalStore {
  fn get_uri(&self) -> String {
    "local".into()
  }

  fn store_path(&self) -> &Path {
    self.settings.store_path()
  }

  async fn query_path_info(&self, path: &StorePath) -> Result<Option<PathInfo>> {
    let db = self.db.nlock()?;
    let mut stmt = db.prepare(
      "SELECT id, hash, registrationTime, deriver, narSize, ultimate, sigs, ca from ValidPaths \
       where path = ?1",
    )?;
    let row = stmt
      .query_row(params![path.to_string()], |r| {
        let id: u32 = r.get(0)?;
        let nar_hash: String = r.get(1)?;
        let timestamp: i64 = r.get(2)?;
        let drv: Option<String> = r.get(3)?;
        let size: Option<u32> = r.get(4)?;
        let ult: Option<i32> = r.get(5)?;
        let sigs: Option<String> = r.get(6)?;
        let ca: Option<String> = r.get(7)?;
        let pinfo: Result<PathInfo> = try {
          PathInfo {
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
          }
        };
        Ok(pinfo)
      })
      .optional()?
      .transpose()?;
    eprintln!("{:?}", row);
    unimplemented!()
  }
}
