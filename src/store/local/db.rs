use crate::{
  hash::{Encoding, Hash},
  path::{Path as StorePath, PathSet},
  path_info::ValidPathInfo,
  prelude::*,
  Store,
};
use rusqlite::{Connection, DatabaseName};
use std::{
  collections::BTreeSet,
  convert::TryInto,
  path::Path,
  time::{Duration, SystemTime},
};

static QUERY_PATH_INFO: &str = "select id, hash, registrationTime, deriver, narSize, ultimate, \
                                sigs, ca from ValidPaths where path = :path";

static QUERY_REFERENCES: &str =
  "select path from Refs join ValidPaths on reference = id where referrer = :referrer";

static QUERY_REFERRERS: &str = "select path from Refs join ValidPaths on referrer = id where \
                                reference = (select id from ValidPaths where path = :path)";

pub static REGISTER_VALID_PATHS: &str =
  "insert into ValidPaths (path, hash, registrationTime, deriver, narSize, ultimate, sigs, ca) \
   values (:path, :hash, :registrationTime, :deriver, :narSize, :ultimate, :sigs, :ca)";

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct Db(Connection);

impl Db {
  pub fn open(path: &Path, create: bool) -> Result<Self> {
    debug!("opening connection to sqlite DB at {}", path.display());
    let mut conn = Connection::open(path)?;
    if log_enabled!(log::Level::Trace) {
      conn.trace(Some(|x| trace!("{}", x)));
    }
    conn.busy_timeout(Duration::from_millis(60 * 60 * 1000))?;
    conn.pragma_update(None, "foreign_keys", &1u8)?;
    conn.pragma_update(None, "synchronous", &"normal")?;
    let cur_mode = conn.pragma_query_value(Some(DatabaseName::Main), "journal_mode", |r| {
      r.get::<_, String>(0)
    })?;
    let new_mode = "wal";
    if cur_mode != new_mode {
      conn.pragma_update(Some(DatabaseName::Main), "journal_mode", &new_mode)?;
    }
    if new_mode == "wal" {
      conn.pragma_update(None, "wal_autocheckpoint", &40000i64)?;
    }
    if create {
      conn.execute_batch(include_str!("schema.sql"))?;
    }
    Ok(Self(conn))
  }

  pub fn get_path_info<S: Store>(
    &self,
    store: &S,
    path: &StorePath,
  ) -> Result<Option<ValidPathInfo>> {
    let canon = store.print_store_path(path);
    let mut stmt0 = self.prepare(QUERY_PATH_INFO)?;

    let mut mvalid = stmt0.query_and_then_named(
      named_params! {":path": canon.as_str()},
      |row| -> Result<ValidPathInfo> {
        let mderiver: Option<String> = row.get("deriver")?;
        Ok(ValidPathInfo {
          id: row.get::<_, i64>("id")?.try_into()?,
          store_path: path.clone(),
          deriver: mderiver
            .map(|x| store.parse_store_path(Path::new(&x)))
            .transpose()?,
          nar_hash: Hash::decode(&row.get::<_, String>("hash")?)?,
          references: PathSet::new(),
          registration_time: SystemTime::UNIX_EPOCH
            + Duration::from_secs(row.get::<_, i64>("registrationTime")?.try_into()?),
          nar_size: Some(row.get::<_, i64>("narSize")?.try_into()?),
          signatures: row
            .get::<_, Option<String>>("sigs")?
            .map_or(BTreeSet::new(), |s| {
              s.split(' ').map(|x| x.to_string()).collect::<BTreeSet<_>>()
            }),
          content_addressed: row.get("ca")?,
          ultimate: row.get::<_, bool>("ultimate")?,
        })
      },
    )?;

    if let Some(mut pinfo) = mvalid.next().transpose()? {
      pinfo.references = self
        .prepare(QUERY_REFERENCES)?
        .query_and_then_named(named_params! {":referrer": pinfo.id as i64}, |row| {
          Ok(store.parse_store_path(Path::new(&row.get::<_, String>(0)?))?)
        })?
        .collect::<Result<_>>()?;

      Ok(Some(pinfo))
    } else {
      Ok(None)
    }
  }

  pub fn get_referrers<S: Store>(&self, store: &S, path: &StorePath) -> Result<PathSet> {
    self
      .prepare(QUERY_REFERRERS)?
      .query_and_then(&[store.print_store_path(path).as_str()], |row| {
        Ok(store.parse_store_path(Path::new(&row.get::<_, String>(0)?))?)
      })?
      .collect::<Result<_>>()
  }

  pub fn insert_valid_paths<'a, S: Store, I: IntoIterator<Item = &'a ValidPathInfo>>(
    &mut self,
    store: &S,
    paths: I,
  ) -> Result<()> {
    let txn = self.transaction()?;
    for path in paths.into_iter() {
      txn.execute_named(
        REGISTER_VALID_PATHS,
        named_params! {
          ":path": store.print_store_path(&path.store_path),
          ":hash": path.nar_hash.encode_with_type(Encoding::Base16),
          ":registrationTime": path.registration_time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64,
          ":deriver": path.deriver.as_ref().map(|r|store.print_store_path(r)),
          ":narSize": path.nar_size.unwrap_or(0) as i64,
          ":ultimate": path.ultimate,
          ":sigs": itertools::join(&path.signatures, " "),
          ":ca": ""
        },
      )?;
      let row_id = txn.last_insert_rowid();
      debug!("inserted new row: {:?}", row_id);
    }
    txn.commit()?;
    Ok(())
  }
}
