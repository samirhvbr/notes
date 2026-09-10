//! SQLite persistence. Workspace traversal and note I/O belong to notes-core.
use notes_model::CoreError;
use rusqlite::{params, Connection, OptionalExtension};
use std::{collections::BTreeMap, path::Path, time::Duration};

type Result<T> = std::result::Result<T, CoreError>;
fn sql(e: rusqlite::Error) -> CoreError {
    CoreError::Internal {
        message: format!("SQLite: {e}"),
    }
}

fn open(path: &Path, schema: &str, supported: u32) -> Result<Connection> {
    let mut db = Connection::open(path).map_err(sql)?;
    db.busy_timeout(Duration::from_secs(5)).map_err(sql)?;
    let found: u32 = db
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(sql)?;
    if found > supported {
        return Err(CoreError::SchemaAhead {
            store: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            found,
            supported,
        });
    }
    db.pragma_update(None, "journal_mode", "WAL").map_err(sql)?;
    db.pragma_update(None, "synchronous", "NORMAL")
        .map_err(sql)?;
    db.pragma_update(None, "foreign_keys", "ON").map_err(sql)?;
    if found < supported {
        let tx = db.transaction().map_err(sql)?;
        tx.execute_batch(schema).map_err(sql)?;
        tx.pragma_update(None, "user_version", supported)
            .map_err(sql)?;
        tx.commit().map_err(sql)?;
    }
    Ok(db)
}

/// Operational identity snapshot, kept in a separate database from derived data.
/// The serialized shape is the existing versioned registry, preserving all fields.
pub struct RegistryStore(Connection);
impl RegistryStore {
    pub fn open(path: &Path) -> Result<Self> {
        open(
            path,
            "CREATE TABLE registry (id INTEGER PRIMARY KEY CHECK(id=1), payload BLOB NOT NULL);",
            1,
        )
        .map(Self)
    }
    pub fn read(&self) -> Result<Option<Vec<u8>>> {
        self.0
            .query_row("SELECT payload FROM registry WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(sql)
    }
    pub fn write_merged(&mut self, baseline: Option<&[u8]>, bytes: &[u8]) -> Result<()> {
        let tx = self
            .0
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sql)?;
        let current: Option<Vec<u8>> = tx
            .query_row("SELECT payload FROM registry WHERE id=1", [], |r| r.get(0))
            .optional()
            .map_err(sql)?;
        let parse = |b: &[u8]| {
            serde_json::from_slice::<serde_json::Value>(b).map_err(|e| CoreError::Internal {
                message: format!("registry data: {e}"),
            })
        };
        let wanted = parse(bytes)?;
        let base = baseline
            .map(parse)
            .transpose()?
            .unwrap_or_else(|| serde_json::json!({}));
        let mut merged = current
            .as_deref()
            .map(parse)
            .transpose()?
            .unwrap_or_else(|| serde_json::json!({}));
        if current.is_none() {
            merged = wanted;
        } else {
            merge_object(&mut merged, &base, &wanted, true)?;
        }
        let mut paths = std::collections::BTreeSet::new();
        if let Some(notes) = merged.get("notes").and_then(|v| v.as_object()) {
            for note in notes.values() {
                if let Some(path) = note.get("path").and_then(|v| v.as_str()) {
                    if !paths.insert(path) {
                        return Err(CoreError::Unsupported {
                            cap: "concurrent identity assignment; reopen the workspace".into(),
                        });
                    }
                }
            }
        }
        let payload = serde_json::to_vec(&merged).map_err(|e| CoreError::Internal {
            message: e.to_string(),
        })?;
        tx.execute("INSERT INTO registry VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",[payload]).map_err(sql)?;
        tx.commit().map_err(sql)
    }
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let tx = self.0.transaction().map_err(sql)?;
        tx.execute("INSERT INTO registry VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload", [bytes]).map_err(sql)?;
        tx.commit().map_err(sql)
    }
}

