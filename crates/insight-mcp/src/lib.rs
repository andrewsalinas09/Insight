//! The MCP tools (ADR-0031, ADR-0039, R4-B), as one [`Server`] that both the
//! window app and `insight mcp` run.
//!
//! Newline-delimited JSON-RPC 2.0. The running app owns the server and serves it
//! to `insight mcp` over a local pipe ([`bridge`]); with no app, `insight mcp`
//! runs a server itself against the on-disk snapshot cache (R2-13: "when it's
//! closed, they run the same backend standalone").
//!
//! Over MCP, nothing can create or approve a project or change its commands
//! (R3-01): `build` refuses unapproved projects the same way the CLI does.
//! The snapshot a session queries is pinned until `load` switches it (R3-17).

pub mod bridge;
mod deny;
mod view;
mod window;

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_engine::query::{MAX_ROWS_CEILING, QueryDb, TIMEOUT_CEILING_MS, VIEWS};
use insight_solver::path::Follow;
use insight_solver::{paths, settings};
use serde_json::{Value, json};

use deny::DenyRules;

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The up-front instructions: the text after the first `---` line of docs/MCP-INSTRUCTIONS.md.
fn instructions() -> &'static str {
    let doc = include_str!("../../../docs/MCP-INSTRUCTIONS.md");
    doc.split_once("\n---\n").map_or(doc, |(_, body)| body).trim()
}

struct Loaded {
    project: String,
    main: PathBuf,
    attached: Vec<(String, PathBuf)>,
    db: QueryDb,
    /// Every loaded snapshot's source view: `main` first, then each attached one.
    repos: Vec<RepoView>,
}

/// One loaded snapshot's files as the agent may see them.
struct RepoView {
    schema: String,
    root: Option<PathBuf>,
    deny: DenyRules,
    /// Snapshot files the agent's deny rules cover.
    denied: Vec<String>,
    frozen: PathBuf,
}

impl RepoView {
    /// Reads the snapshot's repo, mirrors its deny rules and writes its frozen-source folder.
    fn open(db: &QueryDb, project: &str, schema: &str, file: &Path) -> Result<RepoView> {
        let repo: String = db.conn().query_row(&format!("SELECT value FROM {schema}.meta WHERE key = 'repo'"), [], |r| r.get(0))?;
        let root = settings::load(&paths::project_dir(project)?).ok().and_then(|s| s.repo.get(&repo).map(|r| PathBuf::from(&r.path)));
        let deny = DenyRules::load(root.as_deref());
        let files: Vec<String> = {
            let mut stmt = db.conn().prepare(&format!("SELECT path FROM {schema}.files ORDER BY path"))?;
            stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?
        };
        let denied: Vec<String> = files.iter().filter(|f| deny.denies(root.as_deref(), f)).cloned().collect();
        let frozen = paths::frozen_source_dir(file)?;
        freeze(db, schema, &frozen, &files, &denied)?;
        Ok(RepoView { schema: schema.to_string(), root, deny, denied, frozen })
    }

    fn denies(&self, file: &str) -> bool {
        self.denied.iter().any(|d| d == file) || self.deny.denies(self.root.as_deref(), file)
    }
}

/// Where the server tells the window what changed: an event name and its payload.
pub type WindowSink = Box<dyn Fn(&str, Value) + Send>;

/// The backend's state: what's loaded, the last view, what the user has selected.
#[derive(Default)]
pub struct Server {
    loaded: Option<Loaded>,
    window: Option<WindowSink>,
    /// The last view `show` accepted (resolved), kept for a window that connects later (BQ-42).
    view: Option<Value>,
    /// What the user has selected in the window, for `selection`.
    selection: Value,
}

