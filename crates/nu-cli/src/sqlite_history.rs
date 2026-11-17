// Adapted from reedline 0.42.0, Copyright (c) 2021 Nushell Project.
// MIT licensed; see sqlite_history.LICENSE.
use reedline::{
    CommandLineSearch, History, HistoryItem, HistoryItemId, HistorySessionId, ReedlineError,
    SearchDirection, SearchQuery,
};
type Result<T> = std::result::Result<T, ReedlineError>;
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, ToSql, named_params, params};
use std::{path::PathBuf, time::Duration};
const SQLITE_APPLICATION_ID: i32 = 1151497937;

/// A history that stores the values to an SQLite database.
/// In addition to storing the command, the history can store an additional arbitrary HistoryEntryContext,
/// to add information such as a timestamp, running directory, result...
///
/// Uses Nushell’s rusqlite dependency independently of reedline.
pub struct SqliteBackedHistory {
    db: rusqlite::Connection,
    session: Option<HistorySessionId>,
    session_timestamp: Option<chrono::DateTime<Utc>>,
}

fn deserialize_history_item(row: &rusqlite::Row) -> rusqlite::Result<HistoryItem> {
    let x: Option<String> = row.get("more_info")?;
    Ok(HistoryItem {
        id: Some(HistoryItemId::new(row.get("id")?)),
        start_timestamp: row.get::<&str, Option<i64>>("start_timestamp")?.map(|e| {
            match Utc.timestamp_millis_opt(e) {
                chrono::LocalResult::Single(e) => e,
                _ => chrono::Utc::now(),
            }
        }),
        command_line: row.get("command_line")?,
        session_id: row
            .get::<&str, Option<i64>>("session_id")?
            .map(|id| serde_json::from_value(serde_json::json!(id)).expect("integer session ID")),
        hostname: row.get("hostname")?,
        cwd: row.get("cwd")?,
        duration: row
            .get::<&str, Option<i64>>("duration_ms")?
            .map(|e| Duration::from_millis(e as u64)),
        exit_status: row.get("exit_status")?,
        more_info: x
            .map(|x| {
                serde_json::from_str(&x).map_err(|e| {
                    // hack
                    rusqlite::Error::InvalidColumnType(
                        0,
                        format!("could not deserialize more_info: {e}"),
                        rusqlite::types::Type::Text,
                    )
                })
            })
            .transpose()?,
    })
}

impl History for SqliteBackedHistory {
    fn save(&mut self, mut entry: HistoryItem) -> Result<HistoryItem> {
        let ret: i64 = self
            .db
            .prepare(
                "insert into history
                               (id,  start_timestamp,  command_line,  session_id,  hostname,  cwd,  duration_ms,  exit_status,  more_info)
                        values (:id, :start_timestamp, :command_line, :session_id, :hostname, :cwd, :duration_ms, :exit_status, :more_info)
                    on conflict (history.id) do update set
                        start_timestamp = excluded.start_timestamp,
                        command_line = excluded.command_line,
                        session_id = excluded.session_id,
                        hostname = excluded.hostname,
                        cwd = excluded.cwd,
                        duration_ms = excluded.duration_ms,
                        exit_status = excluded.exit_status,
                        more_info = excluded.more_info
                    returning id",
            )
            .map_err(map_sqlite_err)?
            .query_row(
                named_params! {
                    ":id": entry.id.map(|id| id.0),
                    ":start_timestamp": entry.start_timestamp.map(|e| e.timestamp_millis()),
                    ":command_line": entry.command_line,
                    ":session_id": entry.session_id.map(i64::from),
                    ":hostname": entry.hostname,
                    ":cwd": entry.cwd,
                    ":duration_ms": entry.duration.map(|e| e.as_millis() as i64),
                    ":exit_status": entry.exit_status,
                    ":more_info": entry.more_info.as_ref().map(|e| serde_json::to_string(e).unwrap())
                },
                |row| row.get(0),
            )
            .map_err(map_sqlite_err)?;
        entry.id = Some(HistoryItemId::new(ret));
        Ok(entry)
    }

