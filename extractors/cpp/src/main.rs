//! Insight's C/C++ extractor (ADR-0034): libclang over the project's own
//! compilation database (`compile_commands.json`, produced by the project's
//! build, ADR-0036) → the standard representation (ADR-0033, ADR-0048).
//!
//! Usage: insight-extract-cpp --root <dir> --compdb <compile_commands.json> [--compdb …] --schema <N>
//!
//! - **Which code:** every translation unit whose main file is inside `--root`, and every
//!   header inside `--root` they include. Each file is analyzed once, by the first
//!   translation unit that reaches it (sorted, so deterministic). Declarations in files
//!   outside the root are external leaves.
//! - **How each command is understood:** `cl`/`clang-cl` commands use clang's cl mode;
//!   clang commands pass through. Other compilers (gcc-style cross compilers) are asked
//!   for their own include paths and predefined macros (`-E -dM -v`), and clang parses
//!   with exactly those (`-undef -nostdinc`), so the code sees the same configuration it
//!   was built with.
//! - **Facts:** files; containers (namespaces as modules, class/struct/union/enum
//!   definitions as types, every function with a written body including lambdas, and
//!   control-flow blocks); inactive `#if` regions; calls (exact, or `possible` candidates
//!   for virtual dispatch: the named method if it has a body plus every override in the
//!   analyzed code); handoffs of functions and lambdas to external code; string and
//!   integer literals; and data flow (`flow.rs`): variables, parameters, fields and
//!   globals as state, their reads and writes, and values moving by assignment,
//!   argument binding and return. A call through a variable or field is a call edge
//!   to that state; the solver finds the functions that can reach it.
//!
//! Spans and receipts use file locations: code written in macro arguments points at its
//! own text; code from a macro's template points at the macro call and is `generated`.

mod flow;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clang::source::SourceRange;
use clang::{Clang, CompilationDatabase, Entity, EntityKind, EvaluationResult, Index};
use insight_model::{
    Diagnostic, EdgeFact, EdgeKind, End, FACT_SCHEMA_VERSION, FileFact, Header, LiteralFact, NodeFact, NodeKind, Receipt, Record, Resolution, Severity, Span,
};
use sha2::{Digest, Sha256};

fn main() -> Result<()> {
    // Deeply nested expressions recurse deeply; give the walk room (as the Rust extractor does).
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run)?;
    worker.join().map_err(|_| anyhow::anyhow!("the C/C++ extractor panicked"))?
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut root, mut compdbs, mut schema) = (None, Vec::new(), None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--root" => root = args.next().map(PathBuf::from),
            "--compdb" => compdbs.extend(args.next().map(PathBuf::from)),
            "--schema" => schema = args.next().map(|s| s.parse::<u32>()).transpose()?,
            other => bail!("unknown argument `{other}`"),
        }
    }
    let root = root.context("--root is required")?;
    if schema != Some(FACT_SCHEMA_VERSION) {
        bail!("schema mismatch: asked {schema:?}, extractor speaks {FACT_SCHEMA_VERSION}");
    }
    let clang = Clang::new().map_err(|e| anyhow::anyhow!("cannot load libclang: {e}"))?;
    let index = Index::new(&clang, false, false);
    let mut out = Out { w: BufWriter::new(std::io::stdout().lock()), end: End::default() };
    out.emit(Record::Header(Header {
        schema: FACT_SCHEMA_VERSION,
        extractor: format!("cpp@{}", env!("CARGO_PKG_VERSION")),
        toolchain: clang::get_version(),
    }))?;

    let root_key = key(&canon(&root));
    let mut x = Extractor { out: Some(out), root_key, files: HashMap::new(), ..Default::default() };

    // Every command, sorted by file, inside the root only.
    let mut commands: Vec<(PathBuf, PathBuf, Vec<String>)> = Vec::new();
    for db in &compdbs {
        let dir = db.parent().context("compdb has no folder")?;
        let cdb = CompilationDatabase::from_directory(dir).map_err(|_| anyhow::anyhow!("cannot read {}", db.display()))?;
        for c in cdb.get_all_compile_commands().get_commands() {
            let file = c.get_directory().join(c.get_filename());
            commands.push((canon(&file), c.get_directory(), c.get_arguments()));
        }
    }
    commands.sort_by(|a, b| a.0.cmp(&b.0));
    commands.dedup_by(|a, b| a.0 == b.0);
    let mut drivers = Drivers::default();
    for (n, (file, dir, argv)) in commands.iter().enumerate() {
        if x.rel(file).is_none() {
            continue;
        }
        x.tu = n;
        let args = match drivers.clang_args(&index, file, dir, argv) {
            Ok(a) => a,
            Err(e) => {
                x.diag(x.rel(file), Severity::Error, format!("cannot translate the compile command: {e:#}"))?;
                continue;
            }
        };
        eprintln!("cpp: {}", file.display());
        let tu = match index.parser(file).arguments(&args).detailed_preprocessing_record(true).keep_going(true).parse() {
            Ok(tu) => tu,
            Err(e) => {
                x.diag(x.rel(file), Severity::Error, format!("clang could not parse this translation unit: {e}"))?;
                continue;
            }
        };
        let errors: Vec<String> = tu
            .get_diagnostics()
            .into_iter()
            .filter(|d| matches!(d.get_severity(), clang::diagnostic::Severity::Error | clang::diagnostic::Severity::Fatal))
            .map(|d| {
                let at = d.get_location().get_file_location();
                let file = at.file.map(|f| f.get_path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).unwrap_or_default();
                format!("{file}:{}: {}", at.line, d.get_text())
            })
            .collect();
        if !errors.is_empty() {
            let shown: Vec<&str> = errors.iter().take(5).map(String::as_str).collect();
            x.diag(x.rel(file), Severity::Warning, format!("{} clang error(s); facts from erroneous code may be missing: {}", errors.len(), shown.join(" | ")))?;
        }
        x.file_cache.clear();
        let top = tu.get_entity();
        let file_scope = Ctx::default();
        for c in top.get_children() {
            x.visit_top(c, &file_scope)?;
        }
        let skipped: Vec<(PathBuf, usize, usize)> = tu
            .get_skipped_ranges()
            .iter()
            .filter_map(|r| {
                let (s, e) = (r.get_start().get_file_location(), r.get_end().get_file_location());
                Some((canon(&s.file?.get_path()), s.offset as usize, e.offset as usize))
            })
            .collect();
        x.inactive_regions(&skipped)?;
    }
    x.finish()
}

