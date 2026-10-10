//! Insight's Rust extractor (ADR-0034, R2-27).
//!
//! Loads each Cargo workspace into rust-analyzer (build scripts and proc macros
//! on, so macros expand: BQ-06) and emits containers: files, modules, types,
//! impls, functions (including closures) and blocks. Functions written inside
//! macro-call arguments (`map(|x| …)` inside `println!(…)`) are found through
//! the real macro expansion and mapped back to their source text (R3-11).
//! Knows the Rust language only, no frameworks (ADR-0048). NDJSON on stdout (R2-28).
//!
//! rust-analyzer crates are pinned to 0.0.345 (newer releases need rustc > 1.95);
//! `unicode-ident` is pinned to 1.0.22 to match its lexer's Unicode tables.
//!
//! Usage: insight-extract-rust --root <dir> --package <dir> [--package …] --schema <N>

mod flow;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_model::{
    Diagnostic, EdgeFact, EdgeKind, End, FACT_SCHEMA_VERSION, FileFact, Header, LiteralFact, NodeFact, NodeKind, Receipt, Record, Resolution, Severity, Span,
};
use ra_ap_hir::{AsAssocItem, HasCrate, Semantics};
use ra_ap_ide_db::RootDatabase;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_syntax::{AstNode, AstToken, Edition, SourceFile, SyntaxKind, SyntaxNode, T, ast, ast::HasArgList, ast::HasLoopBody, ast::HasName};
use ra_ap_vfs::FileId;
use sha2::{Digest, Sha256};

const EXTRACTOR: &str = concat!("rust@", env!("CARGO_PKG_VERSION"));

/// A function written inside a macro call's arguments, mapped back to source.
struct MacroFn {
    start: u32,
    end: u32,
    name: String,
    anon: bool,
}

/// The topmost folder (within `root`) whose Cargo.toml declares `[workspace]`, else the package itself.
fn workspace_root(root: &Path, package: &Path) -> PathBuf {
    let mut best = package.to_path_buf();
    for dir in package.ancestors() {
        if !dir.starts_with(root) {
            break;
        }
        let is_ws = std::fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|t| t.lines().any(|l| l.trim() == "[workspace]"));
        if is_ws {
            best = dir.to_path_buf();
        }
    }
    best
}