    fn load(&self, id: HistoryItemId) -> Result<HistoryItem> {
        let entry = self
            .db
            .prepare("select * from history where id = :id")
            .map_err(map_sqlite_err)?
            .query_row(named_params! { ":id": id.0 }, deserialize_history_item)
            .map_err(map_sqlite_err)?;
        Ok(entry)
    }

    fn count(&self, query: SearchQuery) -> Result<i64> {
        let (query, params) = self.construct_query(&query, "coalesce(count(*), 0)");
        let params_borrow: Vec<(&str, &dyn ToSql)> = params.iter().map(|e| (e.0, &*e.1)).collect();
        let result: i64 = self
            .db
            .prepare(&query)
            .map_err(map_sqlite_err)?
            .query_row(&params_borrow[..], |r| r.get(0))
            .map_err(map_sqlite_err)?;
        Ok(result)
    }

    fn search(&self, query: SearchQuery) -> Result<Vec<HistoryItem>> {
        let (query, params) = self.construct_query(&query, "*");
        let params_borrow: Vec<(&str, &dyn ToSql)> = params.iter().map(|e| (e.0, &*e.1)).collect();
        let results: Vec<HistoryItem> = self
            .db
            .prepare(&query)
            .map_err(map_sqlite_err)?
            .query_map(&params_borrow[..], deserialize_history_item)
            .map_err(map_sqlite_err)?
            .collect::<rusqlite::Result<Vec<HistoryItem>>>()
            .map_err(map_sqlite_err)?;
        Ok(results)
    }

    fn update(
        &mut self,
        id: HistoryItemId,
        updater: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> Result<()> {
        // in theory this should run in a transaction
        let item = self.load(id)?;
        self.save(updater(item))?;
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.db
            .execute("delete from history", params![])
            .map_err(map_sqlite_err)?;

        // VACUUM to ensure that sensitive data is completely erased
        // instead of being marked as available for reuse
        self.db
            .execute("VACUUM", params![])
            .map_err(map_sqlite_err)?;

        Ok(())
    }

    fn delete(&mut self, h: HistoryItemId) -> Result<()> {
        let changed = self
            .db
            .execute("delete from history where id = ?", params![h.0])
            .map_err(map_sqlite_err)?;
        if changed == 0 {
            return Err(history_error("Could not find item".to_string()));
        }
        Ok(())
    }

    fn sync(&mut self) -> std::io::Result<()> {
        // no-op (todo?)
        Ok(())
    }

    fn session(&self) -> Option<HistorySessionId> {
        self.session
    }
}
fn map_sqlite_err(err: rusqlite::Error) -> ReedlineError {
    // TODO: better error mapping
    history_error(format!("{err:?}"))
}

type BoxedNamedParams<'a> = Vec<(&'static str, Box<dyn ToSql + 'a>)>;

impl SqliteBackedHistory {
    /// Creates a new history with an associated history file.
    ///
    ///
    /// **Side effects:** creates all nested directories to the file
    ///
    pub fn with_file(
        file: PathBuf,
        session: Option<HistorySessionId>,
        session_timestamp: Option<chrono::DateTime<Utc>>,
    ) -> Result<Self> {
        if let Some(base_dir) = file.parent() {
            std::fs::create_dir_all(base_dir).map_err(|e| history_error(format!("{e}")))?;
        }
        let db = Connection::open(&file).map_err(map_sqlite_err)?;
        Self::from_connection(db, session, session_timestamp)
    }
    /// Creates a new history in memory
    pub fn in_memory() -> Result<Self> {
        Self::from_connection(
            Connection::open_in_memory().map_err(map_sqlite_err)?,
            None,
            None,
        )
    }
    /// initialize a new database / migrate an existing one
    fn from_connection(
        db: Connection,
        session: Option<HistorySessionId>,
        session_timestamp: Option<chrono::DateTime<Utc>>,
    ) -> Result<Self> {
        // https://phiresky.github.io/blog/2020/sqlite-performance-tuning/
        db.pragma_update(None, "journal_mode", "wal")
            .map_err(map_sqlite_err)?;
        db.pragma_update(None, "synchronous", "normal")
            .map_err(map_sqlite_err)?;
        db.pragma_update(None, "mmap_size", "1000000000")
            .map_err(map_sqlite_err)?;
        db.pragma_update(None, "foreign_keys", "on")
            .map_err(map_sqlite_err)?;
        db.pragma_update(None, "application_id", SQLITE_APPLICATION_ID)
            .map_err(map_sqlite_err)?;
        let db_version: i32 = db
            .query_row(
                "SELECT user_version FROM pragma_user_version",
                params![],
                |r| r.get(0),
            )
            .map_err(map_sqlite_err)?;
        if db_version != 0 {
            return Err(history_error(format!(
                "Unknown database version {db_version}"
            )));
        }
        db.execute_batch(
            "
        create table if not exists history (
            id integer primary key autoincrement,
            command_line text not null,
            start_timestamp integer,
            session_id integer,
            hostname text,
            cwd text,
            duration_ms integer,
            exit_status integer,
            more_info text
        ) strict;
        create index if not exists idx_history_time on history(start_timestamp);
        create index if not exists idx_history_cwd on history(cwd); -- suboptimal for many hosts
        create index if not exists idx_history_exit_status on history(exit_status);
        create index if not exists idx_history_cmd on history(command_line);
        create index if not exists idx_history_cmd on history(session_id);
        -- todo: better indexes
        ",
        )
        .map_err(map_sqlite_err)?;
        Ok(SqliteBackedHistory {
            db,
            session,
            session_timestamp,
        })
    }