// ------------------------------------------------------------------ output

struct Out {
    w: BufWriter<std::io::StdoutLock<'static>>,
    end: End,
}

impl Out {
    fn emit(&mut self, r: Record) -> Result<()> {
        match &r {
            Record::File(_) => self.end.files += 1,
            Record::Node(_) => self.end.nodes += 1,
            Record::Edge(_) => self.end.edges += 1,
            Record::Literal(_) => self.end.literals += 1,
            Record::Diagnostic(_) => self.end.diagnostics += 1,
            _ => {}
        }
        serde_json::to_writer(&mut self.w, &r)?;
        self.w.write_all(b"\n")?;
        Ok(())
    }
}

// ------------------------------------------------------------------ paths

fn canon(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let s = c.to_string_lossy();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s).to_string())
}

/// Comparison key: forward slashes, lowercase on Windows (case-insensitive file system).
fn key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    if cfg!(windows) { s.to_lowercase() } else { s }
}

// ------------------------------------------------------------------ compile commands → clang arguments

#[derive(Default)]
struct Drivers {
    /// (compiler, target flags) → extra clang arguments from asking that compiler.
    queried: HashMap<String, Vec<String>>,
}

impl Drivers {
    fn clang_args(&mut self, index: &Index, file: &Path, dir: &Path, argv: &[String]) -> Result<Vec<String>> {
        let compiler = argv.first().context("empty compile command")?;
        let name = Path::new(compiler).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let mode = if name == "cl" || name == "clang-cl" {
            Mode::Cl
        } else if name.contains("clang") {
            Mode::Clang
        } else {
            Mode::Gcc
        };
        let file_key = key(&canon(file));
        let mut out = vec![format!("-working-directory={}", dir.display()), "-ferror-limit=0".into()];
        if mode == Mode::Cl {
            out.insert(0, "--driver-mode=cl".into());
        }
        let mut target_flags = Vec::new();
        let mut it = argv.iter().skip(1).peekable();
        while let Some(a) = it.next() {
            if key(&canon(&dir.join(a))) == file_key {
                continue;
            }
            let takes_value = |flags: &[&str]| flags.contains(&a.as_str());
            match mode {
                Mode::Cl | Mode::Clang => {
                    if matches!(a.as_str(), "-c" | "/c" | "/FS") || ["/Fo", "-Fo", "/Fd", "-Fd", "/Fe", "-Fe"].iter().any(|p| a.starts_with(p)) {
                        continue;
                    }
                    if a == "-o" || (mode == Mode::Clang && takes_value(&["-MF", "-MT", "-MQ"])) {
                        it.next();
                        continue;
                    }
                    if mode == Mode::Clang && matches!(a.as_str(), "-MD" | "-MMD") {
                        continue;
                    }
                    out.push(a.clone());
                }
                Mode::Gcc => {
                    // Only what selects source and configuration; the compiler's own target
                    // settings come from asking it (below).
                    const VALUED: &[&str] = &["-I", "-isystem", "-iquote", "-idirafter", "-D", "-U", "-include", "-imacros", "-x"];
                    if VALUED.contains(&a.as_str()) {
                        out.push(a.clone());
                        out.extend(it.next().cloned());
                    } else if VALUED.iter().any(|p| a.starts_with(p) && a.len() > p.len()) {
                        out.push(a.clone());
                    } else if a.starts_with("-std=") {
                        // The language version selects both what clang parses and which macros
                        // the compiler predefines (gcc 15 defaults C to C23).
                        out.push(a.clone());
                        target_flags.push(a.clone());
                    } else if a.starts_with("-m") || a.starts_with("-f") || a.starts_with("--sysroot") || a.starts_with("-std=") || a.starts_with("-O") {
                        target_flags.push(a.clone());
                    }
                }
            }
        }
        if mode == Mode::Gcc {
            let cxx = argv.iter().any(|a| a == "-x" || a.starts_with("-std=c++") || a.starts_with("-std=gnu++"))
                || matches!(file.extension().and_then(|e| e.to_str()), Some("cpp" | "cc" | "cxx" | "C" | "c++"));
            let qkey = format!("{compiler}\u{0}{cxx}\u{0}{}", target_flags.join("\u{0}"));
            if !self.queried.contains_key(&qkey) {
                let extra = query_gcc(index, compiler, dir, &target_flags, cxx)?;
                self.queried.insert(qkey.clone(), extra);
            }
            let mut extra = self.queried[&qkey].clone();
            extra.extend(out);
            out = extra;
        }
        Ok(out)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Cl,
    Clang,
    Gcc,
}

/// Asks a gcc-style compiler for its include search path and predefined macros, and
/// returns clang arguments that reproduce that configuration.
fn query_gcc(index: &Index, compiler: &str, dir: &Path, target_flags: &[String], cxx: bool) -> Result<Vec<String>> {
    let out = std::process::Command::new(compiler)
        .current_dir(dir)
        .args(target_flags)
        .args(["-x", if cxx { "c++" } else { "c" }, "-E", "-dM", "-v", "-"])
        .stdin(std::process::Stdio::null())
        .output()
        .with_context(|| format!("running {compiler} to read its configuration"))?;
    // gcc announces its extended floating types (`_Float32`, `_Float64x`, …) through these
    // macros; clang has none of them, and headers that see the macros would use the types.
    let macros: String = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !["__FLT32", "__FLT64", "__FLT128", "__BFLT16", "__STDCPP_FLOAT", "__STDCPP_BFLOAT"].iter().any(|p| l.starts_with(&format!("#define {p}"))))
        .map(|l| format!("{l}\n"))
        .chain(
            // The parser is clang: headers that adapt to clang (types it builds in) must see it.
            ["#define __clang__ 1\n".to_string()],
        )
        .collect();
    let err = String::from_utf8_lossy(&out.stderr);
    let mut includes = Vec::new();
    let mut inside = false;
    for line in err.lines() {
        if line.starts_with("#include <...> search starts here") || line.starts_with("#include \"...\" search starts here") {
            inside = true;
        } else if line.starts_with("End of search list") {
            inside = false;
        } else if inside {
            includes.push(canon(Path::new(line.trim())).display().to_string());
        }
    }
    if macros.is_empty() {
        bail!("{compiler} printed no predefined macros");
    }
    let mut h = Sha256::new();
    h.update(macros.as_bytes());
    let name: String = h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect();
    let header = std::env::temp_dir().join(format!("insight-cpp-macros-{name}.h"));
    std::fs::write(&header, &macros)?;
    let triple = std::process::Command::new(compiler).arg("-dumpmachine").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let wide = macros.contains("#define __SIZEOF_POINTER__ 8");
    let fallback = if wide { "x86_64-unknown-linux-gnu" } else { "i686-unknown-linux-gnu" };
    let target = match triple.filter(|t| !t.is_empty()) {
        Some(t) if clang_accepts_target(index, &t) => t,
        _ => fallback.to_string(),
    };
    let mut args = vec![format!("--target={target}"), "-undef".into(), "-nostdinc".into(), "-include".into(), header.display().to_string()];
    for i in includes {
        args.push("-isystem".into());
        args.push(i);
    }
    Ok(args)
}