fn norm(p: &str) -> String {
    p.trim_start_matches(r"\\?\").replace('/', "\\").to_lowercase()
}

fn main() -> Result<()> {
    // rust-analyzer recurses deeply (type inference, macro expansion); like
    // rust-analyzer itself, run on a thread with a large stack.
    std::thread::Builder::new().name("extract".into()).stack_size(256 * 1024 * 1024).spawn(run)?.join().map_err(|_| anyhow::anyhow!("extractor thread panicked"))?
}

fn run() -> Result<()> {
    let mut root = None;
    let mut packages = Vec::new();
    let mut schema = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--root" => root = args.next().map(PathBuf::from),
            "--package" => packages.extend(args.next()),
            "--schema" => schema = args.next().map(|s| s.parse::<u32>()).transpose()?,
            other => bail!("unknown argument {other}"),
        }
    }
    let root = root.context("need --root")?;
    if schema != Some(FACT_SCHEMA_VERSION) {
        bail!("schema mismatch: asked {schema:?}, extractor speaks {FACT_SCHEMA_VERSION}");
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for p in &packages {
        collect_rs(&root.join(p), &mut files)?;
    }
    files.sort();
    files.dedup();

    let stdout = std::io::stdout();
    let mut out = Out { w: BufWriter::new(stdout.lock()), end: End::default() };
    out.emit(Record::Header(Header {
        schema: FACT_SCHEMA_VERSION,
        extractor: EXTRACTOR.into(),
        toolchain: "rust-analyzer 0.0.345 (ra_ap crates)".into(),
    }))?;

    // Load every workspace once; map each file to the database that owns it.
    let roots: BTreeSet<PathBuf> = packages.iter().map(|p| workspace_root(&root, &root.join(p))).collect();
    let cargo_config = CargoConfig { sysroot: Some(RustLibSource::Discover), all_targets: true, set_test: true, ..Default::default() };
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let load_config = LoadCargoConfig {
        load_out_dirs_from_check: true,
        with_proc_macro_server: ProcMacroServerChoice::Sysroot,
        prefill_caches: false,
        num_worker_threads: threads,
        proc_macro_processes: 1,
    };
    let mut dbs: Vec<RootDatabase> = Vec::new();
    let mut owner: HashMap<String, (usize, FileId)> = HashMap::new();
    for ws in &roots {
        eprintln!("loading workspace {}", ws.display());
        let (db, vfs, _pm) = load_workspace_at(ws, &cargo_config, &load_config, &|msg| eprintln!("  {msg}"))?;
        for (id, path) in vfs.iter() {
            if let Some(abs) = path.as_path() {
                owner.entry(norm(abs.as_str())).or_insert((dbs.len(), id));
            }
        }
        dbs.push(db);
    }
    let semas: Vec<Semantics<'_, RootDatabase>> = dbs.iter().map(Semantics::new).collect();

    // Pass 1: containers.
    let mut indexed = Vec::new();
    for abs in files {
        let ix = owner.get(&norm(&abs.to_string_lossy())).copied();
        let sema = ix.map(|(i, id)| (&semas[i], id));
        indexed.push(extract_file(&root, &abs, sema, ix, &mut out)?);
    }

    // Pass 2: edges and literals, per database.
    let mut fns = HashMap::new();
    let mut paths = HashMap::new();
    for f in &indexed {
        for c in f.containers.iter().filter(|c| c.is_fn) {
            fns.insert((f.file.clone(), c.start, c.end), c.id.clone());
        }
        if let Some(key) = f.sema {
            paths.insert(key, f.file.clone());
        }
    }
    let mut externals = BTreeSet::new();
    let mut states = HashMap::new();
    for (ix, db) in dbs.iter().enumerate() {
        let mut g = Global { fns, paths, externals, states, db, ix };
        // Type inference (method resolution) needs the database attached to this thread.
        ra_ap_hir::attach_db(db, || -> Result<()> {
            let mine: Vec<&Indexed> = indexed.iter().filter(|f| f.sema.is_some_and(|(i, _)| i == ix)).collect();
            // State nodes first: edges in one file name state declared in another.
            for f in &mine {
                flow::state_nodes(&semas[ix], f, &mut g, &mut out)?;
            }
            for f in &mine {
                edges_for_file(&semas[ix], f, f.sema.expect("filtered").1, &mut g, &mut out)?;
                flow::data_for_file(&semas[ix], f, f.sema.expect("filtered").1, &mut g, &mut out)?;
            }
            Ok(())
        })?;
        (fns, paths, externals, states) = (g.fns, g.paths, g.externals, g.states);
    }
    for id in externals {
        let name = id.rsplit(':').next().unwrap_or(&id).to_string();
        out.emit(Record::Node(NodeFact { id, parent: None, kind: NodeKind::External, name, span: None, anonymous: false, generated: false }))?;
    }
    let end = out.end.clone();
    out.emit(Record::End(end))?;
    out.w.flush()?;
    Ok(())
}

struct Out<W: Write> {
    w: W,
    end: End,
}

impl<W: Write> Out<W> {
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

/// Every `.rs` file under a package, skipping build output and hidden folders.
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if name == "target" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            collect_rs(&path, out)?;
        } else if ft.is_file() && name.ends_with(".rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn rel(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root).unwrap_or(abs).to_string_lossy().replace('\\', "/")
}

struct Lines(Vec<u32>);
impl Lines {
    fn new(text: &str) -> Self {
        let mut starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i as u32 + 1);
            }
        }
        Lines(starts)
    }
    fn line(&self, byte: u32) -> u32 {
        match self.0.binary_search(&byte) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }
}

struct FileCx<'a, W: Write> {
    out: &'a mut Out<W>,
    lines: Lines,
    /// (scope id, name) -> count, for positional ordinals (R3-G #1).
    ordinals: HashMap<(String, String), u32>,
    /// Functions inside macro-call arguments, sorted by (start, longest first).
    macro_fns: Vec<MacroFn>,
    /// Every container emitted for this file, for pass 2's range lookups.
    containers: Vec<Ctr>,
}