    fn construct_query<'a>(
        &self,
        query: &'a SearchQuery,
        select_expression: &str,
    ) -> (String, BoxedNamedParams<'a>) {
        // TODO: this whole function could be done with less allocs
        let (is_asc, asc) = match query.direction {
            SearchDirection::Forward => (true, "asc"),
            SearchDirection::Backward => (false, "desc"),
        };
        let mut wheres = Vec::new();
        let mut params: BoxedNamedParams = Vec::new();
        if let Some(start) = query.start_time {
            wheres.push(if is_asc {
                "start_timestamp > :start_time"
            } else {
                "start_timestamp < :start_time"
            });
            params.push((":start_time", Box::new(start.timestamp_millis())));
        }
        if let Some(end) = query.end_time {
            wheres.push(if is_asc {
                ":end_time >= start_timestamp"
            } else {
                ":end_time <= start_timestamp"
            });
            params.push((":end_time", Box::new(end.timestamp_millis())));
        }
        if let Some(start) = query.start_id {
            wheres.push(if is_asc {
                "id > :start_id"
            } else {
                "id < :start_id"
            });
            params.push((":start_id", Box::new(start.0)));
        }
        if let Some(end) = query.end_id {
            wheres.push(if is_asc {
                ":end_id >= id"
            } else {
                ":end_id <= id"
            });
            params.push((":end_id", Box::new(end.0)));
        }
        let limit = match query.limit {
            Some(l) => {
                params.push((":limit", Box::new(l)));
                "limit :limit"
            }
            None => "",
        };
        if let Some(command_line) = &query.filter.command_line {
            match command_line {
                CommandLineSearch::Exact(e) => {
                    wheres.push("command_line == :command_line");
                    params.push((":command_line", Box::new(e)));
                }
                CommandLineSearch::Prefix(prefix) => {
                    wheres.push("instr(command_line, :command_line) == 1");
                    params.push((":command_line", Box::new(prefix)));
                }
                CommandLineSearch::Substring(cont) => {
                    wheres.push("instr(command_line, :command_line) >= 1");
                    params.push((":command_line", Box::new(cont)));
                }
            };
        }

        if let Some(str) = query.filter.excluded_command_line() {
            wheres.push("command_line != :not_cmd");
            params.push((":not_cmd", Box::new(str)));
        }
        if let Some(hostname) = &query.filter.hostname {
            wheres.push("hostname = :hostname");
            params.push((":hostname", Box::new(hostname)));
        }
        if let Some(cwd_exact) = &query.filter.cwd_exact {
            wheres.push("cwd = :cwd");
            params.push((":cwd", Box::new(cwd_exact)));
        }
        if let Some(cwd_prefix) = &query.filter.cwd_prefix {
            wheres.push("cwd like :cwd_like");
            let cwd_like = format!("{cwd_prefix}%");
            params.push((":cwd_like", Box::new(cwd_like)));
        }
        if let Some(exit_successful) = query.filter.exit_successful {
            if exit_successful {
                wheres.push("exit_status = 0");
            } else {
                wheres.push("exit_status != 0");
            }
        }
        if let (Some(session_id), Some(session_timestamp)) =
            (query.filter.session, self.session_timestamp)
        {
            // Filter so that we get rows:
            // - that have the same session_id, or
            // - were executed before our session started
            wheres.push("(session_id = :session_id OR start_timestamp < :session_timestamp)");
            params.push((":session_id", Box::new(i64::from(session_id))));
            params.push((
                ":session_timestamp",
                Box::new(session_timestamp.timestamp_millis()),
            ));
        }
        let mut wheres = wheres.join(" and ");
        if wheres.is_empty() {
            wheres = "true".to_string();
        }
        let query = format!(
            "SELECT {select_expression} \
             FROM history \
             WHERE ({wheres}) \
             ORDER BY id {asc} \
             {limit}"
        );
        (query, params)
    }
}