/// Runs the server on stdin/stdout in this process (no window).
pub fn serve() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut server = Server::default();
    for line in stdin.lock().lines() {
        if let Some(r) = server.handle_line(&line?) {
            stdout.write_all(r.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

impl Server {
    /// Connects a window: `show` and `load` are pushed to it from now on.
    pub fn attach_window(&mut self, sink: WindowSink) {
        self.window = Some(sink);
    }

    /// One JSON-RPC message in, at most one reply out (notifications get none).
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return None;
        }
        let reply = match serde_json::from_str::<Value>(line) {
            Ok(msg) => self.handle(&msg),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
        };
        reply.map(|r| r.to_string())
    }

    /// The loaded snapshot as boxes and lines, for the window.
    pub fn graph(&self) -> Result<Value> {
        window::graph(self.loaded()?)
    }

    /// One file of the loaded snapshot, for the window's code view.
    pub fn file(&self, path: &str) -> Result<String> {
        window::file_text(self.loaded()?, path)
    }

    /// The last accepted view, if any.
    pub fn view(&self) -> Option<&Value> {
        self.view.as_ref()
    }

    /// The window reports what the user selected (and where the view stands).
    pub fn set_selection(&mut self, s: Value) {
        self.selection = s;
    }

    /// Projects and their snapshots, newest first, and what's loaded, for the window's picker.
    pub fn projects(&self) -> Result<Value> {
        let root = paths::data_root()?.join("projects");
        let mut names: Vec<String> = std::fs::read_dir(&root)
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().join("INSIGHT.md").exists()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        names.sort();
        let list: Vec<Value> = names
            .into_iter()
            .map(|p| {
                let snaps: Vec<Value> = snapshots(&p).into_iter().map(|(file, repo, commit)| json!({"file": file, "repo": repo, "commit": commit})).collect();
                json!({"name": p, "snapshots": snaps})
            })
            .collect();
        let loaded = self.loaded.as_ref().map(|l| json!({"project": l.project, "snapshot": file_name(&l.main)}));
        Ok(json!({"projects": list, "loaded": loaded}))
    }

    /// Loads a snapshot as `main`, the same as the `load` tool.
    pub fn load_main(&mut self, project: &str, snapshot: Option<&str>) -> Result<String> {
        self.load(project, snapshot, None)
    }

    fn push(&self, event: &str, payload: Value) -> bool {
        match &self.window {
            Some(w) => {
                w(event, payload);
                true
            }
            None => false,
        }
    }

    fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        // Notifications (no id) get no reply.
        let id = id?;
        let result = match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
                let version = PROTOCOL_VERSIONS.iter().find(|v| **v == asked).unwrap_or(&PROTOCOL_VERSIONS[0]);
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "insight", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": instructions(),
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (text, is_error) = match self.call(&name, &args) {
                    Ok(t) => (t, false),
                    Err(e) => (format!("{e:#}"), true),
                };
                Ok(json!({"content": [{"type": "text", "text": text}], "isError": is_error}))
            }
            _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        })
    }

    fn call(&mut self, name: &str, a: &Value) -> Result<String> {
        match name {
            "status" => self.status(),
            "load" => self.load(req_str(a, "project")?, opt_str(a, "snapshot"), opt_str(a, "alias")),
            "build" => build(req_str(a, "project")?, opt_str(a, "commit")),
            "sql" => {
                let l = self.loaded()?;
                let r = l.db.query(req_str(a, "query")?, req_u64(a, "max_rows")? as usize, req_u64(a, "timeout_ms")?)?;
                Ok(r.render())
            }
            "path" => self.path(a),
            "source" => self.source(a),
            "show" => self.show(a),
            "selection" => Ok(if self.selection.is_null() { "nothing selected".to_string() } else { serde_json::to_string_pretty(&self.selection)? }),
            other => bail!("unknown tool `{other}`"),
        }
    }

    fn show(&mut self, a: &Value) -> Result<String> {
        // Jumping to a step of the view already on screen (BQ-39).
        if let Some(step) = a.get("step").and_then(Value::as_u64) {
            let steps = self.view.as_ref().and_then(|v| v["steps"].as_array()).map_or(0, Vec::len);
            anyhow::ensure!(step >= 1 && step as usize <= steps, "the current view has {steps} step(s); `step` counts from 1");
            let shown = self.push("step", json!(step - 1));
            return Ok(format!("step {step} of {steps}{}", if shown { "" } else { " (window: not connected)" }));
        }
        let raw = a.get("view").context("`view` (object) or `step` (integer) is required")?;
        let v = view::resolve(self.loaded()?, raw)?;
        let counts = format!(
            "{} open, {} highlighted, {} notes, {} steps, {} chains",
            v["open"].as_array().map_or(0, Vec::len),
            v["highlight"].as_array().map_or(0, Vec::len),
            v["notes"].as_array().map_or(0, Vec::len),
            v["steps"].as_array().map_or(0, Vec::len),
            v["layout"]["chain"].as_array().map_or(0, Vec::len)
        );
        let shown = self.push("view", v.clone());
        self.view = Some(v);
        Ok(if shown {
            format!("shown: {counts}. The user steps with Next/Back; `selection` reports what they pick.")
        } else {
            format!("accepted ({counts}), window: not connected. It's kept as the latest view.")
        })
    }

    fn loaded(&self) -> Result<&Loaded> {
        self.loaded.as_ref().context("no snapshot is loaded: call `load` (see `status` for projects and snapshots)")
    }

    fn status(&self) -> Result<String> {
        let mut o = format!(
            "Insight {} (fact schema {}, DB layout {})\n",
            env!("CARGO_PKG_VERSION"),
            insight_model::FACT_SCHEMA_VERSION,
            insight_engine::DB_LAYOUT_VERSION
        );
        let global = paths::data_root()?.join("INSIGHT.md");
        o.push_str(&format!(
            "Global INSIGHT.md (prose: knowledge notes, preferences, Wishes): {}{}\n",
            global.display(),
            if global.exists() { "" } else { " (not created yet)" }
        ));
        o.push_str("\n## Projects\n");
        let root = paths::data_root()?.join("projects");
        let mut projects: Vec<String> = std::fs::read_dir(&root)
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().join("INSIGHT.md").exists()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        projects.sort();
        if projects.is_empty() {
            o.push_str("(none: the user creates projects with `insight init`)\n");
        }
        for p in &projects {
            let dir = paths::project_dir(p)?;
            let approval = match (settings::load(&dir), settings::approved(&dir)) {
                (Ok(s), Ok(Some(a))) if a.fingerprint() == settings::Commands::of(&s).fingerprint() => "commands approved",
                (Ok(_), Ok(_)) => "commands NOT approved (the user runs `insight approve`)",
                _ => "INSIGHT.md unreadable",
            };
            o.push_str(&format!("- {p}: {approval}; settings: {}\n", dir.join("INSIGHT.md").display()));
            // The last build, per repo, so setup problems are visible at a glance.
            let last_log = std::fs::read_dir(dir.join("logs"))
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("build-"))
                .max_by_key(|e| e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH))
                .map(|e| e.path());
            if let Some(log) = last_log {
                let text = std::fs::read_to_string(&log).unwrap_or_default();
                o.push_str(&format!("  - last build log: {}\n", log.display()));
                for l in text.lines().filter(|l| l.starts_with("built ") || l.starts_with("== repo") && l.contains("FAILED") || l.starts_with("result:")) {
                    let l: String = l.chars().take(300).collect();
                    o.push_str(&format!("    {l}\n"));
                }
            }
            for (file, repo, commit) in snapshots(p) {
                o.push_str(&format!("  - snapshot `{file}`: repo {repo}, commit {commit}\n"));
            }
        }
        o.push_str("\n## Loaded\n");
        let Some(l) = &self.loaded else {
            o.push_str("nothing: call `load`\n");
            o.push_str(&static_status());
            return Ok(o);
        };
        o.push_str(&format!("- `main` (unqualified, pinned): {} / {}\n", l.project, file_name(&l.main)));
        for (alias, p) in &l.attached {
            o.push_str(&format!("- `{alias}`: {}\n", file_name(p)));
        }
        o.push_str("\nFrozen-source folders (Grep/Read these bytes, not the working tree):\n");
        for r in &l.repos {
            o.push_str(&format!("- `{}`: {}\n", r.schema, r.frozen.display()));
            if !r.denied.is_empty() {
                o.push_str(&format!(
                    "  deny rules hide {} file(s): `source` refuses them, they're absent from the folder, and `files.content` reads as NULL.\n",
                    r.denied.len()
                ));
            }
            if !r.deny.sources.is_empty() {
                o.push_str(&format!("  mirrored deny rules: {}\n", r.deny.sources.join("; ")));
            }
        }
        o.push_str("\n## Schema (main)\n");
        let mut stmt = l.db.conn().prepare("SELECT sql FROM main.sqlite_master WHERE sql IS NOT NULL ORDER BY type DESC, name")?;
        for s in stmt.query_map([], |r| r.get::<_, String>(0))? {
            o.push_str(&format!("{};\n", s?));
        }
        o.push_str("\n## Enum values with counts (main)\n");
        for (title, q) in [
            ("nodes kind/subkind", "SELECT kind || coalesce('/' || subkind, ''), count(*) FROM main.nodes GROUP BY 1 ORDER BY 2 DESC"),
            ("nodes lang", "SELECT coalesce(lang, 'NULL'), count(*) FROM main.nodes GROUP BY 1 ORDER BY 2 DESC"),
            ("edges kind/subkind", "SELECT kind || coalesce('/' || subkind, ''), count(*) FROM main.edges GROUP BY 1 ORDER BY 2 DESC"),
            ("edges resolution", "SELECT resolution, count(*) FROM main.edges GROUP BY 1 ORDER BY 2 DESC"),
            ("receipts role", "SELECT role, count(*) FROM main.receipts GROUP BY 1 ORDER BY 2 DESC"),
            ("meta", "SELECT key, value FROM main.meta ORDER BY key"),
        ] {
            let mut stmt = l.db.conn().prepare(q)?;
            let vals: Vec<String> = stmt.query_map([], |r| Ok(format!("{}={}", r.get::<_, String>(0)?, r.get_ref(1)?.as_str().map(String::from).unwrap_or_else(|_| r.get::<_, i64>(1).map(|v| v.to_string()).unwrap_or_default()))))?.collect::<Result<_, _>>()?;
            o.push_str(&format!("- {title}: {}\n", vals.join(", ")));
        }
        o.push_str(&static_status());
        Ok(o)
    }

    fn load(&mut self, project: &str, snapshot: Option<&str>, alias: Option<&str>) -> Result<String> {
        paths::check_name(project)?;
        let file = match snapshot {
            Some(name) => {
                let known: Vec<String> = snapshots(project).into_iter().map(|(f, _, _)| f).collect();
                if !known.iter().any(|k| k == name) {
                    bail!("`{name}` is not a snapshot of `{project}`; known: {}", known.join(", "));
                }
                paths::snapshots_dir(project)?.join(name)
            }
            None => insight_solver::latest_per_repo(project)?.remove(0).1,
        };
        // A project of several repos: with no snapshot named, every repo's newest is loaded,
        // the first as `main` and the others attached under their repo's name.
        let others: Vec<(String, PathBuf)> = if snapshot.is_none() && alias.is_none() { insight_solver::latest_per_repo(project)?.into_iter().skip(1).collect() } else { Vec::new() };
        if let Some(alias) = alias {
            let l = self.loaded.as_mut().context("load a main snapshot (no alias) first")?;
            if alias.eq_ignore_ascii_case("main") || alias.eq_ignore_ascii_case("temp") || l.attached.iter().any(|(a, _)| a == alias) {
                bail!("alias `{alias}` is taken");
            }
            l.db.attach(alias, &file)?;
            // Freezing reads content, which SQL may be hiding.
            l.db.set_hide_content(false);
            let view = RepoView::open(&l.db, project, alias, &file);
            l.db.set_hide_content(l.repos.iter().chain(view.as_ref().ok()).any(|r| !r.denied.is_empty()));
            let view = view?;
            l.attached.push((alias.to_string(), file.clone()));
            l.repos.push(view);
            return Ok(format!("attached {} as `{alias}`: query it as `{alias}.nodes`, `{alias}.edges`, …", file_name(&file)));
        }
        let db = QueryDb::open(&file)?;
        let main = RepoView::open(&db, project, "main", &file)?;
        let files: i64 = db.conn().query_row("SELECT count(*) FROM main.files", [], |r| r.get(0))?;
        let mut summary = format!(
            "loaded {} / {} as `main` (pinned until the next `load` without an alias)\n{} files; frozen-source folder: {}{}",
            project,
            file_name(&file),
            files,
            main.frozen.display(),
            if main.denied.is_empty() { String::new() } else { format!("\n{} file(s) hidden by the agent's deny rules", main.denied.len()) }
        );
        let mut attached = Vec::new();
        let mut repos = vec![main];
        for (repo, p) in others {
            let a = insight_solver::schema_alias(&repo);
            db.attach(&a, &p)?;
            let view = RepoView::open(&db, project, &a, &p)?;
            summary.push_str(&format!("\nattached repo `{repo}` ({}) as `{a}`; frozen-source folder: {}", file_name(&p), view.frozen.display()));
            if !view.denied.is_empty() {
                summary.push_str(&format!(" ({} file(s) hidden by deny rules)", view.denied.len()));
            }
            attached.push((a, p));
            repos.push(view);
        }
        if !attached.is_empty() {
            summary.push_str("\n`path` searches every loaded repo; nodes show as `<schema>:<path>`; `same` follows identical source across repos. `source` takes those `<schema>:` prefixes too.");
        }
        db.set_hide_content(repos.iter().any(|r| !r.denied.is_empty()));
        let snap = file_name(&file);
        self.loaded = Some(Loaded { project: project.to_string(), main: file, attached, db, repos });
        // A view refers to the snapshot it was made on.
        self.view = None;
        self.push("loaded", json!({"project": project, "snapshot": snap}));
        Ok(summary)
    }

    fn path(&self, a: &Value) -> Result<String> {
        let l = self.loaded()?;
        let follow = a.get("follow").and_then(Value::as_array).context("`follow` is required: choose the hop kinds; there is no default (ADR-0047)")?;
        let follow: Vec<&str> = follow.iter().filter_map(Value::as_str).collect();
        let follow = Follow::parse_list(&follow.join(","))?;
        let opts = insight_solver::path::Options {
            follow,
            max_occurrences: a.get("max_occurrences").and_then(Value::as_u64).map(|v| v as usize),
            max_depth: a.get("max_depth").and_then(Value::as_u64).map(|v| v as usize),
            schemas: std::iter::once("main".to_string()).chain(l.attached.iter().map(|(a, _)| a.clone())).collect(),
        };
        let receipts = match a.get("receipts").and_then(Value::as_str).unwrap_or("expr") {
            "expr" => true,
            "none" => false,
            other => bail!("receipts is `expr` or `none`, not `{other}`"),
        };
        let r = insight_solver::path::path(l.db.conn(), &str_list(a, "from"), &str_list(a, "to"), &opts)?;
        insight_solver::path::render(l.db.conn(), &r, receipts)
    }

    fn source(&self, a: &Value) -> Result<String> {
        let l = self.loaded()?;
        let refs = str_list(a, "refs");
        if refs.is_empty() {
            bail!("`refs` is required: node ids/suffixes, `file`, `file:line` or `file:start-end`");
        }
        let mut out = String::new();
        for r in &refs {
            match self.one_source(l, r) {
                Ok(t) => out.push_str(&t),
                Err(e) => out.push_str(&format!("== {r}\n{e:#}\n")),
            }
            out.push('\n');
        }
        Ok(out)
    }

    fn one_source(&self, l: &Loaded, r: &str) -> Result<String> {
        // `<schema>:…` (as `path` prints nodes of several repos) picks a loaded repo; default `main`.
        let (repo, r) = match r.split_once(':').and_then(|(s, rest)| l.repos.iter().find(|v| v.schema == s).map(|v| (v, rest))) {
            Some(x) => x,
            None => (&l.repos[0], r),
        };
        let schema = repo.schema.as_str();
        let conn = l.db.conn();
        let exists = |f: &str| conn.query_row(&format!("SELECT 1 FROM {schema}.files WHERE path = ?1"), [f], |_| Ok(())).is_ok();
        // `file`, `file:line`, `file:start-end`; otherwise a node.
        let (file, range) = match r.rsplit_once(':') {
            Some((f, lines)) if exists(f) => {
                let (s, e) = match lines.split_once('-') {
                    Some((s, e)) => (s.trim().parse::<usize>()?, e.trim().parse::<usize>()?),
                    None => {
                        let n = lines.trim().parse::<usize>()?;
                        (n, n)
                    }
                };
                (f.to_string(), Some((s, e)))
            }
            _ if exists(r) => (r.to_string(), None),
            _ => {
                // One schema searched: its global id is the node id.
                let nid = insight_solver::path::resolve_in(conn, &[schema.to_string()], r)?;
                let (path, file, s, e): (String, Option<String>, Option<i64>, Option<i64>) =
                    conn.query_row(&format!("SELECT path, file, line_start, line_end FROM {schema}.nodes WHERE nid = ?1"), [nid], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?)))?;
                let file = file.with_context(|| format!("`{path}` has no source (an external or literal node)"))?;
                return self.lines(l, repo, &file, s.zip(e).map(|(s, e)| (s as usize, e as usize)), &path);
            }
        };
        self.lines(l, repo, &file, range, r)
    }

    fn lines(&self, l: &Loaded, repo: &RepoView, file: &str, range: Option<(usize, usize)>, title: &str) -> Result<String> {
        if repo.denies(file) {
            bail!("`{file}` is covered by the agent's deny rules");
        }
        // Allowed files are readable here even while `files.content` is hidden from SQL.
        let hidden = l.repos.iter().any(|r| !r.denied.is_empty());
        l.db.set_hide_content(false);
        let content = l.db.conn().query_row::<Vec<u8>, _, _>(&format!("SELECT content FROM {}.files WHERE path = ?1", repo.schema), [file], |r| r.get(0));
        l.db.set_hide_content(hidden);
        let content = content?;
        let text = String::from_utf8_lossy(&content);
        let all: Vec<&str> = text.lines().collect();
        let (s, e) = range.unwrap_or((1, all.len()));
        if s == 0 || s > e || s > all.len().max(1) {
            bail!("{file} has {} lines; asked for {s}-{e}", all.len());
        }
        let e = e.min(all.len());
        let mut out = format!("== {title} ({file}:{s}-{e})\n");
        for (i, line) in all[s - 1..e].iter().enumerate() {
            out.push_str(&format!("{:>5}  {line}\n", s + i));
        }
        Ok(out)
    }
}

