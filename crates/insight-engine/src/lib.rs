//! Snapshot storage (ADR-0038, R2-30, R3-G #8/#9).
//!
//! A snapshot is written into a temporary file, marked complete, then renamed
//! into place, so a crash can never leave a half-written database that looks
//! valid (R3-G #9). Receipts store spans only; file bytes live in `files`
//! (R3-G #8).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_model::{Diagnostic, FileFact, NodeFact, NodeKind, Receipt, Severity, Span};
pub mod query;
pub use rusqlite;
use rusqlite::{Connection, params};

/// Bumped whenever the table layout changes; recorded in `meta`.
pub const DB_LAYOUT_VERSION: u32 = 4;

const SCHEMA_SQL: &str = r#"
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (
    path TEXT PRIMARY KEY,
    lang TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    content BLOB NOT NULL
);
CREATE TABLE nodes (
    nid INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    parent_nid INTEGER REFERENCES nodes(nid),
    kind TEXT NOT NULL,
    subkind TEXT,
    name TEXT NOT NULL,
    anonymous INTEGER NOT NULL DEFAULT 0,
    lang TEXT,
    file TEXT,
    byte_start INTEGER,
    byte_end INTEGER,
    line_start INTEGER,
    line_end INTEGER
);
CREATE TABLE edges (
    eid INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    subkind TEXT,
    from_nid INTEGER NOT NULL REFERENCES nodes(nid),
    to_nid INTEGER REFERENCES nodes(nid),
    resolution TEXT NOT NULL,
    group_id INTEGER
);
CREATE TABLE receipts (
    eid INTEGER NOT NULL REFERENCES edges(eid),
    role TEXT NOT NULL,
    file TEXT NOT NULL,
    byte_start INTEGER NOT NULL,
    byte_end INTEGER NOT NULL,
    line INTEGER NOT NULL,
    col INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_col INTEGER NOT NULL,
    container_nid INTEGER REFERENCES nodes(nid)
);
CREATE TABLE tags (node_nid INTEGER NOT NULL REFERENCES nodes(nid), tag TEXT NOT NULL);
CREATE TABLE diagnostics (file TEXT, severity TEXT NOT NULL, message TEXT NOT NULL);
"#;

/// Indexes are created after the bulk insert (faster loads).
const INDEX_SQL: &str = r#"
CREATE INDEX nodes_parent ON nodes(parent_nid);
CREATE INDEX nodes_file ON nodes(file);
CREATE INDEX nodes_kind ON nodes(kind);
CREATE INDEX nodes_name ON nodes(name);
CREATE INDEX edges_from ON edges(from_nid);
CREATE INDEX edges_to ON edges(to_nid);
CREATE INDEX receipts_eid ON receipts(eid);
"#;

/// A named declaration (see [`SnapshotWriter::named_declarations`]).
pub struct Decl {
    pub nid: i64,
    pub path: String,
    pub file: String,
    pub byte_start: u64,
    pub byte_end: u64,
}

/// Writes one codebase snapshot. Call [`SnapshotWriter::finish`] to publish it.
pub struct SnapshotWriter {
    conn: Connection,
    tmp_path: PathBuf,
    final_path: PathBuf,
    next_nid: i64,
    next_eid: i64,
    ids: std::collections::HashMap<String, i64>,
}

impl SnapshotWriter {
    /// Starts a snapshot that will be published at `final_path`.
    pub fn create(final_path: &Path) -> Result<Self> {
        let dir = final_path.parent().context("snapshot path has no parent")?;
        std::fs::create_dir_all(dir)?;
        let tmp_path = final_path.with_extension("sqlite.tmp");
        if tmp_path.exists() {
            std::fs::remove_file(&tmp_path)?;
        }
        let conn = Connection::open(&tmp_path)?;
        conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
        conn.execute_batch(SCHEMA_SQL)?;
        conn.execute_batch("BEGIN")?;
        Ok(Self { conn, tmp_path, final_path: final_path.to_path_buf(), next_nid: 1, next_eid: 1, ids: Default::default() })
    }