fn history_error(message: String) -> ReedlineError {
    std::io::Error::other(message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn all() -> SearchQuery {
        SearchQuery::everything(SearchDirection::Forward, None)
    }
    fn session(id: i64) -> HistorySessionId {
        serde_json::from_value(serde_json::json!(id)).unwrap()
    }
    fn item(cmd: &str, time: i64, sid: i64) -> HistoryItem {
        HistoryItem {
            id: None,
            command_line: cmd.into(),
            start_timestamp: Some(Utc.timestamp_millis_opt(time).unwrap()),
            session_id: Some(session(sid)),
            hostname: Some("host".into()),
            cwd: Some("/work/sub".into()),
            duration: Some(Duration::from_millis(25)),
            exit_status: Some(0),
            more_info: Some(Default::default()),
        }
    }
    #[test]
    fn persistence_filters_and_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        let mut h = SqliteBackedHistory::with_file(path.clone(), None, None).unwrap();
        assert_eq!(
            h.db.query_row("PRAGMA application_id", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1151497937
        );
        let a = h.save(item("echo alpha", 1000, 1)).unwrap();
        let b = h.save(item("echo beta", 2000, 2)).unwrap();
        let mut failed = item("fail", 3000, 2);
        failed.exit_status = Some(1);
        let c = h.save(failed).unwrap();
        h.sync().unwrap();
        drop(h);
        let mut h = SqliteBackedHistory::with_file(path, None, None).unwrap();
        assert_eq!(h.load(a.id.unwrap()).unwrap(), a);
        assert_eq!(h.count(all()).unwrap(), 3);
        let mut q = all();
        q.direction = SearchDirection::Backward;
        q.limit = Some(2);
        assert_eq!(h.search(q).unwrap(), vec![c.clone(), b.clone()]);
        for search in [
            CommandLineSearch::Exact("echo beta".into()),
            CommandLineSearch::Prefix("echo b".into()),
            CommandLineSearch::Substring("beta".into()),
        ] {
            let mut q = all();
            q.filter.command_line = Some(search);
            q.filter.hostname = Some("host".into());
            q.filter.cwd_exact = Some("/work/sub".into());
            q.filter.cwd_prefix = Some("/work".into());
            q.filter.exit_successful = Some(true);
            assert_eq!(h.search(q).unwrap(), vec![b.clone()]);
        }
        let mut q = all();
        q.filter.hostname = Some("other".into());
        assert!(h.search(q).unwrap().is_empty());
        let mut q = all();
        q.filter.exit_successful = Some(false);
        assert_eq!(h.search(q).unwrap(), vec![c.clone()]);
        let mut q = all();
        q.start_id = a.id;
        q.end_id = b.id;
        assert_eq!(h.search(q).unwrap(), vec![b.clone()]);
        h.update(b.id.unwrap(), &|mut e| {
            e.command_line = "changed".into();
            e
        })
        .unwrap();
        assert_eq!(h.load(b.id.unwrap()).unwrap().command_line, "changed");
        h.delete(a.id.unwrap()).unwrap();
        assert!(h.delete(a.id.unwrap()).is_err());
        h.clear().unwrap();
        assert_eq!(h.count(all()).unwrap(), 0);
    }
    #[test]
    fn timestamps_and_sessions() {
        let mut h = SqliteBackedHistory::in_memory().unwrap();
        h.session = Some(session(1));
        h.session_timestamp = Some(Utc.timestamp_millis_opt(2000).unwrap());
        let old = h.save(item("old", 1000, 2)).unwrap();
        let own = h.save(item("own", 3000, 1)).unwrap();
        h.save(item("concurrent", 4000, 2)).unwrap();
        assert_eq!(h.session(), Some(session(1)));
        let q = SearchQuery::everything(SearchDirection::Forward, h.session());
        assert_eq!(h.search(q).unwrap(), vec![old, own.clone()]);
        let mut q = all();
        q.start_time = Some(Utc.timestamp_millis_opt(1000).unwrap());
        q.end_time = Some(Utc.timestamp_millis_opt(3000).unwrap());
        assert_eq!(h.search(q).unwrap(), vec![own.clone()]);
        let mut q = all();
        q.direction = SearchDirection::Backward;
        q.start_time = Some(Utc.timestamp_millis_opt(4000).unwrap());
        q.end_time = Some(Utc.timestamp_millis_opt(3000).unwrap());
        assert_eq!(h.search(q).unwrap(), vec![own]);
    }
    #[test]
    fn previous_format_and_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let db = Connection::open(&path).unwrap();
        // Independent fixture using the on-disk reedline 0.42 schema.
        db.execute_batch(
            "PRAGMA application_id=1151497937;
          CREATE TABLE history (id INTEGER PRIMARY KEY AUTOINCREMENT, command_line TEXT NOT NULL,
          start_timestamp INTEGER, session_id INTEGER, hostname TEXT, cwd TEXT,
          duration_ms INTEGER, exit_status INTEGER, more_info TEXT) STRICT;
          INSERT INTO history VALUES (7,'legacy',1000,42,'old-host','/old',25,0,'null');",
        )
        .unwrap();
        drop(db);
        let mut h = SqliteBackedHistory::with_file(path, None, None).unwrap();
        let old = h.load(HistoryItemId::new(7)).unwrap();
        assert_eq!(old.command_line, "legacy");
        assert_eq!(old.session_id, Some(session(42)));
        assert!(old.more_info.is_some());
        assert_eq!(
            h.save(item("new", 2000, 1)).unwrap().id,
            Some(HistoryItemId::new(8))
        );
        let app: i64 =
            h.db.query_row("PRAGMA application_id", [], |r| r.get(0))
                .unwrap();
        assert_eq!(app, SQLITE_APPLICATION_ID as i64);
        let columns: Vec<String> =
            h.db.prepare("PRAGMA table_info(history)")
                .unwrap()
                .query_map([], |r| r.get(1))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
        assert_eq!(
            columns,
            [
                "id",
                "command_line",
                "start_timestamp",
                "session_id",
                "hostname",
                "cwd",
                "duration_ms",
                "exit_status",
                "more_info"
            ]
        );
        let indices: i64 = h.db.query_row("SELECT count(*) FROM sqlite_master WHERE type='index' AND name IN ('idx_history_time','idx_history_cwd','idx_history_exit_status','idx_history_cmd')", [], |r| r.get(0)).unwrap();
        assert_eq!(indices, 4);
        for (pragma, expected) in [("journal_mode", "wal")] {
            assert_eq!(
                h.db.query_row(&format!("PRAGMA {pragma}"), [], |r| r.get::<_, String>(0))
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            h.db.query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            h.db.query_row("PRAGMA mmap_size", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1_000_000_000
        );
        h.db.pragma_update(None, "user_version", 1).unwrap();
        let db = h.db;
        assert!(SqliteBackedHistory::from_connection(db, None, None).is_err());
    }
}
