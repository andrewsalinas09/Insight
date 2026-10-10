//! The build pipeline (ARCHITECTURE "pipeline"; ADR-0036, R3-01…R3-11).
//!
//! `build`: settings + approval -> export the commit (gix) -> install/check in
//! Insight's own tree with the tripwire -> discover projects -> run extractors
//! -> verify every file read against the tree's bytes -> write the snapshot ->
//! independent function count (the M0 pass).

pub mod discover;
pub mod export;
pub mod path;
pub mod paths;
pub mod run;
pub mod settings;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_engine::SnapshotWriter;
use insight_model::{FACT_SCHEMA_VERSION, NodeKind, Record};
use sha2::{Digest, Sha256};

/// A refusal with its own exit code, so scripts and Claude can tell them apart (R3-10).
#[derive(Debug)]
pub enum Refusal {
    NotInitialized(String),
    NeedsSettings(String),
    NeedsApproval(String),
    CheckFailed { command: String, code: i32 },
    Tripwire { command: String, files: Vec<PathBuf> },
    /// (file, extractor count, counter count, where they differ).
    CountMismatch(Vec<(String, usize, usize, String)>),
}

impl Refusal {
    pub fn exit_code(&self) -> i32 {
        match self {
            Refusal::NotInitialized(_) | Refusal::NeedsSettings(_) => 2,
            Refusal::NeedsApproval(_) => 3,
            Refusal::CheckFailed { .. } => 5,
            Refusal::Tripwire { .. } => 6,
            Refusal::CountMismatch(_) => 7,
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NotInitialized(p) => write!(f, "project `{p}` does not exist: run `insight init` first"),
            Refusal::NeedsSettings(m) => write!(f, "needs settings: {m}"),
            Refusal::NeedsApproval(diff) => {
                write!(f, "the build commands changed since they were approved; run `insight approve <project>` after checking:\n{diff}")
            }
            Refusal::CheckFailed { command, code } => {
                write!(f, "the project does not build (ADR-0036): `{command}` exited with {code}")
            }
            Refusal::Tripwire { command, files } => {
                write!(f, "tripwire (R3-02): the live repo changed while `{command}` ran:")?;
                for p in files {
                    write!(f, "\n  {}", p.display())?;
                }
                Ok(())
            }
            Refusal::CountMismatch(rows) => {
                write!(f, "function count mismatch (R3-11), file: extractor vs independent counter:")?;
                for (file, ours, theirs, at) in rows {
                    write!(f, "\n  {file}: {ours} vs {theirs} ({at})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Refusal {}

/// The log every command writes (R2-20: the terminal shows only its path).
pub struct Log {
    pub path: PathBuf,
    file: std::fs::File,
}

impl Log {
    /// A log inside an existing project; never creates a project folder.
    pub fn create(project: &str, verb: &str) -> Result<Self> {
        Self::create_in(&project_dir_checked(project)?.join("logs"), verb)
    }
    /// A log outside any project (e.g. `init` before the project exists).
    /// Names are unique per call: `create_new` never overwrites an earlier log.
    pub fn create_in(dir: &Path, verb: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
        for n in 0.. {
            let path = dir.join(if n == 0 { format!("{verb}-{secs}.log") } else { format!("{verb}-{secs}-{n}.log") });
            match std::fs::File::create_new(&path) {
                Ok(file) => return Ok(Log { path, file }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
        unreachable!()
    }
    pub fn line(&mut self, s: impl AsRef<str>) {
        let _ = writeln!(self.file, "{}", s.as_ref());
    }
    pub fn file(&self) -> &std::fs::File {
        &self.file
    }
}

// ---------------------------------------------------------------- init / approve

/// `insight init <project> <folder>…`: creates the project and its INSIGHT.md.
/// Each folder's repo is found by following `.git` upward (ADR-0045).
pub fn init(project: &str, folders: &[PathBuf], log: &mut Log) -> Result<PathBuf> {
    let dir = paths::project_dir(project)?;
    if dir.join("INSIGHT.md").exists() {
        bail!("project `{project}` already exists at {}", dir.display());
    }
    let mut repos = BTreeMap::new();
    for f in folders {
        let abs = std::fs::canonicalize(f).with_context(|| format!("{} does not exist", f.display()))?;
        let root = abs.ancestors().find(|a| a.join(".git").exists()).with_context(|| format!("{} is not inside a git repo", abs.display()))?;
        let root = strip_verbatim(root);
        let name = root.file_name().context("repo root has no name")?.to_string_lossy().to_string();
        log.line(format!("repo `{name}` at {}", root.display()));
        repos.insert(name, root);
    }
    std::fs::create_dir_all(&dir)?;
    let mut md = format!(
        "# {project}\n\nNotes for Claude about this project go here, in prose (ADR-0044).\n\n\
         Settings live in `insight` blocks. `install` commands (installs, codegen, anything that\n\
         rewrites) and `check` commands (the build gate, ADR-0036) run only in Insight's own copy\n\
         of the code (R3-02). Changing them needs `insight approve {project}` (R3-01).\n\n```insight\ntagging = \"on-request\"\n"
    );
    for (name, root) in &repos {
        md.push_str(&format!("\n[repo.\"{name}\"]\npath = \"{}\"\ninstall = []\ncheck = []\n", root.display().to_string().replace('\\', "/")));
    }
    md.push_str("```\n");
    std::fs::write(dir.join("INSIGHT.md"), md)?;
    log.line(format!("created {}", dir.join("INSIGHT.md").display()));
    Ok(dir)
}

/// `insight approve <project>`: records the current commands as approved (R3-01).
pub fn approve(project: &str, log: &mut Log) -> Result<()> {
    let dir = project_dir_checked(project)?;
    let s = settings::load(&dir)?;
    let cmds = settings::Commands::of(&s);
    let prev = settings::approved(&dir)?.unwrap_or(settings::Commands(Default::default()));
    log.line(format!("approving commands for `{project}`:\n{}", cmds.diff(&prev)));
    settings::approve(&dir, &cmds)
}

fn project_dir_checked(project: &str) -> Result<PathBuf> {
    let dir = paths::project_dir(project)?;
    if !dir.join("INSIGHT.md").exists() {
        return Err(Refusal::NotInitialized(project.to_string()).into());
    }
    Ok(dir)
}

fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s))
}

// ---------------------------------------------------------------- build

pub struct BuildOptions {
    pub commit: Option<String>,
}

pub struct Built {
    pub repo: String,
    pub snapshot: PathBuf,
    pub functions: usize,
}

pub fn build(project: &str, opts: &BuildOptions, log: &mut Log) -> Result<Vec<Built>> {
    let dir = project_dir_checked(project)?;
    let s = settings::load(&dir)?;
    let cmds = settings::Commands::of(&s);
    match settings::approved(&dir)? {
        Some(a) if a.fingerprint() == cmds.fingerprint() => {}
        Some(a) => return Err(Refusal::NeedsApproval(cmds.diff(&a)).into()),
        None => return Err(Refusal::NeedsApproval(cmds.diff(&settings::Commands(Default::default()))).into()),
    }
    if s.repo.is_empty() {
        return Err(Refusal::NeedsSettings("no [repo.\"…\"] table in INSIGHT.md".into()).into());
    }
    let Some(rev) = &opts.commit else {
        bail!("M0 builds commits only: pass --commit <rev> (working-copy snapshots come later)");
    };
    // Every repo is built; one that fails keeps its last good snapshot (R3-20) and the
    // build reports its refusal after the others are done.
    let mut out = Vec::new();
    let mut failed: Option<anyhow::Error> = None;
    for (name, repo) in &s.repo {
        match build_repo(project, name, repo, rev, log) {
            Ok(b) => out.push(b),
            Err(e) => {
                log.line(format!("== repo `{name}` FAILED: {e:#}"));
                failed.get_or_insert(e);
            }
        }
    }
    for b in &out {
        log.line(format!("built {}: {} functions -> {}", b.repo, b.functions, b.snapshot.display()));
    }
    match failed {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

fn build_repo(project: &str, name: &str, repo: &settings::RepoSettings, rev: &str, log: &mut Log) -> Result<Built> {
    if repo.check.is_empty() {
        return Err(Refusal::NeedsSettings(format!("[repo.\"{name}\"] has no `check` command (the build gate, ADR-0036)")).into());
    }
    let live = PathBuf::from(&repo.path);
    let commit = export::resolve(&live, rev)?;
    log.line(format!("== repo `{name}` at {commit}"));
    let tree = paths::tree_dir(&commit)?;
    let exported = export::export(&live, &commit, &tree)?;
    log.line(format!("exported {} files to {}", exported.files.len(), tree.display()));
    for (path, c) in &exported.submodules {
        log.line(format!("  submodule {path} at {c}"));
    }
    for s in &exported.skipped {
        log.line(format!("  not exported: {s}"));
    }

    // Snapshot key: everything that affects the facts (R3-08).
    let ts_script = std::fs::read(insight_lang_ts::script_path(&paths::cache_root()?)?)?;
    let rust_helper = std::fs::read(insight_lang_rust::helper_path()?)?;
    let cpp_helper = std::fs::read(insight_lang_cpp::helper_path()?)?;
    let mut h = Sha256::new();
    h.update(commit.as_bytes());
    h.update(FACT_SCHEMA_VERSION.to_le_bytes());
    h.update(insight_engine::DB_LAYOUT_VERSION.to_le_bytes());
    h.update(&ts_script);
    h.update(&rust_helper);
    h.update(&cpp_helper);
    h.update(settings::Commands::of(&settings::Settings { tagging: String::new(), repo: [(name.to_string(), repo.clone())].into() }).fingerprint());
    let key: String = h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
    let snap_path = paths::snapshot_path(project, name, &key)?;

    // Install and check, in Insight's tree only, watched by the tripwire (R3-02).
    let toolchain = rustc_version().unwrap_or_else(|_| "unknown".into());
    let target = paths::cargo_target_dir(project, &toolchain)?;
    let env = vec![("CARGO_TARGET_DIR".to_string(), target.display().to_string())];
    for (kind, list) in [("install", &repo.install), ("check", &repo.check)] {
        for c in list {
            log.line(format!("-- {kind}: {c}"));
            match run::run(c, &tree, &env, log.file(), &live)? {
                run::Outcome::Ok => {}
                run::Outcome::Failed(code) => return Err(Refusal::CheckFailed { command: c.clone(), code }.into()),
                run::Outcome::Tripped(files) => return Err(Refusal::Tripwire { command: c.clone(), files }.into()),
            }
        }
    }

    // Extract.
    let projects = discover::discover(&exported.files, |p| std::fs::read_to_string(tree.join(p)).ok());
    log.line(format!("rust packages: {:?}", projects.rust_packages));
    log.line(format!("ts projects: {:?}", projects.ts_projects));
    let compdbs = discover::compile_databases(&tree);
    log.line(format!("compilation databases: {:?}", compdbs.iter().map(|p| p.strip_prefix(&tree).unwrap_or(p).display().to_string()).collect::<Vec<_>>()));
    let mut w = SnapshotWriter::create(&snap_path)?;
    w.meta("project", project)?;
    w.meta("repo", name)?;
    w.meta("commit", &commit)?;
    w.meta("fact_schema", &FACT_SCHEMA_VERSION.to_string())?;
    let mut tally = Tally::default();
    let cache = paths::cache_root()?;
    if !projects.ts_projects.is_empty() {
        log.line(format!("node {}", insight_lang_ts::node_version()?));
        let cmd = insight_lang_ts::command(&cache, &tree, &projects.ts_projects, FACT_SCHEMA_VERSION)?;
        ingest(cmd, "ts", &tree, &exported, &mut w, &mut tally, log)?;
    }
    if !projects.rust_packages.is_empty() {
        let mut cmd = insight_lang_rust::command(&tree, &projects.rust_packages, FACT_SCHEMA_VERSION)?;
        // rust-analyzer runs build scripts; they share the check's private target (R3-16).
        cmd.envs(env.iter().map(|(k, v)| (k, v)));
        ingest(cmd, "rust", &tree, &exported, &mut w, &mut tally, log)?;
    }
    if !compdbs.is_empty() {
        let cmd = insight_lang_cpp::command(&tree, &compdbs, FACT_SCHEMA_VERSION)?;
        ingest(cmd, "cpp", &tree, &exported, &mut w, &mut tally, log)?;
    }
    link_literals(&mut w, &tally, log)?;
    let (calls, cands) = w.resolve_indirect_calls(tally.next_group)?;
    log.line(format!("calls through variables and fields: {calls} resolved to {cands} possible candidates"));

    // Never silently incomplete: source files no project covered are stated.
    for path in exported.files.keys() {
        let in_deps = path.split('/').any(|s| s == "node_modules");
        if !in_deps && insight_count::Lang::from_path(path).is_some() && !tally.files.contains_key(path) {
            log.line(format!("  not analyzed (no project covers it): {path}"));
            w.diagnostic(&insight_model::Diagnostic {
                file: Some(path.clone()),
                severity: insight_model::Severity::Info,
                message: "not analyzed: no Cargo package, tsconfig or compiled translation unit covers this file".into(),
            })?;
        }
    }

    // The M0 pass: an independent count must match exactly, per file (R3-11).
    let mut mismatches = Vec::new();
    let mut total = 0;
    let mut unchecked_total = 0;
    for (file, content) in &tally.files {
        let Some(lang) = insight_count::Lang::from_path(file) else { continue };
        let skip: Vec<(usize, usize)> = tally.inactive.get(file).cloned().unwrap_or_default();
        // Where the counter's own parse failed, it cannot vouch. A function is identified on
        // both sides by its last line (both agree where a body ends; starts differ with
        // templates and attributes): any last line where either side's function touches a
        // failed parse is left out of both counts and reported as unchecked.
        let unparsed = insight_count::unparsed_ranges(lang, content)?;
        let touches = |s: usize, e: usize| unparsed.iter().any(|(a, b)| s < *b && *a < e);
        let line_of = |b: usize| content.as_bytes()[..b.min(content.len())].iter().filter(|c| **c == b'\n').count() as u32 + 1;
        let theirs_all: Vec<(u32, bool)> = insight_count::function_spans(lang, content, &skip)?.iter().map(|(s, e)| (line_of(e.saturating_sub(1)), touches(*s, *e))).collect();
        let mine_all: Vec<(u32, bool)> = tally.function_spans.get(file).cloned().unwrap_or_default().iter().map(|(s, e, l)| (*l, touches(*s, *e))).collect();
        let excluded: std::collections::BTreeSet<u32> = theirs_all.iter().chain(&mine_all).filter(|(_, t)| *t).map(|(l, _)| *l).collect();
        let unchecked = mine_all.iter().filter(|(l, _)| excluded.contains(l)).count();
        if !excluded.is_empty() {
            unchecked_total += unchecked;
            log.line(format!("  unchecked: {file}: {unchecked} function(s) where the independent counter could not parse (unexpanded macros)"));
            w.diagnostic(&insight_model::Diagnostic {
                file: Some(file.clone()),
                severity: insight_model::Severity::Info,
                message: format!("{unchecked} function(s) not verified by the independent count: its parser failed there (unexpanded macros)"),
            })?;
        }
        let mut a: Vec<u32> = mine_all.iter().map(|(l, _)| *l).filter(|l| !excluded.contains(l)).collect();
        let mut b: Vec<u32> = theirs_all.iter().map(|(l, _)| *l).filter(|l| !excluded.contains(l)).collect();
        let (ours, theirs) = (a.len(), b.len());
        total += tally.functions.get(file).copied().unwrap_or(0);
        if ours != theirs {
            // What only one side has, as multisets of last lines.
            a.sort_unstable();
            b.sort_unstable();
            let (only_a, only_b) = multiset_difference(&a, &b);
            let show = |v: &[u32]| v.iter().take(8).map(u32::to_string).collect::<Vec<_>>().join(",");
            let at = format!("functions ending on lines only the extractor found: [{}]; only the counter: [{}]", show(&only_a), show(&only_b));
            mismatches.push((file.clone(), ours, theirs, at));
        }
    }
    log.line(format!(
        "functions: {total} in {} files; independent count mismatches: {}; unchecked (counter could not parse): {unchecked_total}",
        tally.files.len(),
        mismatches.len()
    ));
    if !mismatches.is_empty() {
        return Err(Refusal::CountMismatch(mismatches).into());
    }
    let snapshot = w.finish()?;
    log.line(format!("snapshot: {}", snapshot.display()));
    let conn = insight_engine::open_snapshot(&snapshot)?;
    for (lang, kind, n) in insight_engine::node_counts(&conn)? {
        log.line(format!("  {lang:5} {kind:19} {n}"));
    }
    Ok(Built { repo: name.to_string(), snapshot, functions: total })
}

#[derive(Default)]
struct Tally {
    /// file -> its text (for the independent count and receipt columns).
    files: BTreeMap<String, String>,
    /// Functions written in each file (not macro-generated; SCHEMA.md §6).
    functions: BTreeMap<String, usize>,
    /// The byte span and last line of each of those functions, to check and show where a count differs.
    function_spans: BTreeMap<String, Vec<(usize, usize, u32)>>,
    /// Inactive `#if` regions per file: the counter skips functions written there.
    inactive: BTreeMap<String, Vec<(usize, usize)>>,
    /// Every literal occurrence, for the link passes after all extractors ran.
    literals: Vec<insight_model::LiteralFact>,
    /// Next free snapshot-wide group id (extractor group numbers are local).
    next_group: i64,
}

fn edge_kind(k: insight_model::EdgeKind) -> &'static str {
    match k {
        insight_model::EdgeKind::Call => "call",
        insight_model::EdgeKind::ValueFlow => "value-flow",
        insight_model::EdgeKind::Read => "read",
        insight_model::EdgeKind::Write => "write",
        insight_model::EdgeKind::Access => "access",
        insight_model::EdgeKind::Binds => "binds",
    }
}

fn resolution(r: insight_model::Resolution) -> &'static str {
    match r {
        insight_model::Resolution::Exact => "exact",
        insight_model::Resolution::Possible => "possible",
        insight_model::Resolution::Unresolved => "unresolved",
    }
}

/// Streams one extractor's NDJSON into the snapshot, checking the protocol (R2-28)
/// and that every file it read is byte-identical to the exported tree (R3-05).
/// Edges are inserted once the stream is complete, so they may name any node it sent.
#[allow(clippy::too_many_arguments)]
fn ingest(
    mut cmd: std::process::Command,
    lang: &str,
    tree: &Path,
    exported: &export::Exported,
    w: &mut SnapshotWriter,
    tally: &mut Tally,
    log: &mut Log,
) -> Result<()> {
    cmd.stderr(log.file().try_clone()?);
    let mut child = cmd.spawn().with_context(|| format!("starting the {lang} extractor"))?;
    let stdout = child.stdout.take().expect("piped");
    let mut sent = insight_model::End::default();
    let mut header = false;
    let mut end = None;
    let mut file_lang: BTreeMap<String, String> = BTreeMap::new();
    let mut edges = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        let rec: Record = serde_json::from_str(&line).with_context(|| format!("{lang} extractor sent a bad record: {line}"))?;
        if end.is_some() {
            bail!("{lang} extractor sent records after `end`");
        }
        match rec {
            Record::Header(h) => {
                if h.schema != FACT_SCHEMA_VERSION {
                    bail!("{lang} extractor speaks schema {}, expected {FACT_SCHEMA_VERSION}", h.schema);
                }
                log.line(format!("{lang} extractor {} on {}", h.extractor, h.toolchain));
                w.meta(&format!("extractor.{lang}"), &format!("{} / {}", h.extractor, h.toolchain))?;
                header = true;
            }
            _ if !header => bail!("{lang} extractor did not start with a header"),
            Record::File(f) => {
                let bytes = std::fs::read(tree.join(&f.path))?;
                let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
                if sha != f.sha256 {
                    bail!("{} changed while it was being read (R3-05)", f.path);
                }
                if !exported.files.contains_key(&f.path) {
                    log.line(format!("  note: {} is not from git (generated by an install step)", f.path));
                }
                w.file(&f, &bytes)?;
                tally.files.insert(f.path.clone(), String::from_utf8_lossy(&bytes).into_owned());
                file_lang.insert(f.path, f.lang);
                sent.files += 1;
            }
            Record::Node(n) if n.kind == NodeKind::External => {
                // One leaf per symbol (BQ-20); the same symbol may be named by both languages' extractors.
                if w.nid(&n.id).is_none() {
                    w.node(&n, lang, None)?;
                }
                sent.nodes += 1;
            }
            Record::Node(n) => {
                let file = n.id.split(" › ").next().unwrap_or(&n.id).to_string();
                let flang = file_lang.get(&file).with_context(|| format!("node `{}` before its file record", n.id))?.clone();
                if n.kind == NodeKind::Function && !n.generated {
                    *tally.functions.entry(file.clone()).or_default() += 1;
                    if let Some(s) = n.span {
                        tally.function_spans.entry(file.clone()).or_default().push((s.byte_start as usize, s.byte_end as usize, s.line_end));
                    }
                }
                if let (NodeKind::Inactive, Some(s)) = (n.kind, n.span) {
                    tally.inactive.entry(file.clone()).or_default().push((s.byte_start as usize, s.byte_end as usize));
                }
                w.node(&n, &flang, Some(&file))?;
                sent.nodes += 1;
            }
            Record::Edge(e) => {
                edges.push(e);
                sent.edges += 1;
            }
            Record::Literal(l) => {
                tally.literals.push(l);
                sent.literals += 1;
            }
            Record::Diagnostic(d) => {
                log.line(format!("  {lang} {:?}: {} {}", d.severity, d.file.as_deref().unwrap_or(""), d.message));
                w.diagnostic(&d)?;
                sent.diagnostics += 1;
            }
            Record::End(e) => end = Some(e),
        }
    }
    let status = child.wait()?;
    if !status.success() {
        bail!("the {lang} extractor failed ({status}); see the log");
    }
    let end = end.with_context(|| format!("the {lang} extractor stopped without an `end` record (truncated)"))?;
    if end != sent {
        bail!("the {lang} extractor's end counts {end:?} do not match what it sent {sent:?}");
    }

    let base = tally.next_group;
    let mut max_group = 0;
    for e in &edges {
        let from = w.nid(&e.from).with_context(|| format!("{lang} edge from unknown node `{}`", e.from))?;
        let to = match &e.to {
            Some(t) => Some(w.nid(t).with_context(|| format!("{lang} edge to unknown node `{t}`"))?),
            None => None,
        };
        let group = e.group.map(|g| {
            max_group = max_group.max(g as i64);
            base + g as i64
        });
        let eid = w.edge(edge_kind(e.kind), e.subkind.as_deref(), from, to, resolution(e.resolution), group)?;
        for r in &e.receipts {
            w.receipt(eid, r)?;
        }
    }
    tally.next_group = base + max_group + 1;
    log.line(format!("{lang}: {} files, {} nodes, {} edges, {} literals, {} diagnostics", sent.files, sent.nodes, sent.edges, sent.literals, sent.diagnostics));
    Ok(())
}

/// The generic link passes (BQ-22, ADR-0047, ADR-0048), across languages:
/// one `literal` node per distinct value (strings `lit:"x"`, numbers `lit:31`) with a
/// `link/literal` edge from every function containing it, and `link/names` edges from
/// a string literal to every declaration (function, type, module) whose declared name
/// equals it.
fn link_literals(w: &mut SnapshotWriter, tally: &Tally, log: &mut Log) -> Result<()> {
    let mut by_value: BTreeMap<(bool, &str), Vec<&insight_model::LiteralFact>> = BTreeMap::new();
    for l in &tally.literals {
        by_value.entry((l.number, l.value.as_str())).or_default().push(l);
    }
    let decls = w.named_declarations()?;
    let (mut occurrences, mut names) = (0, 0);
    for ((number, value), occs) in &by_value {
        let path = if *number { format!("lit:{value}") } else { format!("lit:{}", serde_json::to_string(value)?) };
        let lit = w.raw_node(&path, None, "literal", None, value, false, None, None, None)?;
        for o in occs {
            let from = w.nid(&o.from).with_context(|| format!("literal in unknown node `{}`", o.from))?;
            let eid = w.edge("link", Some("literal"), from, Some(lit), "exact", None)?;
            w.receipt(eid, &o.receipt)?;
            occurrences += 1;
        }
        for d in decls.get(*value).into_iter().flatten().filter(|_| !number) {
            let eid = w.edge("link", Some("names"), lit, Some(d.nid), "exact", None)?;
            if let Some(text) = tally.files.get(&d.file) {
                w.receipt(eid, &receipt_at(text, &d.file, d.byte_start, d.byte_end, "decl", Some(d.path.clone())))?;
            }
            names += 1;
        }
    }
    log.line(format!("links: {} distinct literals, {occurrences} occurrences, {names} names matches", by_value.len()));
    Ok(())
}

/// A receipt for a byte range of a file, with 1-based line/col (UTF-8 bytes, BQ-11).
fn receipt_at(text: &str, file: &str, s: u64, e: u64, role: &str, container: Option<String>) -> insight_model::Receipt {
    let lc = |b: u64| {
        let before = &text.as_bytes()[..(b as usize).min(text.len())];
        let line = before.iter().filter(|c| **c == b'\n').count() as u32 + 1;
        let col = before.iter().rev().take_while(|c| **c != b'\n').count() as u32 + 1;
        (line, col)
    };
    let ((line, col), (end_line, end_col)) = (lc(s), lc(e));
    insight_model::Receipt { role: role.into(), file: file.into(), byte_start: s, byte_end: e, line, col, end_line, end_col, container }
}

/// The newest complete snapshot of each repo of a project, sorted by repo name.
pub fn latest_per_repo(project: &str) -> Result<Vec<(String, PathBuf)>> {
    let dir = paths::snapshots_dir(project)?;
    let mut best: BTreeMap<String, (std::time::SystemTime, PathBuf)> = BTreeMap::new();
    for e in std::fs::read_dir(&dir).with_context(|| format!("no snapshots for `{project}`: run `insight build` first"))? {
        let p = e?.path();
        if p.extension().is_none_or(|x| x != "sqlite") {
            continue;
        }
        let Ok(conn) = insight_engine::open_snapshot(&p) else { continue };
        let repo: String = conn.query_row("SELECT value FROM meta WHERE key = 'repo'", [], |r| r.get(0))?;
        let t = std::fs::metadata(&p)?.modified()?;
        if best.get(&repo).is_none_or(|(bt, _)| t > *bt) {
            best.insert(repo, (t, p));
        }
    }
    anyhow::ensure!(!best.is_empty(), "no snapshots for `{project}`: run `insight build` first");
    Ok(best.into_iter().map(|(r, (_, p))| (r, p)).collect())
}

/// A repo's SQL schema name when several repos are loaded together: lowercase letters,
/// digits and `_`, starting with a letter.
pub fn schema_alias(repo: &str) -> String {
    let mut a: String = repo.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    if !a.starts_with(|c: char| c.is_ascii_alphabetic()) {
        a.insert(0, 'r');
    }
    if a == "main" || a == "temp" {
        a.push('_');
    }
    a
}

/// The newest complete snapshot of a project (any repo), unless one is named.
pub fn latest_snapshot(project: &str) -> Result<PathBuf> {
    let dir = paths::snapshots_dir(project)?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(&dir).with_context(|| format!("no snapshots for `{project}`: run `insight build` first"))? {
        let p = e?.path();
        if p.extension().is_some_and(|x| x == "sqlite") {
            let t = std::fs::metadata(&p)?.modified()?;
            if best.as_ref().is_none_or(|(bt, _)| t > *bt) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p).with_context(|| format!("no snapshots for `{project}`: run `insight build` first"))
}

fn rustc_version() -> Result<String> {
    let out = std::process::Command::new("rustc").arg("--version").output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}


/// (a − b, b − a) of two sorted multisets.
fn multiset_difference(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let (mut i, mut j, mut oa, mut ob) = (0, 0, Vec::new(), Vec::new());
    while i < a.len() || j < b.len() {
        match (a.get(i), b.get(j)) {
            (Some(x), Some(y)) if x == y => {
                i += 1;
                j += 1;
            }
            (Some(x), Some(y)) if x < y => {
                oa.push(*x);
                i += 1;
            }
            (Some(_), Some(y)) => {
                ob.push(*y);
                j += 1;
            }
            (Some(x), None) => {
                oa.push(*x);
                i += 1;
            }
            (None, Some(y)) => {
                ob.push(*y);
                j += 1;
            }
            (None, None) => break,
        }
    }
    (oa, ob)
}