pub struct Seen {
    pub path: String,
    pub size: u64,
    pub mtime: String,
}
pub struct Cached {
    pub size: u64,
    pub mtime: String,
    pub hash: String,
}
pub struct Indexed<'a> {
    pub seen: &'a Seen,
    pub hash: &'a str,
    pub text: &'a str,
}
pub struct WordHit {
    pub path: String,
    pub line: u32,
    pub col: u32,
    pub context: String,
}
pub struct Index(Connection);
impl Index {
    pub fn open(path: &Path) -> Result<Self> {
        open(path, "DROP TABLE IF EXISTS notes; DROP TABLE IF EXISTS fts; CREATE TABLE notes(path TEXT PRIMARY KEY, size INTEGER NOT NULL, mtime TEXT NOT NULL, hash TEXT NOT NULL, document TEXT NOT NULL);
            CREATE VIRTUAL TABLE fts USING fts5(path UNINDEXED, text, tokenize='unicode61 remove_diacritics 2');", 2).map(Self)
    }
    pub fn documents(&self) -> Result<Vec<(String, notes_markdown::Document)>> {
        let mut q = self
            .0
            .prepare("SELECT path,document FROM notes ORDER BY path")
            .map_err(sql)?;
        let rows = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(sql)?;
        rows.map(|row| {
            let (p, d) = row.map_err(sql)?;
            let d = serde_json::from_str(&d).map_err(|e| CoreError::Internal {
                message: format!("index document: {e}"),
            })?;
            Ok((p, d))
        })
        .collect()
    }
    pub fn plan(&self) -> Result<BTreeMap<String, Cached>> {
        let mut q = self
            .0
            .prepare("SELECT path,size,mtime,hash FROM notes")
            .map_err(sql)?;
        let rows = q
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    Cached {
                        size: r.get(1)?,
                        mtime: r.get(2)?,
                        hash: r.get(3)?,
                    },
                ))
            })
            .map_err(sql)?
            .collect::<std::result::Result<_, _>>()
            .map_err(sql);
        rows
    }
    pub fn apply(&mut self, note: Indexed<'_>, reparse: bool) -> Result<()> {
        let tx = self.0.transaction().map_err(sql)?;
        if reparse {
            let document =
                serde_json::to_string(&notes_markdown::parse(note.text)).map_err(|e| {
                    CoreError::Internal {
                        message: e.to_string(),
                    }
                })?;
            tx.execute("INSERT INTO notes VALUES(?1,?2,?3,?4,?5) ON CONFLICT(path) DO UPDATE SET size=excluded.size,mtime=excluded.mtime,hash=excluded.hash,document=excluded.document", params![note.seen.path,note.seen.size,note.seen.mtime,note.hash,document]).map_err(sql)?;
            tx.execute("DELETE FROM fts WHERE path=?1", [&note.seen.path])
                .map_err(sql)?;
            tx.execute(
                "INSERT INTO fts VALUES(?1,?2)",
                params![note.seen.path, note.text],
            )
            .map_err(sql)?;
        } else {
            tx.execute(
                "UPDATE notes SET size=?2,mtime=?3 WHERE path=?1",
                params![note.seen.path, note.seen.size, note.seen.mtime],
            )
            .map_err(sql)?;
        }
        tx.commit().map_err(sql)
    }
    pub fn remove(&mut self, paths: &[String]) -> Result<()> {
        let tx = self.0.transaction().map_err(sql)?;
        for p in paths {
            tx.execute("DELETE FROM notes WHERE path=?1", [p])
                .map_err(sql)?;
            tx.execute("DELETE FROM fts WHERE path=?1", [p])
                .map_err(sql)?;
        }
        tx.commit().map_err(sql)
    }
    pub fn words(&self, query: &str, limit: usize) -> Result<Vec<WordHit>> {
        let terms: Vec<_> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{s}\""))
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.0.prepare("SELECT path,highlight(fts,1,char(1),char(2)) FROM fts WHERE fts MATCH ?1 ORDER BY rank,path LIMIT ?2").map_err(sql)?;
        let rows = stmt
            .query_map(params![terms.join(" AND "), limit as u64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(sql)?;
        rows.map(|row| {
            let (path, text) = row.map_err(sql)?;
            let first = text.find('\u{1}').unwrap_or(0);
            let before = &text[..first];
            let line = before.bytes().filter(|b| *b == b'\n').count() as u32 + 1;
            let col = before.rsplit('\n').next().unwrap_or("").chars().count() as u32 + 1;
            let context = text
                .lines()
                .nth(line as usize - 1)
                .unwrap_or("")
                .replace(['\u{1}', '\u{2}'], "")
                .chars()
                .take(240)
                .collect();
            Ok(WordHit {
                path,
                line,
                col,
                context,
            })
        })
        .collect()
    }
}

fn merge_object(
    current: &mut serde_json::Value,
    base: &serde_json::Value,
    wanted: &serde_json::Value,
    top: bool,
) -> Result<()> {
    let empty = serde_json::Map::new();
    let a = base.as_object().unwrap_or(&empty);
    let b = wanted.as_object().unwrap_or(&empty);
    let keys = a
        .keys()
        .chain(b.keys())
        .collect::<std::collections::BTreeSet<_>>();
    let object = current.as_object_mut().ok_or_else(|| CoreError::Internal {
        message: "registry is not an object".into(),
    })?;
    for key in keys {
        if a.get(key) == b.get(key) {
            continue;
        }
        if top && key == "notes" {
            let target = object
                .entry(key.clone())
                .or_insert_with(|| serde_json::json!({}));
            merge_object(
                target,
                a.get(key).unwrap_or(&serde_json::Value::Null),
                b.get(key).unwrap_or(&serde_json::Value::Null),
                false,
            )?;
        } else {
            if object.get(key) != a.get(key) && object.get(key) != b.get(key) {
                return Err(CoreError::Unsupported {
                    cap: "identity registry changed in another process; reopen the workspace"
                        .into(),
                });
            }
            match b.get(key) {
                Some(value) => {
                    object.insert(key.clone(), value.clone());
                }
                None => {
                    object.remove(key);
                }
            }
        }
    }
    Ok(())
}