/// Writes the snapshot's allowed files once into a read-only folder.
fn freeze(db: &QueryDb, schema: &str, dir: &Path, files: &[String], denied: &[String]) -> Result<()> {
    let marker = dir.join(".insight-complete");
    if marker.exists() {
        return Ok(());
    }
    let mut stmt = db.conn().prepare(&format!("SELECT content FROM {schema}.files WHERE path = ?1"))?;
    for f in files {
        // Paths come from extractors; never write outside the folder.
        let rel = Path::new(f);
        if denied.contains(f) || rel.is_absolute() || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            continue;
        }
        let dest = dir.join(rel);
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        let bytes: Vec<u8> = stmt.query_row([f], |r| r.get(0))?;
        if dest.exists() {
            let mut perm = std::fs::metadata(&dest)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            std::fs::set_permissions(&dest, perm)?;
        }
        std::fs::write(&dest, bytes)?;
        let mut perm = std::fs::metadata(&dest)?.permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&dest, perm)?;
    }
    std::fs::write(marker, b"")?;
    Ok(())
}

fn build(project: &str, commit: Option<&str>) -> Result<String> {
    let mut log = insight_solver::Log::create(project, "build")?;
    let r = insight_solver::build(project, &insight_solver::BuildOptions { commit: commit.map(String::from) }, &mut log);
    let mut o = format!("log: {}\n", log.path.display());
    match r {
        Ok(built) => {
            for b in built {
                o.push_str(&format!("OK {}: {} functions -> snapshot `{}`\n", b.repo, b.functions, file_name(&b.snapshot)));
            }
            o.push_str("The loaded snapshot is unchanged (pinned); call `load` to switch.\n");
            Ok(o)
        }
        Err(e) => {
            let code = e.downcast_ref::<insight_solver::Refusal>().map(insight_solver::Refusal::exit_code).unwrap_or(1);
            bail!("{o}refused (exit {code}): {e:#}")
        }
    }
}

