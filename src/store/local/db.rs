use crate::{error::*, hash::Hash, path::Path as StorePath, path_info::ValidPathInfo, Store};
use rusqlite::Connection;
use std::{
  collections::BTreeSet,
  convert::TryInto,
  path::Path,
  time::{Duration, SystemTime},
};

static QUERY_PATH_INFO: &str = "select id, hash, registrationTime, deriver, narSize, ultimate, \
                                sigs, ca from ValidPaths where path = ?";

static QUERY_REFERENCES: &str =
  "select path from Refs join ValidPaths on reference = id where referrer = ?";

#[derive(derive_more::Deref)]
pub struct Db(Connection);

impl Db {
  pub fn open(path: &Path) -> Result<Self> {
    Ok(Self(Connection::open(path)?))
  }

  pub fn get_path_info<S: Store>(
    &self,
    store: &S,
    path: &StorePath,
  ) -> Result<Option<ValidPathInfo>> {
    #[derive(Debug)]
    struct OwnedPathInfo {
      id: i64,
      hash_str: String,
      reg_time: i64,
      deriver: Option<String>,
      nar_size: i64,
      signatures: Option<String>,
      ca: Option<String>,
    }

    let canon = store.print_store_path(path);
    let mut stmt0 = self.prepare(QUERY_PATH_INFO)?;

    let mut mvalid = stmt0.query_and_then(&[canon.as_str()], |row| -> Result<ValidPathInfo> {
      let mderiver: Option<String> = row.get("deriver")?;
      Ok(ValidPathInfo {
        id: row.get::<_, i64>("id")?.try_into()?,
        store_path: path.clone(),
        deriver: mderiver
          .map(|x| store.parse_store_path(Path::new(&x)))
          .transpose()?,
        nar_hash: Hash::decode(&row.get::<_, String>("hash")?)?,
        references: BTreeSet::new(),
        registration_time: SystemTime::UNIX_EPOCH
          + Duration::from_secs(row.get::<_, i64>("registrationTime")?.try_into()?),
        nar_size: row.get::<_, i64>("narSize")?.try_into()?,
        signatures: row
          .get::<_, Option<String>>("sigs")?
          .map_or(BTreeSet::new(), |s| {
            s.split(' ').map(|x| x.to_string()).collect::<BTreeSet<_>>()
          }),
        content_addressed: row.get("ca")?,
      })
    })?;

    if let Some(mut pinfo) = mvalid.next().transpose()? {
      pinfo.references = self
        .prepare(QUERY_REFERENCES)?
        .query_and_then(&[pinfo.id as i64], |row| {
          Ok(store.parse_store_path(Path::new(&row.get::<_, String>(0)?))?)
        })?
        .collect::<Result<_>>()?;

      Ok(Some(pinfo))
    } else {
      Ok(None)
    }
  }
}