/// A container's source range and id (pass 1 output, pass 2 input).
#[derive(Clone)]
struct Ctr {
    start: u32,
    end: u32,
    id: String,
    is_fn: bool,
    /// A path segment (not a block): state is named under the nearest one.
    scope: bool,
}

/// What pass 2 needs for one file.
struct Indexed {
    file: String,
    sema: Option<(usize, FileId)>,
    tree: SyntaxNode,
    lines: Lines,
    containers: Vec<Ctr>,
}

impl Indexed {
    /// The innermost container (`fns_only`: function, else the file) holding a range.
    fn innermost(&self, s: u32, e: u32, fns_only: bool) -> String {
        self.containers
            .iter()
            .filter(|c| c.start <= s && e <= c.end && (c.is_fn || !fns_only))
            .min_by_key(|c| c.end - c.start)
            .map(|c| c.id.clone())
            .unwrap_or_else(|| self.file.clone())
    }

    fn receipt(&self, role: &str, s: u32, e: u32) -> Receipt {
        let (line, end_line) = (self.lines.line(s), self.lines.line(e));
        let col = |b: u32, l: u32| b - self.lines.0[(l - 1) as usize] + 1;
        Receipt {
            role: role.into(),
            file: self.file.clone(),
            byte_start: s as u64,
            byte_end: e as u64,
            line,
            col: col(s, line),
            end_line,
            end_col: col(e, end_line),
            container: Some(self.innermost(s, e, false)),
        }
    }
}

impl<W: Write> FileCx<'_, W> {
    fn name(&mut self, scope: &str, name: &str, anon: bool) -> String {
        let k = self.ordinals.entry((scope.to_string(), name.to_string())).or_insert(0);
        *k += 1;
        if anon || *k > 1 { format!("{name}#{k}") } else { name.to_string() }
    }

    fn span(&self, n: &SyntaxNode) -> Span {
        let r = n.text_range();
        self.span_of(u32::from(r.start()), u32::from(r.end()))
    }

    fn span_of(&self, s: u32, e: u32) -> Span {
        Span { byte_start: s as u64, byte_end: e as u64, line_start: self.lines.line(s), line_end: self.lines.line(e.saturating_sub(1).max(s)) }
    }

    fn node(&mut self, id: String, parent: &str, kind: NodeKind, name: String, span: Span, anonymous: bool) -> Result<String> {
        self.containers.push(Ctr { start: span.byte_start as u32, end: span.byte_end as u32, id: id.clone(), is_fn: kind == NodeKind::Function, scope: kind != NodeKind::Block });
        self.out.emit(Record::Node(NodeFact { id: id.clone(), parent: Some(parent.to_string()), kind, name, span: Some(span), anonymous, generated: false }))?;
        Ok(id)
    }
}