fn clang_accepts_target(index: &Index, triple: &str) -> bool {
    let probe = clang::Unsaved::new("insight_probe.c", "int x;\n");
    index
        .parser("insight_probe.c")
        .arguments(&[format!("--target={triple}")])
        .unsaved(&[probe])
        .parse()
        .is_ok_and(|tu| tu.get_diagnostics().iter().all(|d| !matches!(d.get_severity(), clang::diagnostic::Severity::Error | clang::diagnostic::Severity::Fatal)))
}

// ------------------------------------------------------------------ the walk

struct FileInfo {
    bytes: Vec<u8>,
    /// Byte offset of each line start.
    lines: Vec<usize>,
}

impl FileInfo {
    fn line_col(&self, off: usize) -> (u32, u32) {
        let i = self.lines.partition_point(|s| *s <= off).saturating_sub(1);
        (i as u32 + 1, (off - self.lines[i]) as u32 + 1)
    }
}

/// Where the walk is: the nearest path segment (scope), the nearest container (parent,
/// may be a block), the innermost function (or the file) for edges, and the name a
/// lambda gets from where it appears.
#[derive(Clone, Default)]
struct Ctx {
    file: String,
    scope: String,
    parent: String,
    func: String,
    lambda_name: Option<String>,
}

struct PendingCall {
    from: String,
    /// USRs that name the callee (the declaration, then its template), in order.
    usrs: Vec<String>,
    external: Option<(String, String)>,
    dynamic: bool,
    implicit: bool,
    constructor: bool,
    /// A call through a variable, parameter or field: the state called.
    via: Option<flow::Ref>,
    receipt: Receipt,
}

struct PendingHandoff {
    from: String,
    /// A function by USR, or a lambda by `file@offset`.
    target: String,
    receipt: Receipt,
}

#[derive(Default)]
struct Extractor {
    out: Option<Out>,
    root_key: String,
    tu: usize,
    /// key(abs path) → (owning TU, repo-relative path), for files inside the root.
    owner: HashMap<String, (usize, String)>,
    files: HashMap<String, FileInfo>,
    /// Per TU: libclang file id → repo-relative path if this TU analyzes it.
    file_cache: HashMap<(u64, u64, u64), Option<String>>,
    ordinals: HashMap<String, HashMap<String, usize>>,
    fn_by_usr: HashMap<String, String>,
    lambda_by_pos: HashMap<String, String>,
    /// overridden method USR → overriding method USRs.
    overriders: BTreeMap<String, Vec<String>>,
    calls: Vec<PendingCall>,
    handoffs: Vec<PendingHandoff>,
    group: u64,
    /// Files this TU analyzes, with their skipped ranges (emitted after the walk).
    claimed: Vec<String>,
    /// Spans of the containers the walk found, per file, during one TU.
    found: HashMap<String, Vec<(usize, usize)>>,
    state_by_usr: HashMap<String, String>,
    /// Function USR → its parameters' state ids, in order.
    fn_params: HashMap<String, Vec<String>>,
    data_edges: Vec<flow::PendingData>,
    /// Expressions in a write position (not reads): (file, offset).
    skip_read: HashSet<(String, usize)>,
    externals: BTreeMap<String, String>,
    /// Type definitions already walked: (file, start, end).
    types_seen: HashSet<(String, usize, usize)>,
    /// Every node id emitted (ids are unique per snapshot).
    ids: HashSet<String>,
}

impl Extractor {
    /// Registers an external leaf (emitted at the end); returns its id.
    fn external(&mut self, id: &str, name: &str) -> String {
        self.externals.entry(id.to_string()).or_insert_with(|| name.to_string());
        id.to_string()
    }

    fn out(&mut self) -> &mut Out {
        self.out.as_mut().expect("output")
    }

    fn emit(&mut self, r: Record) -> Result<()> {
        self.out().emit(r)
    }

    fn diag(&mut self, file: Option<String>, severity: Severity, message: String) -> Result<()> {
        self.emit(Record::Diagnostic(Diagnostic { file, severity, message }))
    }

    /// The repo-relative path of an absolute path inside the root.
    fn rel(&self, abs: &Path) -> Option<String> {
        let k = key(abs);
        let r = k.strip_prefix(&self.root_key)?.strip_prefix('/')?;
        // Keep the file system's case: cut the same number of characters from the real path.
        let real = abs.to_string_lossy().replace('\\', "/");
        let cut = real.len() - r.len();
        Some(real[cut..].to_string())
    }