    pub fn meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)", params![key, value])?;
        Ok(())
    }

    /// Stores a file's exact bytes (the frozen copy, R3-05).
    pub fn file(&self, fact: &FileFact, content: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO files(path, lang, sha256, bytes, content) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![fact.path, fact.lang, fact.sha256, fact.bytes as i64, content],
        )?;
        Ok(())
    }

    /// Inserts a container or external node. Parents must be inserted before children.
    pub fn node(&mut self, fact: &NodeFact, lang: &str, file: Option<&str>) -> Result<i64> {
        let (kind, subkind) = match fact.kind {
            NodeKind::External => ("external", None),
            k => ("container", Some(kind_str(k))),
        };
        self.raw_node(&fact.id, fact.parent.as_deref(), kind, subkind, &fact.name, fact.anonymous, Some(lang), file, fact.span.as_ref())
    }

    /// Inserts any node by its parts; returns its nid.
    #[allow(clippy::too_many_arguments)]
    pub fn raw_node(
        &mut self,
        path: &str,
        parent: Option<&str>,
        kind: &str,
        subkind: Option<&str>,
        name: &str,
        anonymous: bool,
        lang: Option<&str>,
        file: Option<&str>,
        span: Option<&Span>,
    ) -> Result<i64> {
        if self.ids.contains_key(path) {
            bail!("duplicate node id `{path}`");
        }
        let parent = match parent {
            Some(p) => Some(*self.ids.get(p).with_context(|| format!("parent `{p}` of `{path}` not inserted yet"))?),
            None => None,
        };
        let nid = self.next_nid;
        self.next_nid += 1;
        self.conn.execute(
            "INSERT INTO nodes(nid, path, parent_nid, kind, subkind, name, anonymous, lang, file, byte_start, byte_end, line_start, line_end)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                nid,
                path,
                parent,
                kind,
                subkind,
                name,
                anonymous,
                lang,
                file,
                span.map(|s| s.byte_start as i64),
                span.map(|s| s.byte_end as i64),
                span.map(|s| s.line_start),
                span.map(|s| s.line_end)
            ],
        )?;
        self.ids.insert(path.to_string(), nid);
        Ok(nid)
    }

    /// Declarations by declared name, for `names` links (ADR-0048): named functions,
    /// types and modules. A repeated name's ordinal (`f#2`) is not part of the name.
    pub fn named_declarations(&self) -> Result<std::collections::HashMap<String, Vec<Decl>>> {
        let mut stmt = self.conn.prepare(
            "SELECT nid, path, name, file, byte_start, byte_end FROM nodes
             WHERE kind = 'container' AND subkind IN ('function', 'type', 'module') AND anonymous = 0",
        )?;
        let mut out: std::collections::HashMap<String, Vec<Decl>> = Default::default();
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?))
        })?;
        for row in rows {
            let (nid, path, name, file, s, e) = row?;
            let declared = match name.rsplit_once('#') {
                Some((base, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => base.to_string(),
                _ => name,
            };
            out.entry(declared).or_default().push(Decl { nid, path, file, byte_start: s as u64, byte_end: e as u64 });
        }
        Ok(out)
    }

    pub fn nid(&self, path: &str) -> Option<i64> {
        self.ids.get(path).copied()
    }

    /// Inserts an edge between known nodes; returns its eid.
    pub fn edge(&mut self, kind: &str, subkind: Option<&str>, from: i64, to: Option<i64>, resolution: &str, group: Option<i64>) -> Result<i64> {
        let eid = self.next_eid;
        self.next_eid += 1;
        self.conn.execute(
            "INSERT INTO edges(eid, kind, subkind, from_nid, to_nid, resolution, group_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![eid, kind, subkind, from, to, resolution, group],
        )?;
        Ok(eid)
    }

    pub fn receipt(&self, eid: i64, r: &Receipt) -> Result<()> {
        let container = r.container.as_deref().and_then(|c| self.ids.get(c).copied());
        self.conn.execute(
            "INSERT INTO receipts(eid, role, file, byte_start, byte_end, line, col, end_line, end_col, container_nid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![eid, r.role, r.file, r.byte_start as i64, r.byte_end as i64, r.line, r.col, r.end_line, r.end_col, container],
        )?;
        Ok(())
    }

    pub fn diagnostic(&self, d: &Diagnostic) -> Result<()> {
        self.conn.execute(
            "INSERT INTO diagnostics(file, severity, message) VALUES (?1, ?2, ?3)",
            params![d.file, severity_str(d.severity), d.message],
        )?;
        Ok(())
    }

    /// Commits, indexes, marks complete, and atomically publishes (R3-G #9).
    pub fn finish(self) -> Result<PathBuf> {
        let Self { conn, tmp_path, final_path, .. } = self;
        conn.execute_batch("COMMIT")?;
        conn.execute_batch(INDEX_SQL)?;
        conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES ('layout_version', ?1)", params![DB_LAYOUT_VERSION.to_string()])?;
        conn.execute("INSERT OR REPLACE INTO meta(key, value) VALUES ('complete', '1')", [])?;
        conn.close().map_err(|(_, e)| e)?;
        if final_path.exists() {
            // Content-addressed key: an identical snapshot already exists.
            std::fs::remove_file(&tmp_path)?;
        } else {
            std::fs::rename(&tmp_path, &final_path)?;
        }
        Ok(final_path)
    }
}