/// Pass 1 for one file: the file record and its containers.
fn extract_file<W: Write>(
    root: &Path,
    abs: &Path,
    sema: Option<(&Semantics<'_, RootDatabase>, FileId)>,
    sema_ix: Option<(usize, FileId)>,
    out: &mut Out<W>,
) -> Result<Indexed> {
    let bytes = std::fs::read(abs)?;
    let text = String::from_utf8(bytes.clone()).with_context(|| format!("{} is not UTF-8", abs.display()))?;
    let file = rel(root, abs);
    out.emit(Record::File(FileFact {
        path: file.clone(),
        lang: "rust".into(),
        sha256: Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect(),
        bytes: bytes.len() as u64,
    }))?;
    let lines = Lines::new(&text);
    let last_line = lines.0.len() as u32;
    out.emit(Record::Node(NodeFact {
        id: file.clone(),
        parent: None,
        kind: NodeKind::File,
        name: file.rsplit('/').next().unwrap_or(&file).to_string(),
        span: Some(Span { byte_start: 0, byte_end: bytes.len() as u64, line_start: 1, line_end: last_line }),
        anonymous: false,
        generated: false,
    }))?;
    let (tree, macro_fns) = match sema {
        Some((sema, id)) => {
            let tree = sema.parse_guess_edition(id).syntax().clone();
            if tree.text() != text.as_str() {
                bail!("{file}: rust-analyzer's text differs from the file on disk");
            }
            let fns = macro_functions(sema, &tree, id);
            (tree, fns)
        }
        None => {
            out.emit(Record::Diagnostic(Diagnostic {
                file: Some(file.clone()),
                severity: Severity::Warning,
                message: "not part of any crate's module tree; macro arguments not analyzed".into(),
            }))?;
            (SourceFile::parse(&text, Edition::Edition2024).tree().syntax().clone(), Vec::new())
        }
    };
    let mut cx = FileCx { out, lines, ordinals: HashMap::new(), macro_fns, containers: Vec::new() };
    cx.containers.push(Ctr { start: 0, end: bytes.len() as u32, id: file.clone(), is_fn: false, scope: true });
    for child in tree.children() {
        visit(&mut cx, &child, &file, &file)?;
    }
    Ok(Indexed { file, sema: sema_ix, tree, lines: cx.lines, containers: cx.containers })
}

// ================================ pass 2: edges and literals ================================
// Generic Rust semantics only (ADR-0048): calls resolved by rust-analyzer (through
// macro expansions, including attribute macros), handoffs of closures and function
// items to external code, and string literals (BQ-22).

/// Lookups shared by every file in pass 2.
struct Global<'a> {
    /// (file, start, end) -> function node id.
    fns: HashMap<(String, u32, u32), String>,
    /// (database index, FileId) -> repo-relative path.
    paths: HashMap<(usize, FileId), String>,
    externals: BTreeSet<String>,
    /// (file, start, end) of a declaration -> state node id.
    states: HashMap<(String, u32, u32), String>,
    db: &'a RootDatabase,
    ix: usize,
}

fn edges_for_file<W: Write>(sema: &Semantics<'_, RootDatabase>, f: &Indexed, id: FileId, g: &mut Global<'_>, out: &mut Out<W>) -> Result<()> {
    let this_file = sema.attach_first_edition(id);
    let mut seen_calls: BTreeSet<(u32, u32)> = BTreeSet::new();
    for tok in f.tree.descendants_with_tokens().filter_map(|e| e.into_token()) {
        let r = tok.text_range();
        let (ts, te) = (u32::from(r.start()), u32::from(r.end()));
        if tok.kind() == SyntaxKind::STRING {
            if let Some(s) = ast::String::cast(tok.clone())
                && let Ok(v) = s.value() {
                    let from = f.innermost(ts, te, true);
                    out.emit(Record::Literal(LiteralFact { value: v.to_string(), number: false, from, receipt: f.receipt("literal", ts, te) }))?;
                }
            continue;
        }
        if tok.kind() == SyntaxKind::INT_NUMBER {
            // Integers in canonical decimal (`0x1F` and `31u8` are both 31).
            if let Some(v) = ast::IntNumber::cast(tok.clone()).and_then(|n| n.value().ok()) {
                let from = f.innermost(ts, te, true);
                out.emit(Record::Literal(LiteralFact { value: v.to_string(), number: true, from, receipt: f.receipt("literal", ts, te) }))?;
            }
            continue;
        }
        if tok.kind() != SyntaxKind::IDENT {
            continue;
        }
        // The token as the compiler sees it: itself, or its copies inside macro expansions.
        for d in sema.descend_into_macros(tok.clone()) {
            let Some(call) = call_named_by(&d) else { continue };
            let Some(orig) = sema.original_range_opt(call.syntax()) else { continue };
            if orig.file_id != this_file {
                continue;
            }
            let (cs, ce) = (u32::from(orig.range.start()), u32::from(orig.range.end()));
            if !seen_calls.insert((cs, ce)) {
                continue;
            }
            let from = f.innermost(cs, ce, true);
            let receipt = f.receipt("call", cs, ce);
            match resolve_call(sema, &call) {
                Callee::Fn(func) => {
                    let (to, external) = target_of(g, func);
                    out.emit(Record::Edge(EdgeFact {
                        kind: EdgeKind::Call,
                        subkind: None,
                        from: from.clone(),
                        to: to.clone(),
                        resolution: if to.is_some() { Resolution::Exact } else { Resolution::Unresolved },
                        group: None,
                        receipts: vec![receipt],
                    }))?;
                    if external {
                        for (arg_fn, (as_, ae)) in fn_args(sema, f, g, &call, this_file) {
                            out.emit(Record::Edge(EdgeFact {
                                kind: EdgeKind::ValueFlow,
                                subkind: Some("handoff".into()),
                                from: from.clone(),
                                to: Some(arg_fn),
                                resolution: Resolution::Exact,
                                group: None,
                                receipts: vec![f.receipt("arg", as_, ae)],
                            }))?;
                        }
                    }
                }
                Callee::Indirect | Callee::Unknown => {
                    // Through a local, field or static: a call to that state (the solver finds
                    // the functions that reach it). Otherwise unresolved.
                    let to = flow::called_state(sema, g, &call);
                    let resolution = if to.is_some() { Resolution::Exact } else { Resolution::Unresolved };
                    out.emit(Record::Edge(EdgeFact { kind: EdgeKind::Call, subkind: None, from, to, resolution, group: None, receipts: vec![receipt] }))?
                }
                Callee::NotACall => {}
            }
            break;
        }
    }
    Ok(())
}