    /// The repo-relative path of a libclang file if this TU analyzes it; claims it on first sight.
    fn claim(&mut self, f: &clang::source::File) -> Result<Option<String>> {
        let id = f.get_id();
        if let Some(r) = self.file_cache.get(&id) {
            return Ok(r.clone());
        }
        let abs = canon(&f.get_path());
        let k = key(&abs);
        let r = match self.rel(&abs) {
            None => None,
            Some(rel) => match self.owner.get(&k) {
                Some((tu, rel)) if *tu == self.tu => Some(rel.clone()),
                Some(_) => None,
                None => {
                    self.owner.insert(k, (self.tu, rel.clone()));
                    self.new_file(&abs, &rel, f)?;
                    Some(rel)
                }
            },
        };
        self.file_cache.insert(id, r.clone());
        Ok(r)
    }

    fn new_file(&mut self, abs: &Path, rel: &str, f: &clang::source::File) -> Result<()> {
        let bytes = std::fs::read(abs)?;
        let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        let lang = if rel.ends_with(".c") { "c" } else { "cpp" };
        self.emit(Record::File(FileFact { path: rel.to_string(), lang: lang.into(), sha256: sha, bytes: bytes.len() as u64 }))?;
        let mut lines = vec![0];
        lines.extend(bytes.iter().enumerate().filter(|(_, b)| **b == b'\n').map(|(i, _)| i + 1));
        let info = FileInfo { bytes, lines };
        let span = Span { byte_start: 0, byte_end: info.bytes.len() as u64, line_start: 1, line_end: info.lines.len() as u32 };
        self.files.insert(rel.to_string(), info);
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        self.emit(Record::Node(NodeFact { id: rel.into(), parent: None, kind: NodeKind::File, name, span: Some(span), anonymous: false, generated: false }))?;
        let _ = f;
        self.claimed.push(rel.to_string());
        Ok(())
    }

    /// Inactive `#if` regions of this configuration (P9), for the files this TU analyzed. A
    /// header included twice can be inactive in one inclusion and compiled in the other; a
    /// region that contains anything the walk found is not inactive.
    ///
    /// `skipped` is the TU's whole list (`clang_getAllSkippedRanges`): the per-file query
    /// only reports a header's first inclusion.
    fn inactive_regions(&mut self, skipped: &[(PathBuf, usize, usize)]) -> Result<()> {
        let mut by_file: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        for (path, s, e) in skipped {
            if let Some((tu, rel)) = self.owner.get(&key(path))
                && *tu == self.tu {
                    by_file.entry(rel.clone()).or_default().push((*s, *e));
                }
        }
        for rel in std::mem::take(&mut self.claimed) {
            let mut ranges = by_file.remove(&rel).unwrap_or_default();
            ranges.sort_unstable();
            ranges.dedup();
            let found = self.found.remove(&rel).unwrap_or_default();
            for (s, e) in ranges {
                if found.iter().any(|(a, b)| s <= *a && *b <= e) {
                    continue;
                }
                if let Some(sp) = self.span_of(&rel, s, e) {
                    let seg = self.next_name(&rel, "#if-inactive", true);
                    self.emit(Record::Node(NodeFact { id: format!("{rel} › {seg}"), parent: Some(rel.clone()), kind: NodeKind::Inactive, name: seg, span: Some(sp), anonymous: true, generated: false }))?;
                }
            }
        }
        self.found.clear();
        Ok(())
    }

    fn span_of(&self, rel: &str, s: usize, e: usize) -> Option<Span> {
        let f = self.files.get(rel)?;
        let e = e.clamp(s, f.bytes.len());
        let s = s.min(e);
        let (ls, _) = f.line_col(s);
        let (le, _) = f.line_col(e.saturating_sub(1).max(s));
        Some(Span { byte_start: s as u64, byte_end: e as u64, line_start: ls, line_end: le })
    }

    /// File-location byte range of an entity (or a range) in an analyzed file.
    fn range_in(&mut self, r: Option<SourceRange>) -> Result<Option<(String, usize, usize)>> {
        let Some(r) = r else { return Ok(None) };
        let s = r.get_start().get_file_location();
        let e = r.get_end().get_file_location();
        let Some(f) = s.file else { return Ok(None) };
        let Some(rel) = self.claim(&f)? else { return Ok(None) };
        let same = e.file.is_some_and(|ef| ef.get_id() == f.get_id());
        let end = if same && e.offset >= s.offset { e.offset as usize } else { s.offset as usize };
        Ok(Some((rel, s.offset as usize, end)))
    }

    fn receipt(&self, role: &str, rel: &str, s: usize, e: usize, container: &str) -> Receipt {
        let f = &self.files[rel];
        let e = e.clamp(s, f.bytes.len());
        let (line, col) = f.line_col(s);
        let (end_line, end_col) = f.line_col(e);
        Receipt { role: role.into(), file: rel.into(), byte_start: s as u64, byte_end: e as u64, line, col, end_line, end_col, container: Some(container.to_string()) }
    }

    fn next_name(&mut self, scope: &str, name: &str, anon: bool) -> String {
        let k = self.ordinals.entry(scope.to_string()).or_default().entry(name.to_string()).or_default();
        *k += 1;
        if anon || *k > 1 { format!("{name}#{k}") } else { name.to_string() }
    }

    #[allow(clippy::too_many_arguments)]
    fn node(&mut self, ctx: &Ctx, seg: &str, name: &str, kind: NodeKind, span: Span, anonymous: bool, generated: bool) -> Result<String> {
        let id = format!("{} › {seg}", ctx.scope);
        if !self.ids.insert(id.clone()) {
            anyhow::bail!("two declarations produced the node id `{id}`");
        }
        self.found.entry(ctx.file.clone()).or_default().push((span.byte_start as usize, span.byte_end as usize));
        self.emit(Record::Node(NodeFact { id: id.clone(), parent: Some(ctx.parent.clone()), kind, name: name.to_string(), span: Some(span), anonymous, generated }))?;
        Ok(id)
    }

    /// A top-level entity (or any declaration outside functions): analyzed only if its file is.
    fn visit_top(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        if e.get_kind() == EntityKind::InclusionDirective {
            // Claim included project files even when they declare nothing (only macros).
            if let Some(f) = e.get_file() {
                self.claim(&f)?;
            }
            return Ok(());
        }
        if e.is_preprocessing() {
            return Ok(());
        }
        let Some((rel, _, _)) = self.range_in(e.get_range())? else { return Ok(()) };
        let ctx = if ctx.file == rel {
            ctx.clone()
        } else {
            Ctx { file: rel.clone(), scope: rel.clone(), parent: rel.clone(), func: rel.clone(), lambda_name: None }
        };
        self.visit(e, &ctx)
    }