/// (file name, repo, commit) of each complete snapshot of a project, newest first.
fn snapshots(project: &str) -> Vec<(String, String, String)> {
    let Ok(dir) = paths::snapshots_dir(project) else { return Vec::new() };
    let mut out: Vec<(std::time::SystemTime, String, String, String)> = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "sqlite") {
            continue;
        }
        let Ok(conn) = insight_engine::open_snapshot(&p) else { continue };
        let meta = |k: &str| conn.query_row("SELECT value FROM meta WHERE key = ?1", [k], |r| r.get::<_, String>(0)).unwrap_or_default();
        let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        out.push((t, file_name(&p), meta("repo"), meta("commit")));
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.0));
    out.into_iter().map(|(_, f, r, c)| (f, r, c)).collect()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()
}

fn req_str<'a>(a: &'a Value, k: &str) -> Result<&'a str> {
    a.get(k).and_then(Value::as_str).with_context(|| format!("`{k}` (string) is required"))
}

fn opt_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

fn req_u64(a: &Value, k: &str) -> Result<u64> {
    a.get(k).and_then(Value::as_u64).with_context(|| format!("`{k}` (integer) is required"))
}

/// A string or a list of strings.
fn str_list(a: &Value, k: &str) -> Vec<String> {
    match a.get(k) {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(v)) => v.iter().filter_map(Value::as_str).map(String::from).collect(),
        _ => Vec::new(),
    }
}