enum CallNode {
    Call(ast::CallExpr),
    Method(ast::MethodCallExpr),
}

impl CallNode {
    fn syntax(&self) -> &SyntaxNode {
        match self {
            CallNode::Call(c) => c.syntax(),
            CallNode::Method(m) => m.syntax(),
        }
    }
    fn args(&self) -> Vec<ast::Expr> {
        let list = match self {
            CallNode::Call(c) => c.arg_list(),
            CallNode::Method(m) => m.arg_list(),
        };
        list.map(|l| l.args().collect()).unwrap_or_default()
    }
}

/// The call whose callee name is this token: `f(…)`, `a::b::f(…)`, `x.f(…)`.
fn call_named_by(tok: &ra_ap_syntax::SyntaxToken) -> Option<CallNode> {
    let name_ref = ast::NameRef::cast(tok.parent()?)?;
    let parent = name_ref.syntax().parent()?;
    if let Some(mc) = ast::MethodCallExpr::cast(parent.clone()) {
        return (mc.name_ref()? == name_ref).then_some(CallNode::Method(mc));
    }
    let segment = ast::PathSegment::cast(parent)?;
    let path = segment.parent_path();
    if path.segment()? != segment {
        return None;
    }
    let path_expr = ast::PathExpr::cast(path.syntax().parent()?)?;
    let call = ast::CallExpr::cast(path_expr.syntax().parent()?)?;
    (call.expr()?.syntax() == path_expr.syntax()).then_some(CallNode::Call(call))
}

enum Callee {
    Fn(ra_ap_hir::Function),
    /// Through a local, field or other function value: one-hop value flow comes later.
    Indirect,
    Unknown,
    /// A tuple-struct or variant constructor, not a function call.
    NotACall,
}

fn resolve_call(sema: &Semantics<'_, RootDatabase>, call: &CallNode) -> Callee {
    use ra_ap_hir::{ModuleDef, PathResolution};
    match call {
        CallNode::Method(m) => sema.resolve_method_call(m).map(Callee::Fn).unwrap_or(Callee::Unknown),
        CallNode::Call(c) => {
            let Some(ast::Expr::PathExpr(pe)) = c.expr() else { return Callee::Indirect };
            let Some(path) = pe.path() else { return Callee::Unknown };
            match sema.resolve_path(&path) {
                Some(PathResolution::Def(ModuleDef::Function(f))) => Callee::Fn(f),
                Some(PathResolution::Def(ModuleDef::EnumVariant(_) | ModuleDef::Adt(_))) => Callee::NotACall,
                Some(PathResolution::Local(_)) => Callee::Indirect,
                Some(_) => Callee::Indirect,
                None => Callee::Unknown,
            }
        }
    }
}