    fn visit(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        use EntityKind as K;
        match e.get_kind() {
            K::Namespace => {
                let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
                let (name, anon) = match e.get_name() {
                    Some(n) if !n.is_empty() => (n, false),
                    _ => ("(anonymous)".to_string(), true),
                };
                let seg = self.next_name(&ctx.scope, &name, anon);
                let span = self.span_of(&rel, s, en).context("span")?;
                let id = self.node(ctx, &seg, &seg, NodeKind::Module, span, anon, false)?;
                let inner = Ctx { scope: id.clone(), parent: id, ..ctx.clone() };
                for c in e.get_children() {
                    self.visit_top(c, &inner)?;
                }
            }
            K::ClassDecl | K::StructDecl | K::UnionDecl | K::EnumDecl | K::ClassTemplate | K::ClassTemplatePartialSpecialization => {
                if !e.is_definition() {
                    return Ok(());
                }
                let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
                // `class X { … } member;` lists the definition twice (also under the member).
                if !self.types_seen.insert((rel.clone(), s, en)) {
                    return Ok(());
                }
                let (name, anon) = match e.get_name().filter(|n| !n.is_empty() && !e.is_anonymous()) {
                    Some(n) => (n, false),
                    None => (kind_word(e.get_kind()).to_string(), true),
                };
                let seg = self.next_name(&ctx.scope, &name, anon);
                let span = self.span_of(&rel, s, en).context("span")?;
                let generated = self.generated(e);
                let id = self.node(ctx, &seg, &seg, NodeKind::Type, span, anon, generated)?;
                let inner = Ctx { scope: id.clone(), parent: id, ..ctx.clone() };
                for c in e.get_children() {
                    self.visit_top(c, &inner)?;
                }
            }
            K::FunctionDecl | K::Method | K::Constructor | K::Destructor | K::ConversionFunction | K::FunctionTemplate | K::LambdaExpr => {
                self.function(e, ctx)?;
            }
            K::CallExpr => {
                self.call(e, ctx)?;
                let inner = Ctx { lambda_name: callee_name(e), ..ctx.clone() };
                self.children(e, &inner)?;
            }
            K::VarDecl | K::FieldDecl => {
                if e.get_kind() == K::FieldDecl {
                    self.state_node(e, ctx, NodeKind::Field)?;
                } else if e.is_definition() {
                    let global = ctx.func == ctx.file || e.get_storage_class() == Some(clang::StorageClass::Static);
                    self.state_node(e, ctx, if global { NodeKind::Static } else { NodeKind::Var })?;
                    self.initializer(e, ctx)?;
                }
                let inner = Ctx { lambda_name: e.get_name(), ..ctx.clone() };
                self.children(e, &inner)?;
            }
            K::BinaryOperator | K::CompoundAssignOperator | K::UnaryOperator => {
                self.assignment(e, ctx)?;
                self.children(e, ctx)?;
            }
            K::ReturnStmt => {
                self.return_stmt(e, ctx)?;
                self.children(e, ctx)?;
            }
            K::DeclRefExpr | K::MemberRefExpr => {
                self.read(e, ctx)?;
                self.children(e, ctx)?;
            }
            // Parameters are state nodes emitted by their function; default arguments are still code.
            K::ParmDecl => self.children(e, ctx)?,
            K::StringLiteral | K::IntegerLiteral => self.literal(e, ctx)?,
            K::IfStmt => self.if_blocks(e, ctx)?,
            K::ForStmt | K::ForRangeStmt | K::WhileStmt => {
                let label = if e.get_kind() == K::WhileStmt { "while" } else { "for" };
                let kids = e.get_children();
                self.with_blocks(e, &kids, &[(label, kids.last().copied())], ctx)?;
            }
            K::DoStmt => {
                let kids = e.get_children();
                self.with_blocks(e, &kids, &[("do", kids.first().copied())], ctx)?;
            }
            K::TryStmt => {
                let kids = e.get_children();
                let mut blocks = vec![("try", kids.first().copied())];
                for k in kids.iter().skip(1).filter(|k| k.get_kind() == K::CatchStmt) {
                    blocks.push(("catch", k.get_children().into_iter().rfind(|c| c.get_kind() == K::CompoundStmt)));
                }
                let rest: Vec<Entity> = kids.iter().skip(1).flat_map(|k| k.get_children()).collect();
                let all: Vec<Entity> = kids.iter().take(1).copied().chain(rest).collect();
                self.with_blocks(e, &all, &blocks, ctx)?;
            }
            K::ConditionalOperator => {
                let kids = e.get_children();
                if kids.len() == 3 {
                    self.with_blocks(e, &kids, &[("?", Some(kids[1])), (":", Some(kids[2]))], ctx)?;
                } else {
                    self.children(e, ctx)?;
                }
            }
            K::SwitchStmt => self.switch_blocks(e, ctx)?,
            _ => self.children(e, ctx)?,
        }
        Ok(())
    }

    fn children(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        for c in e.get_children() {
            if ctx.func == ctx.file || is_decl_scope(e.get_kind()) {
                self.visit_top(c, ctx)?;
            } else {
                self.visit(c, ctx)?;
            }
        }
        Ok(())
    }

    /// True when a function's body brace is written in a macro's template, not in the file.
    fn generated(&self, e: Entity) -> bool {
        let Some(loc) = e.get_location() else { return false };
        let sp = loc.get_spelling_location();
        let fl = loc.get_file_location();
        sp.offset != fl.offset || sp.file.map(|f| f.get_id()) != fl.file.map(|f| f.get_id())
    }

