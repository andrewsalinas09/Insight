//! Read-only SQL for agents (ADR-0038; hardening R3-G #7; limits as parameters R4-B).
//!
//! - **Immutable read-only open:** URI `immutable=1`, so nothing can write and no locks are taken.
//! - **Authorizer allowlist:** only SELECT, reads, recursive CTEs, functions (not `load_extension`)
//!   and a few read-only introspection pragmas. ATTACH, DETACH, PRAGMA writes, VACUUM and every
//!   write are denied. Snapshots are attached only by Insight, with the authorizer lifted for that one statement.
//! - **One statement:** rusqlite refuses a prepare with a tail.
//! - **Deadline:** a progress handler interrupts the statement at `timeout_ms`.
//! - **Heap cap:** `PRAGMA hard_heap_limit`, plus length limits.
//! - Row limits are the caller's `max_rows`, under hard ceilings. Every result reports the total.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

pub const MAX_ROWS_CEILING: usize = 10_000;
pub const TIMEOUT_CEILING_MS: u64 = 30_000;
const HEAP_LIMIT_BYTES: i64 = 1 << 30;
const MAX_CELL_BYTES: usize = 64 * 1024;

const READ_ONLY_PRAGMAS: &[&str] = &["table_info", "table_xinfo", "index_list", "index_info", "foreign_key_list"];

/// Views shipped with every snapshot (R4-B). Created in the connection's TEMP schema.
pub const VIEWS: &[(&str, &str, &str)] = &[
    (
        "callers",
        "One row per call edge: caller and callee paths, resolution, group.",
        "CREATE TEMP VIEW callers AS
         SELECT e.eid, e.resolution, e.group_id, e.from_nid AS caller_nid, f.path AS caller, e.to_nid AS callee_nid, t.path AS callee
         FROM main.edges e JOIN main.nodes f ON f.nid = e.from_nid LEFT JOIN main.nodes t ON t.nid = e.to_nid
         WHERE e.kind = 'call'",
    ),
    (
        "receipt_text",
        "Every receipt with its source text (computed from the span, R3-G #8).",
        "CREATE TEMP VIEW receipt_text AS
         SELECT r.*, CAST(substr(f.content, r.byte_start + 1, r.byte_end - r.byte_start) AS TEXT) AS text
         FROM main.receipts r JOIN main.files f ON f.path = r.file",
    ),
];

pub struct QueryDb {
    conn: Connection,
    deadline: Arc<Mutex<Option<Instant>>>,
    /// When true, `files.content` reads return NULL (a deny rule matches a file in the snapshot).
    hide_content: Arc<Mutex<bool>>,
}

pub enum Cell {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(usize),
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    /// Total rows the statement produced (counted past `max_rows`).
    pub total: usize,
    /// False when the deadline hit while counting rows past `max_rows`: `total` is then a lower bound.
    pub total_exact: bool,
}

fn uri(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file:");
    if !p.starts_with('/') {
        out.push('/');
    }
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"/:._-~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out.push_str("?mode=ro&immutable=1");
    out
}

fn authorize(hide_content: &Mutex<bool>, ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Select | AuthAction::Recursive => Authorization::Allow,
        AuthAction::Read { table_name, column_name } => {
            if table_name == "files" && column_name == "content" && *hide_content.lock().expect("lock") {
                Authorization::Ignore
            } else {
                Authorization::Allow
            }
        }
        AuthAction::Function { function_name } if !function_name.eq_ignore_ascii_case("load_extension") => Authorization::Allow,
        AuthAction::Pragma { pragma_name, pragma_value: None } if READ_ONLY_PRAGMAS.iter().any(|p| p.eq_ignore_ascii_case(pragma_name)) => {
            Authorization::Allow
        }
        _ => Authorization::Deny,
    }
}