/// The node a resolved function maps to: a project function node, or an external leaf.
/// Returns (id, is_external). `None` = a project declaration with no body node (e.g. a trait method).
fn target_of(g: &mut Global<'_>, func: ra_ap_hir::Function) -> (Option<String>, bool) {
    use ra_ap_hir::{AssocItemContainer, HasSource};
    let db = g.db;
    if func.krate(db).origin(db).is_local() {
        let Some(src) = func.source(db) else { return (None, false) };
        let range = src.syntax().original_file_range_rooted(db);
        let fid = range.file_id.file_id(db);
        let Some(path) = g.paths.get(&(g.ix, fid)) else { return (None, false) };
        let key = (path.clone(), u32::from(range.range.start()), u32::from(range.range.end()));
        return (g.fns.get(&key).cloned(), false);
    }
    let krate = func.krate(db).display_name(db).map(|n| n.to_string()).unwrap_or_else(|| "?".into());
    let mut segs: Vec<String> = func.module(db).path_to_root(db).iter().rev().filter_map(|m| m.name(db)).map(|n| n.as_str().to_string()).collect();
    if let Some(assoc) = func.as_assoc_item(db) {
        match assoc.container(db) {
            AssocItemContainer::Trait(t) => segs.push(t.name(db).as_str().to_string()),
            AssocItemContainer::Impl(i) => {
                if let Some(adt) = i.self_ty(db).as_adt() {
                    segs.push(adt.name(db).as_str().to_string());
                }
            }
        }
    }
    segs.push(func.name(db).as_str().to_string());
    let name = segs.join("::");
    let id = format!("ext:rust:{krate}:{name}");
    (Some(id.clone()), {
        g.externals.insert(id);
        true
    })
}