    fn function(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        use EntityKind as K;
        // Overrides are recorded from every method, with or without a body here.
        if matches!(e.get_kind(), K::Method | K::Destructor)
            && let (Some(me), Some(bases)) = (e.get_usr(), e.get_overridden_methods()) {
                for b in bases {
                    if let Some(bu) = b.get_usr() {
                        self.overriders.entry(bu.0).or_default().push(me.0.clone());
                    }
                }
            }
        if e.is_defaulted() {
            return Ok(());
        }
        let kids = e.get_children();
        let Some(body) = kids.iter().rev().find(|c| matches!(c.get_kind(), K::CompoundStmt | K::TryStmt)).copied() else {
            return Ok(());
        };
        let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
        let generated = self.body_generated(body);
        let (seg, name, anon) = if e.get_kind() == K::LambdaExpr {
            let base = ctx.lambda_name.clone().unwrap_or_else(|| "[]".into());
            // Lambdas are named by where they appear, always with an ordinal (BQ-10).
            let seg = self.next_name(&ctx.scope, &base, true);
            (seg.clone(), seg, true)
        } else {
            let name = e.get_name().unwrap_or_else(|| "(unnamed)".into());
            let qual = qualifier(e);
            let full = if qual.is_empty() { name.clone() } else { format!("{qual}::{name}") };
            let seg = self.next_name(&ctx.scope, &full, false);
            let suffix = seg.strip_prefix(&full).unwrap_or("");
            (seg.clone(), format!("{name}{suffix}"), false)
        };
        let span = self.span_of(&rel, s, en).context("span")?;
        let id = self.node(ctx, &seg, &name, NodeKind::Function, span, anon, generated)?;
        if e.get_kind() == K::LambdaExpr {
            self.lambda_by_pos.insert(format!("{rel}@{s}"), id.clone());
        } else if let Some(u) = e.get_usr() {
            self.fn_by_usr.entry(u.0).or_insert_with(|| id.clone());
        }
        let inner = Ctx { file: ctx.file.clone(), scope: id.clone(), parent: id.clone(), func: id.clone(), lambda_name: None };
        let mut params = Vec::new();
        for p in kids.iter().filter(|k| k.get_kind() == K::ParmDecl) {
            self.state_node(*p, &inner, NodeKind::Var)?;
            if let Some(pid) = p.get_usr().and_then(|u| self.state_by_usr.get(&u.0)).cloned() {
                params.push(pid);
            }
        }
        if let Some(u) = e.get_usr() {
            self.fn_params.entry(u.0).or_insert(params);
        }
        for c in kids {
            // Parameters' default arguments and constructor initializers belong to the function too.
            self.visit(c, &inner)?;
        }
        Ok(())
    }

    fn body_generated(&self, body: Entity) -> bool {
        self.generated(body)
    }

    fn call(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        use EntityKind as K;
        let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
        let receipt = self.receipt("call", &rel, s, en, &ctx.parent);
        let callee = e.get_reference();
        let fn_kinds = [K::FunctionDecl, K::Method, K::Constructor, K::Destructor, K::ConversionFunction, K::FunctionTemplate];
        let Some(callee) = callee.filter(|c| fn_kinds.contains(&c.get_kind())) else {
            // Through a variable, field or parameter: a call to that state (the solver finds the
            // functions that can reach it). Through any other expression: unresolved.
            // Implicit constructions without a callee are not calls.
            let via = match callee {
                Some(c) if matches!(c.get_kind(), K::VarDecl | K::ParmDecl | K::FieldDecl) => self.state_ref(c)?,
                _ => None,
            };
            if callee.is_some() || e.get_name().is_some() {
                self.calls.push(PendingCall { from: ctx.func.clone(), usrs: Vec::new(), external: None, dynamic: false, implicit: false, constructor: false, via, receipt });
            }
            return Ok(());
        };
        self.bind_args(e, callee, ctx)?;
        let mut usrs: Vec<String> = callee.get_usr().map(|u| u.0).into_iter().collect();
        let mut t = callee.get_template();
        while let Some(tt) = t {
            usrs.extend(tt.get_usr().map(|u| u.0));
            t = tt.get_template();
        }
        // Builtins (`sqrtf`, `__builtin_*`) have no USR and no source: external leaves.
        let external = if self.decl_inside(callee)? && !usrs.is_empty() { None } else { Some(external_id(callee)) };
        let name = callee.get_name().unwrap_or_default();
        let implicit = name.starts_with("operator") || callee.get_kind() == K::ConversionFunction;
        let constructor = matches!(callee.get_kind(), K::Constructor | K::Destructor);
        if external.is_some() {
            // Function values handed to code outside the analyzed program.
            for a in e.get_arguments().unwrap_or_default() {
                let Some(target) = self.function_value(a)? else { continue };
                let Some((ar, as_, ae)) = self.range_in(a.get_range())? else { continue };
                let r = self.receipt("arg", &ar, as_, ae, &ctx.parent);
                self.handoffs.push(PendingHandoff { from: ctx.func.clone(), target, receipt: r });
            }
        }
        self.calls.push(PendingCall { from: ctx.func.clone(), usrs, external, dynamic: e.is_dynamic_call(), implicit, constructor, via: None, receipt });
        Ok(())
    }

    /// A function or lambda passed as a value: `f`, `&f`, `Class::m`, `[..](..){..}`.
    fn function_value(&mut self, a: Entity) -> Result<Option<String>> {
        use EntityKind as K;
        let mut cur = a;
        loop {
            match cur.get_kind() {
                K::LambdaExpr => {
                    return Ok(self.range_in(cur.get_range())?.map(|(rel, s, _)| format!("lambda:{rel}@{s}")));
                }
                K::DeclRefExpr => {
                    return Ok(cur
                        .get_reference()
                        .filter(|r| matches!(r.get_kind(), K::FunctionDecl | K::Method | K::FunctionTemplate))
                        .and_then(|r| r.get_usr())
                        .map(|u| u.0));
                }
                K::UnexposedExpr | K::ParenExpr | K::UnaryOperator | K::CallExpr if cur.get_children().len() == 1 => cur = cur.get_children()[0],
                _ => return Ok(None),
            }
        }
    }

    /// True if an entity is declared in a file inside the root.
    fn decl_inside(&self, d: Entity) -> Result<bool> {
        let Some(loc) = d.get_location() else { return Ok(false) };
        let Some(f) = loc.get_file_location().file else { return Ok(false) };
        Ok(self.rel(&canon(&f.get_path())).is_some())
    }