/// Opens a published snapshot read-only and refuses incomplete ones (R3-G #9).
pub fn open_snapshot(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let complete: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = 'complete'", [], |r| r.get(0))
        .ok();
    if complete.as_deref() != Some("1") {
        bail!("snapshot {} is incomplete", path.display());
    }
    Ok(conn)
}

/// Node counts by (lang, kind/subkind), for the M0 stats log.
pub fn node_counts(conn: &Connection) -> Result<Vec<(String, String, i64)>> {
    let mut stmt = conn.prepare("SELECT coalesce(lang,''), kind || '/' || coalesce(subkind,''), count(*) FROM nodes GROUP BY 1, 2 ORDER BY 1, 2")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Function counts per file and language, for the independent-count check (R3-11).
pub fn function_counts_by_file(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare("SELECT file, count(*) FROM nodes WHERE kind = 'container' AND subkind = 'function' GROUP BY file ORDER BY file")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The container subkind (BQ-09: `container{file|module|type|function|block}`).
pub fn kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::File => "file",
        NodeKind::Module => "module",
        NodeKind::Type => "type",
        NodeKind::Function => "function",
        NodeKind::Block => "block",
        NodeKind::External => "external",
    }
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span { byte_start: 0, byte_end: 10, line_start: 1, line_end: 2 }
    }

    #[test]
    fn writes_and_reopens_a_complete_snapshot() {
        let dir = std::env::temp_dir().join(format!("insight-engine-test-{}", std::process::id()));
        let path = dir.join("snap.sqlite");
        let _ = std::fs::remove_file(&path);
        let mut w = SnapshotWriter::create(&path).unwrap();
        let file = NodeFact { id: "a.ts".into(), parent: None, kind: NodeKind::File, name: "a.ts".into(), span: Some(span()), anonymous: false };
        let func = NodeFact { id: "a.ts › f".into(), parent: Some("a.ts".into()), kind: NodeKind::Function, name: "f".into(), span: Some(span()), anonymous: false };
        w.node(&file, "ts", Some("a.ts")).unwrap();
        w.node(&func, "ts", Some("a.ts")).unwrap();
        let out = w.finish().unwrap();
        let conn = open_snapshot(&out).unwrap();
        assert_eq!(function_counts_by_file(&conn).unwrap(), vec![("a.ts".to_string(), 1)]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_duplicate_ids_and_unknown_parents() {
        let dir = std::env::temp_dir().join(format!("insight-engine-dup-{}", std::process::id()));
        let mut w = SnapshotWriter::create(&dir.join("s.sqlite")).unwrap();
        let orphan = NodeFact { id: "x › f".into(), parent: Some("x".into()), kind: NodeKind::Function, name: "f".into(), span: Some(span()), anonymous: false };
        assert!(w.node(&orphan, "ts", Some("x")).is_err());
        let file = NodeFact { id: "x".into(), parent: None, kind: NodeKind::File, name: "x".into(), span: Some(span()), anonymous: false };
        w.node(&file, "ts", Some("x")).unwrap();
        assert!(w.node(&file, "ts", Some("x")).is_err());
        drop(w);
        std::fs::remove_dir_all(dir).ok();
    }
}