/// Closures and function items passed as arguments, mapped to their function nodes.
fn fn_args(sema: &Semantics<'_, RootDatabase>, f: &Indexed, g: &mut Global<'_>, call: &CallNode, this_file: ra_ap_hir::EditionedFileId) -> Vec<(String, (u32, u32))> {
    use ra_ap_hir::{ModuleDef, PathResolution};
    let mut out = Vec::new();
    for arg in call.args() {
        let Some(orig) = sema.original_range_opt(arg.syntax()) else { continue };
        if orig.file_id != this_file {
            continue;
        }
        let (s, e) = (u32::from(orig.range.start()), u32::from(orig.range.end()));
        let id = match &arg {
            ast::Expr::ClosureExpr(_) => g.fns.get(&(f.file.clone(), s, e)).cloned(),
            ast::Expr::PathExpr(pe) => match pe.path().and_then(|p| sema.resolve_path(&p)) {
                Some(PathResolution::Def(ModuleDef::Function(func))) => match target_of(g, func) {
                    (Some(id), false) => Some(id),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        if let Some(id) = id {
            out.push((id, (s, e)));
        }
    }
    out
}

/// Finds every function written inside a macro call's arguments by descending
/// its `|`, `||` and `fn` tokens into the real expansion (all nesting levels),
/// and keeping those whose expansion maps back to this exact source text.
fn macro_functions(sema: &Semantics<'_, RootDatabase>, tree: &SyntaxNode, id: FileId) -> Vec<MacroFn> {
    let this_file = sema.attach_first_edition(id);
    let mut found: BTreeMap<(u32, std::cmp::Reverse<u32>), (String, bool)> = BTreeMap::new();
    for tt in tree.descendants().filter(|n| n.kind() == SyntaxKind::TOKEN_TREE && n.parent().is_some_and(|p| p.kind() == SyntaxKind::MACRO_CALL)) {
        let range = tt.text_range();
        let mac = tt.parent().and_then(ast::MacroCall::cast);
        let mac_name = mac.and_then(|m| m.path()).and_then(|p| p.segment()).map(|s| format!("{}!", s.syntax().text())).unwrap_or_else(|| "macro!".into());
        for tok in tt.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if !matches!(tok.kind(), T![|] | T![||] | T![fn]) {
                continue;
            }
            for d in sema.descend_into_macros(tok.clone()) {
                let Some(p) = d.parent() else { continue };
                let func: Option<(SyntaxNode, String, bool)> = if d.kind() == T![fn] {
                    ast::Fn::cast(p).filter(|f| f.body().is_some()).map(|f| {
                        let name = f.name().map(|n| n.text().to_string()).unwrap_or_else(|| "fn".into());
                        (f.syntax().clone(), name, false)
                    })
                } else {
                    p.ancestors()
                        .take(2)
                        .find_map(ast::ClosureExpr::cast)
                        .filter(|c| c.param_list().is_some_and(|pl| pl.syntax().text_range().contains_range(d.text_range())))
                        .map(|c| (c.syntax().clone(), mac_name.clone(), true))
                };
                let Some((node, name, anon)) = func else { continue };
                let Some(orig) = sema.original_range_opt(&node) else { continue };
                if orig.file_id == this_file && range.contains_range(orig.range) {
                    found.insert((u32::from(orig.range.start()), std::cmp::Reverse(u32::from(orig.range.end()))), (name, anon));
                }
            }
        }
    }
    found.into_iter().map(|((start, std::cmp::Reverse(end)), (name, anon))| MacroFn { start, end, name, anon }).collect()
}

/// Emits the macro-argument functions inside one macro call, nested by containment.
fn emit_macro_fns<W: Write>(cx: &mut FileCx<W>, tt: &SyntaxNode, scope: &str, parent: &str) -> Result<()> {
    let r = tt.text_range();
    let (s, e) = (u32::from(r.start()), u32::from(r.end()));
    let inside: Vec<(u32, u32, String, bool)> =
        cx.macro_fns.iter().filter(|f| f.start >= s && f.end <= e).map(|f| (f.start, f.end, f.name.clone(), f.anon)).collect();
    // Stack of (end, id): the innermost open function containing the next one.
    let mut stack: Vec<(u32, String)> = Vec::new();
    for (fs, fe, name, anon) in inside {
        while stack.last().is_some_and(|(end, _)| fs >= *end) {
            stack.pop();
        }
        let (sc, pa) = match stack.last() {
            Some((_, id)) => (id.clone(), id.clone()),
            None => (scope.to_string(), parent.to_string()),
        };
        let seg = cx.name(&sc, &name, anon);
        let id = format!("{sc} › {seg}");
        let span = cx.span_of(fs, fe);
        cx.node(id.clone(), &pa, NodeKind::Function, seg, span, anon)?;
        stack.push((fe, id));
    }
    Ok(())
}

/// `scope` = nearest path segment (file/module/type/impl/function); `parent` = nearest container.
fn visit<W: Write>(cx: &mut FileCx<W>, n: &SyntaxNode, scope: &str, parent: &str) -> Result<()> {
    if n.kind() == SyntaxKind::TOKEN_TREE && n.parent().is_some_and(|p| p.kind() == SyntaxKind::MACRO_CALL) {
        return emit_macro_fns(cx, n, scope, parent);
    }
    let mut scope = scope.to_string();
    let mut parent = parent.to_string();

    // Containers that are path segments.
    let seg: Option<(NodeKind, String, bool)> = if let Some(f) = ast::Fn::cast(n.clone()) {
        f.body().map(|_| (NodeKind::Function, f.name().map(|x| x.text().to_string()).unwrap_or_else(|| "fn".into()), false))
    } else if let Some(c) = ast::ClosureExpr::cast(n.clone()) {
        let (name, anon) = closure_name(&c);
        Some((NodeKind::Function, name, anon))
    } else if let Some(m) = ast::Module::cast(n.clone()) {
        m.item_list().map(|_| (NodeKind::Module, m.name().map(|x| x.text().to_string()).unwrap_or_else(|| "mod".into()), false))
    } else if let Some(i) = ast::Impl::cast(n.clone()) {
        Some((NodeKind::Type, impl_name(&i), false))
    } else if let Some(t) = ast::Trait::cast(n.clone()) {
        Some((NodeKind::Type, t.name().map(|x| x.text().to_string()).unwrap_or_else(|| "trait".into()), false))
    } else if let Some(s) = ast::Struct::cast(n.clone()) {
        Some((NodeKind::Type, s.name().map(|x| x.text().to_string()).unwrap_or_default(), false))
    } else if let Some(e) = ast::Enum::cast(n.clone()) {
        Some((NodeKind::Type, e.name().map(|x| x.text().to_string()).unwrap_or_default(), false))
    } else { ast::Union::cast(n.clone()).map(|u| (NodeKind::Type, u.name().map(|x| x.text().to_string()).unwrap_or_default(), false)) };
    if let Some((kind, name, anon)) = seg {
        let segname = cx.name(&scope, &name, anon);
        let id = format!("{scope} › {segname}");
        let span = cx.span(n);
        cx.node(id.clone(), &parent, kind, segname, span, anon)?;
        scope = id.clone();
        parent = id;
    }

    // Blocks (BQ-18): if/else arms, match arms, loops.
    let blocks: Vec<(&str, SyntaxNode)> = if let Some(i) = ast::IfExpr::cast(n.clone()) {
        let mut v = Vec::new();
        if let Some(t) = i.then_branch() {
            v.push(("if", t.syntax().clone()));
        }
        if let Some(ast::ElseBranch::Block(b)) = i.else_branch() {
            v.push(("else", b.syntax().clone()));
        }
        v
    } else if let Some(a) = ast::MatchArm::cast(n.clone()) {
        a.expr().map(|e| vec![("arm", e.syntax().clone())]).unwrap_or_default()
    } else if let Some(l) = ast::LoopExpr::cast(n.clone()) {
        l.loop_body().map(|b| vec![("loop", b.syntax().clone())]).unwrap_or_default()
    } else if let Some(w) = ast::WhileExpr::cast(n.clone()) {
        w.loop_body().map(|b| vec![("while", b.syntax().clone())]).unwrap_or_default()
    } else if let Some(f) = ast::ForExpr::cast(n.clone()) {
        f.loop_body().map(|b| vec![("for", b.syntax().clone())]).unwrap_or_default()
    } else {
        Vec::new()
    };

    if blocks.is_empty() {
        for child in n.children() {
            visit(cx, &child, &scope, &parent)?;
        }
        return Ok(());
    }
    let bodies: Vec<SyntaxNode> = blocks.iter().map(|(_, b)| b.clone()).collect();
    for child in n.children() {
        if !bodies.contains(&child) {
            visit(cx, &child, &scope, &parent)?;
        }
    }
    for (label, body) in blocks {
        let name = cx.name(&format!("{scope}\0blocks"), label, true);
        let id = format!("{scope} › {name}");
        let span = cx.span(&body);
        cx.node(id.clone(), &parent, NodeKind::Block, label.to_string(), span, true)?;
        visit(cx, &body, &scope, &id)?;
    }
    Ok(())
}

fn impl_name(i: &ast::Impl) -> String {
    let ty = i.self_ty().map(|t| t.syntax().text().to_string()).unwrap_or_default();
    match i.trait_() {
        Some(tr) => format!("impl {} for {ty}", tr.syntax().text()),
        None => format!("impl {ty}"),
    }
}

/// Closures: named by their `let` binding, else by the call they're passed to, else `|…|`.
fn closure_name(c: &ast::ClosureExpr) -> (String, bool) {
    let Some(parent) = c.syntax().parent() else { return ("|…|".into(), true) };
    if let Some(l) = ast::LetStmt::cast(parent.clone())
        && let Some(ast::Pat::IdentPat(p)) = l.pat()
            && let Some(n) = p.name() {
                return (n.text().to_string(), false);
            }
    if parent.kind() == SyntaxKind::ARG_LIST
        && let Some(call) = parent.parent() {
            if let Some(mc) = ast::MethodCallExpr::cast(call.clone())
                && let Some(n) = mc.name_ref() {
                    return (n.text().to_string(), true);
                }
            if let Some(ce) = ast::CallExpr::cast(call)
                && let Some(e) = ce.expr() {
                    let t = e.syntax().text().to_string();
                    return (t.rsplit("::").take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("::"), true);
                }
        }
    ("|…|".into(), true)
}