fn static_status() -> String {
    let views: String = VIEWS.iter().map(|(name, what, _)| format!("- {name}: {what}\n")).collect();
    STATIC_STATUS.replace("{views}", &views).replace("{max_rows}", &MAX_ROWS_CEILING.to_string()).replace("{timeout}", &TIMEOUT_CEILING_MS.to_string())
}

const STATIC_STATUS: &str = "
## `follow` tokens (exact predicates over `edges`)
- call: kind='call' AND to_nid IS NOT NULL (resolution 'possible' is labeled `call (possible)`)
- handoff: kind='value-flow' AND subkind='handoff'
- literal: F -[kind='link' AND subkind='literal']-> literal node <-[same]- G, for G != F
- names: F -[link/literal]-> literal node -[kind='link' AND subkind='names']-> declaration
- data: (kind='value-flow' AND subkind IN ('assign','result','return')) OR kind='binds' (virtual-dispatch binds are resolution='possible')
- write / access: kind='write' / kind='access' (function -> state)
- read: kind='read' walked from the state to the reading function
- constraints: max_occurrences (skip literals occurring more often), max_depth (route length)

## Shipped views (TEMP schema)
{views}

## Pedantic switches
None yet: nothing lean-by-default is withheld today (standard-library origin, implicit calls and macro-generated facts aren't recorded yet).

## Ceilings
sql: max_rows <= {max_rows}, timeout_ms <= {timeout}; one read-only statement; no ATTACH/PRAGMA writes/extensions.

## Not built yet (see SCHEMA.md)
reach()/nid() SQL table functions, the writers and race_candidates views, compare, tag_coverage; in `show`: groups, collapse, code excerpts, abstraction nodes, example data, and views over attached repos; implements, exits, timing; multi-repo link layer.
";

fn tools() -> Value {
    json!([
        {
            "name": "status",
            "description": "Projects, snapshots, what's loaded, the live schema with enum counts, follow-token definitions, shipped views, the frozen-source folder, ceilings. Call first.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "load",
            "description": "Load a snapshot. Without `alias` it becomes the pinned, unqualified `main` (replacing the previous one); with `alias` it's attached for SQL as `<alias>.<table>`.",
            "inputSchema": {"type": "object", "properties": {
                "project": {"type": "string"},
                "snapshot": {"type": "string", "description": "A snapshot file name from `status`; default: the project's newest."},
                "alias": {"type": "string", "description": "Attach under this SQL alias instead of replacing `main`."}
            }, "required": ["project"]}
        },
        {
            "name": "build",
            "description": "Build snapshots of an initialized, approved project (refuses otherwise). Doesn't change the loaded snapshot.",
            "inputSchema": {"type": "object", "properties": {
                "project": {"type": "string"},
                "commit": {"type": "string", "description": "A git revision; default: the working copy."}
            }, "required": ["project"]}
        },
        {
            "name": "sql",
            "description": "One read-only SQLite statement against the loaded snapshot(s). You set the row and time limits (under the ceilings in `status`); the result reports the total row count.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "max_rows": {"type": "integer", "minimum": 0, "maximum": MAX_ROWS_CEILING},
                "timeout_ms": {"type": "integer", "minimum": 1, "maximum": TIMEOUT_CEILING_MS}
            }, "required": ["query", "max_rows", "timeout_ms"]}
        },
        {
            "name": "path",
            "description": "Deterministic reachability with receipts. You choose every hop kind (no default). Give `from`, `to`, or both (each a node id/suffix or a list). Returns every hop on some route, labeled, with cycles and recursion reported.",
            "inputSchema": {"type": "object", "properties": {
                "from": {"type": "array", "items": {"type": "string"}},
                "to": {"type": "array", "items": {"type": "string"}},
                "follow": {"type": "array", "items": {"type": "string", "enum": ["call", "handoff", "literal", "names", "data", "read", "write", "access", "same"]}, "minItems": 1},
                "max_occurrences": {"type": "integer", "minimum": 1, "description": "Skip literals that occur more often than this."},
                "max_depth": {"type": "integer", "minimum": 1, "description": "Keep only routes of at most this many hops."},
                "receipts": {"type": "string", "enum": ["expr", "none"], "description": "Source lines per hop (expr) or none."}
            }, "required": ["follow"]}
        },
        {
            "name": "show",
            "description": "Put a view on the user's Insight window (ADR-0040): boxes to open, highlights, a focus, notes, steps (Next/Back) and layout. Every code reference must resolve (a node id, a path suffix as `path` takes, or {\"path\": {from, to, follow, ...}} standing for that route). `layout` steers placement: `chain` lists sequences laid out left to right on one row (e.g. the route you're explaining); `order` puts boxes first within their parent; `options` passes ELK layered options (e.g. {\"elk.spacing.nodeNode\": 30}). Or pass only `step` to jump the window to step N of the current view.",
            "inputSchema": {"type": "object", "properties": {
                "view": {"type": "object", "properties": {
                    "title": {"type": "string"},
                    "open": {"type": "array", "description": "Boxes to open (their containers open too)."},
                    "highlight": {"type": "array"},
                    "focus": {"description": "One ref: the camera fits it."},
                    "notes": {"type": "array", "items": {"type": "object", "properties": {"on": {}, "text": {"type": "string"}}, "required": ["text"]}},
                    "steps": {"type": "array", "items": {"type": "object", "properties": {
                        "focus": {}, "highlight": {"type": "array"}, "open": {"type": "array"}, "note": {"type": "string"}
                    }}},
                    "play": {"type": "boolean", "description": "Auto-advance the steps (a walkthrough); the user can pause."},
                    "layout": {"type": "object", "properties": {
                        "chain": {"type": "array", "items": {"type": "array"}},
                        "order": {"type": "array"},
                        "options": {"type": "object"}
                    }}
                }},
                "step": {"type": "integer", "minimum": 1}
            }}
        },
        {
            "name": "selection",
            "description": "What the user has selected in the window, and which step of the view they're on.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "source",
            "description": "Read snapshot source, batched: each ref is a node id/suffix (its span), `file`, `file:line` or `file:start-end` (repo-relative). Serves only snapshot files.",
            "inputSchema": {"type": "object", "properties": {
                "refs": {"type": "array", "items": {"type": "string"}, "minItems": 1}
            }, "required": ["refs"]}
        }
    ])
}