    fn literal(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
        let (value, number) = match e.evaluate() {
            Some(EvaluationResult::String(v)) => (v.to_string_lossy().into_owned(), false),
            Some(EvaluationResult::SignedInteger(v)) => (v.to_string(), true),
            Some(EvaluationResult::UnsignedInteger(v)) => (v.to_string(), true),
            _ => return Ok(()),
        };
        let receipt = self.receipt("literal", &rel, s, en, &ctx.parent);
        self.emit(Record::Literal(LiteralFact { value, number, from: ctx.func.clone(), receipt }))
    }

    // ---- blocks (BQ-18)

    fn block(&mut self, label: &str, body: Entity, ctx: &Ctx) -> Result<Option<Ctx>> {
        let Some((rel, s, en)) = self.range_in(body.get_range())? else { return Ok(None) };
        if rel != ctx.file {
            return Ok(None);
        }
        let seg = self.next_name(&format!("{}\u{0}blocks", ctx.scope), label, true);
        let span = self.span_of(&rel, s, en).context("span")?;
        let id = self.node(ctx, &seg, &seg, NodeKind::Block, span, true, false)?;
        Ok(Some(Ctx { parent: id, ..ctx.clone() }))
    }

    /// Visits `kids`; each listed body becomes a block and is visited inside it.
    fn with_blocks(&mut self, _e: Entity, kids: &[Entity], blocks: &[(&str, Option<Entity>)], ctx: &Ctx) -> Result<()> {
        let bodies: Vec<Entity> = blocks.iter().filter_map(|(_, b)| *b).collect();
        for k in kids {
            if !bodies.contains(k) {
                self.visit(*k, ctx)?;
            }
        }
        for (label, body) in blocks {
            let Some(body) = body else { continue };
            match self.block(label, *body, ctx)? {
                Some(inner) => self.visit(*body, &inner)?,
                None => self.visit(*body, ctx)?,
            }
        }
        Ok(())
    }

    fn if_blocks(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        let kids = e.get_children();
        let stmts: Vec<Entity> = kids.iter().copied().filter(|k| k.is_statement()).collect();
        let mut blocks: Vec<(&str, Option<Entity>)> = Vec::new();
        let n = stmts.len();
        // `else` is present when the keyword sits between the last two statements.
        let has_else = n >= 2 && {
            let (a, b) = (stmts[n - 2], stmts[n - 1]);
            match (a.get_range(), b.get_range()) {
                (Some(ra), Some(rb)) => SourceRange::new(ra.get_end(), rb.get_start()).tokenize().iter().any(|t| t.get_spelling() == "else"),
                _ => false,
            }
        };
        if has_else {
            blocks.push(("if", Some(stmts[n - 2])));
            if stmts[n - 1].get_kind() != EntityKind::IfStmt {
                blocks.push(("else", Some(stmts[n - 1])));
            }
        } else if let Some(t) = stmts.last() {
            blocks.push(("if", Some(*t)));
        }
        self.with_blocks(e, &kids, &blocks, ctx)
    }

    /// `case`/`default` blocks span from the label to the statement before the next label.
    fn switch_blocks(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        use EntityKind as K;
        for k in e.get_children() {
            if k.get_kind() != K::CompoundStmt {
                self.visit(k, ctx)?;
                continue;
            }
            let stmts = k.get_children();
            let mut i = 0;
            while i < stmts.len() {
                let s = stmts[i];
                if !matches!(s.get_kind(), K::CaseStmt | K::DefaultStmt) {
                    self.visit(s, ctx)?;
                    i += 1;
                    continue;
                }
                let mut j = i + 1;
                while j < stmts.len() && !matches!(stmts[j].get_kind(), K::CaseStmt | K::DefaultStmt) {
                    j += 1;
                }
                let label = if s.get_kind() == K::CaseStmt { "case" } else { "default" };
                let start = s.get_range();
                let end = stmts[j - 1].get_range();
                let range = match (start, end) {
                    (Some(a), Some(b)) => Some(SourceRange::new(a.get_start(), b.get_end())),
                    _ => start,
                };
                let inner = match self.range_in(range)? {
                    Some((rel, a, b)) if rel == ctx.file => {
                        let seg = self.next_name(&format!("{}\u{0}blocks", ctx.scope), label, true);
                        let span = self.span_of(&rel, a, b).context("span")?;
                        let id = self.node(ctx, &seg, &seg, NodeKind::Block, span, true, false)?;
                        Ctx { parent: id, ..ctx.clone() }
                    }
                    _ => ctx.clone(),
                };
                for st in &stmts[i..j] {
                    self.visit(*st, &inner)?;
                }
                i = j;
            }
        }
        Ok(())
    }

    // ---- end of stream: resolve calls through USRs, then emit edges and external leaves