impl QueryDb {
    /// Opens a complete snapshot for agent queries.
    pub fn open(path: &Path) -> Result<Self> {
        crate::open_snapshot(path)?; // refuses incomplete snapshots (R3-G #9)
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(uri(path), flags).with_context(|| format!("open {}", path.display()))?;
        conn.query_row(&format!("PRAGMA hard_heap_limit = {HEAP_LIMIT_BYTES}"), [], |_| Ok(()))?;
        conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, 256 << 20)?;
        conn.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 1 << 20)?;
        for (_, _, ddl) in VIEWS {
            conn.execute_batch(ddl)?;
        }
        let deadline: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let d = deadline.clone();
        conn.progress_handler(10_000, Some(move || d.lock().expect("lock").is_some_and(|t| Instant::now() >= t)))?;
        let db = QueryDb { conn, deadline, hide_content: Arc::new(Mutex::new(false)) };
        db.guard()?;
        Ok(db)
    }

    fn guard(&self) -> Result<()> {
        let h = self.hide_content.clone();
        self.conn.authorizer(Some(move |ctx: AuthContext<'_>| authorize(&h, ctx)))?;
        Ok(())
    }

    /// Attaches another complete snapshot under `alias` (Insight only; agents cannot ATTACH).
    pub fn attach(&self, alias: &str, path: &Path) -> Result<()> {
        ensure!(
            !alias.is_empty() && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !alias.starts_with(|c: char| c.is_ascii_digit()),
            "alias `{alias}` must be a plain identifier"
        );
        crate::open_snapshot(path)?;
        self.conn.authorizer(None::<for<'r> fn(AuthContext<'r>) -> Authorization>)?;
        let r = self.conn.execute(&format!("ATTACH DATABASE ?1 AS \"{alias}\""), [uri(path)]);
        self.guard()?;
        r?;
        Ok(())
    }

    pub fn set_hide_content(&self, hide: bool) {
        *self.hide_content.lock().expect("lock") = hide;
    }

    /// The connection, for Insight's own reads (still under the authorizer).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Runs one agent statement.
    pub fn query(&self, sql: &str, max_rows: usize, timeout_ms: u64) -> Result<QueryResult> {
        ensure!(max_rows <= MAX_ROWS_CEILING, "max_rows is at most {MAX_ROWS_CEILING}");
        ensure!((1..=TIMEOUT_CEILING_MS).contains(&timeout_ms), "timeout_ms is 1..={TIMEOUT_CEILING_MS}");
        *self.deadline.lock().expect("lock") = Some(Instant::now() + Duration::from_millis(timeout_ms));
        let r = self.run(sql, max_rows);
        *self.deadline.lock().expect("lock") = None;
        r
    }

    fn run(&self, sql: &str, max_rows: usize) -> Result<QueryResult> {
        let mut stmt = self.conn.prepare(sql)?;
        ensure!(stmt.readonly(), "only read-only statements are allowed");
        let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let n = columns.len();
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        let mut total = 0usize;
        loop {
            match rows.next() {
                Ok(Some(row)) => {
                    total += 1;
                    if out.len() < max_rows {
                        let mut cells = Vec::with_capacity(n);
                        for i in 0..n {
                            cells.push(match row.get_ref(i)? {
                                ValueRef::Null => Cell::Null,
                                ValueRef::Integer(v) => Cell::Int(v),
                                ValueRef::Real(v) => Cell::Real(v),
                                ValueRef::Text(t) => {
                                    let mut s = String::from_utf8_lossy(t).to_string();
                                    if s.len() > MAX_CELL_BYTES {
                                        let mut cut = MAX_CELL_BYTES;
                                        while !s.is_char_boundary(cut) {
                                            cut -= 1;
                                        }
                                        s = format!("{}… [{} bytes total]", &s[..cut], t.len());
                                    }
                                    Cell::Text(s)
                                }
                                ValueRef::Blob(b) => Cell::Blob(b.len()),
                            });
                        }
                        out.push(cells);
                    }
                }
                Ok(None) => return Ok(QueryResult { columns, rows: out, total, total_exact: true }),
                Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::OperationInterrupted => {
                    if out.len() < max_rows {
                        bail!("timeout_ms reached after {total} rows; raise timeout_ms or narrow the query");
                    }
                    return Ok(QueryResult { columns, rows: out, total, total_exact: false });
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl QueryResult {
    /// Tab-separated text: a header, the rows, then the totals. Tabs and newlines in cells are escaped.
    pub fn render(&self) -> String {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r");
        let mut out = self.columns.join("\t");
        out.push('\n');
        for r in &self.rows {
            let cells: Vec<String> = r
                .iter()
                .map(|c| match c {
                    Cell::Null => "NULL".into(),
                    Cell::Int(v) => v.to_string(),
                    Cell::Real(v) => v.to_string(),
                    Cell::Text(s) => esc(s),
                    Cell::Blob(n) => format!("<blob {n} bytes>"),
                })
                .collect();
            out.push_str(&cells.join("\t"));
            out.push('\n');
        }
        let total = if self.total_exact { self.total.to_string() } else { format!("at least {}", self.total) };
        out.push_str(&format!("-- {} rows shown, {total} total", self.rows.len()));
        if self.rows.len() < self.total {
            out.push_str(" (truncated by max_rows)");
        }
        out.push('\n');
        out
    }
}