    fn finish(mut self) -> Result<()> {
        // Every override, transitively, of each virtual method.
        let mut closure: HashMap<String, Vec<String>> = HashMap::new();
        for base in self.overriders.keys() {
            let mut seen = HashSet::new();
            let mut stack = vec![base.clone()];
            while let Some(u) = stack.pop() {
                for o in self.overriders.get(&u).into_iter().flatten() {
                    if seen.insert(o.clone()) {
                        stack.push(o.clone());
                    }
                }
            }
            let mut v: Vec<String> = seen.into_iter().collect();
            v.sort();
            closure.insert(base.clone(), v);
        }
        let calls = std::mem::take(&mut self.calls);
        for c in calls {
            let target = c.usrs.iter().find_map(|u| self.fn_by_usr.get(u)).cloned();
            let subkind = c.implicit.then(|| "implicit".to_string());
            if let Some(via) = &c.via {
                let to = self.resolve(via);
                let res = if to.is_some() { Resolution::Exact } else { Resolution::Unresolved };
                self.edge(EdgeKind::Call, None, &c.from, to, res, None, c.receipt)?;
                continue;
            }
            if c.usrs.is_empty() && c.external.is_none() {
                self.edge(EdgeKind::Call, None, &c.from, None, Resolution::Unresolved, None, c.receipt)?;
                continue;
            }
            if let Some((id, name)) = &c.external {
                self.external(id, name);
                self.edge(EdgeKind::Call, subkind, &c.from, Some(id.clone()), Resolution::Exact, None, c.receipt)?;
                continue;
            }
            let overrides: Vec<String> = if c.dynamic {
                c.usrs.first().and_then(|u| closure.get(u)).into_iter().flatten().filter_map(|u| self.fn_by_usr.get(u)).cloned().collect()
            } else {
                Vec::new()
            };
            if c.dynamic && !overrides.is_empty() {
                // Virtual dispatch: the named method (if it has a body) and every override.
                self.group += 1;
                let g = self.group;
                let mut cands: Vec<String> = target.into_iter().chain(overrides).collect();
                cands.sort();
                cands.dedup();
                for t in cands {
                    self.edge(EdgeKind::Call, subkind.clone(), &c.from, Some(t), Resolution::Possible, Some(g), c.receipt.clone())?;
                }
                continue;
            }
            match target {
                Some(t) => self.edge(EdgeKind::Call, subkind, &c.from, Some(t), Resolution::Exact, None, c.receipt)?,
                // Compiler-generated constructors and operators have no code to point at.
                None if c.constructor || c.implicit => {}
                None => self.edge(EdgeKind::Call, subkind, &c.from, None, Resolution::Unresolved, None, c.receipt)?,
            }
        }
        let handoffs = std::mem::take(&mut self.handoffs);
        for h in handoffs {
            let to = match h.target.strip_prefix("lambda:") {
                Some(pos) => self.lambda_by_pos.get(pos).cloned(),
                None => self.fn_by_usr.get(&h.target).cloned(),
            };
            if let Some(to) = to {
                let mut e = EdgeFact { kind: EdgeKind::ValueFlow, subkind: Some("handoff".into()), from: h.from, to: Some(to), resolution: Resolution::Exact, group: None, receipts: vec![h.receipt] };
                e.receipts.dedup();
                self.emit(Record::Edge(e))?;
            }
        }
        let data = std::mem::take(&mut self.data_edges);
        for d in data {
            let Some(from) = self.resolve(&d.from) else { continue };
            // An argument of a virtual method binds to the same parameter of every override:
            // `possible` targets, as with the call's candidates.
            if let flow::Ref::Param { usrs, index, ext: None } = &d.to {
                let overrides: Vec<String> = usrs.first().and_then(|u| closure.get(u)).into_iter().flatten().filter_map(|o| self.fn_params.get(o).and_then(|p| p.get(*index))).cloned().collect();
                if !overrides.is_empty() {
                    self.group += 1;
                    let g = self.group;
                    let mut targets: Vec<String> = self.resolve(&d.to).into_iter().chain(overrides).collect();
                    targets.sort();
                    targets.dedup();
                    for t in targets {
                        self.edge(d.kind, d.subkind.clone(), &from, Some(t), Resolution::Possible, Some(g), d.receipt.clone())?;
                    }
                    continue;
                }
            }
            let Some(to) = self.resolve(&d.to) else { continue };
            self.edge(d.kind, d.subkind, &from, Some(to), Resolution::Exact, None, d.receipt)?;
        }
        // External leaves after the edges that name them is fine: the solver inserts edges last.
        for (id, name) in std::mem::take(&mut self.externals) {
            self.emit(Record::Node(NodeFact { id, parent: None, kind: NodeKind::External, name, span: None, anonymous: false, generated: false }))?;
        }
        let mut out = self.out.take().expect("output");
        let end = out.end.clone();
        out.emit(Record::End(end))?;
        out.w.flush()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn edge(&mut self, kind: EdgeKind, subkind: Option<String>, from: &str, to: Option<String>, resolution: Resolution, group: Option<u64>, receipt: Receipt) -> Result<()> {
        self.emit(Record::Edge(EdgeFact { kind, subkind, from: from.to_string(), to, resolution, group, receipts: vec![receipt] }))
    }
}

fn kind_word(k: EntityKind) -> &'static str {
    match k {
        EntityKind::UnionDecl => "union",
        EntityKind::EnumDecl => "enum",
        EntityKind::ClassDecl | EntityKind::ClassTemplate => "class",
        _ => "struct",
    }
}

fn is_named_scope(k: EntityKind) -> bool {
    is_decl_scope(k) || k == EntityKind::EnumDecl
}

fn is_decl_scope(k: EntityKind) -> bool {
    matches!(k, EntityKind::Namespace | EntityKind::ClassDecl | EntityKind::StructDecl | EntityKind::UnionDecl | EntityKind::ClassTemplate | EntityKind::ClassTemplatePartialSpecialization)
}

/// For an out-of-line definition (`void A::B::f() {}`), the qualifier `A::B`: the semantic
/// parents that are not also lexical ones.
fn qualifier(e: Entity) -> String {
    let lexical = e.get_lexical_parent();
    let mut parts = Vec::new();
    let mut p = e.get_semantic_parent();
    while let Some(sp) = p {
        if Some(sp) == lexical || sp.get_kind() == EntityKind::TranslationUnit {
            break;
        }
        if let Some(n) = sp.get_name().filter(|n| !n.is_empty() && is_named_scope(sp.get_kind())) {
            parts.push(n);
        }
        p = sp.get_semantic_parent();
    }
    parts.reverse();
    parts.join("::")
}

/// The callee's written name, which a lambda argument is named after (`for_each#1`).
fn callee_name(e: Entity) -> Option<String> {
    e.get_reference().and_then(|r| r.get_name()).filter(|n| !n.is_empty() && !n.starts_with("operator"))
}

/// `ext:cpp:<declaring header>:<qualified name>` (BQ-20).
fn external_id(d: Entity) -> (String, String) {
    let header = d.get_location().and_then(|l| l.get_file_location().file).map(|f| f.get_path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).unwrap_or_default();
    let mut parts = vec![d.get_name().unwrap_or_default()];
    let mut p = d.get_semantic_parent();
    while let Some(sp) = p {
        // Only named scopes; `extern "C"` blocks and the translation unit are not part of a name.
        if is_named_scope(sp.get_kind())
            && let Some(n) = sp.get_name().filter(|n| !n.is_empty()) {
                parts.push(n);
            }
        p = sp.get_semantic_parent();
    }
    parts.reverse();
    let q = parts.join("::");
    (format!("ext:cpp:{header}:{q}"), parts.last().cloned().unwrap_or_default())
}
