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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_model::{Diagnostic, FACT_SCHEMA_VERSION, End, FileFact, Header, NodeFact, NodeKind, Record, Severity, Span};
use ra_ap_hir::Semantics;
use ra_ap_ide_db::RootDatabase;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode, T, ast, ast::HasLoopBody, ast::HasName};
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
    for abs in files {
        let sema = owner.get(&norm(&abs.to_string_lossy())).map(|(i, id)| (&semas[*i], *id));
        extract_file(&root, &abs, sema, &mut out)?;
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

    fn node(&mut self, id: String, parent: &str, kind: NodeKind, name: String, span: Span) -> Result<String> {
        self.out.emit(Record::Node(NodeFact { id: id.clone(), parent: Some(parent.to_string()), kind, name, span }))?;
        Ok(id)
    }
}

fn extract_file<W: Write>(root: &Path, abs: &Path, sema: Option<(&Semantics<'_, RootDatabase>, FileId)>, out: &mut Out<W>) -> Result<()> {
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
        span: Span { byte_start: 0, byte_end: bytes.len() as u64, line_start: 1, line_end: last_line },
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
    let mut cx = FileCx { out, lines, ordinals: HashMap::new(), macro_fns };
    for child in tree.children() {
        visit(&mut cx, &child, &file, &file)?;
    }
    Ok(())
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
        cx.node(id.clone(), &pa, NodeKind::Function, seg, span)?;
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
    } else if let Some(u) = ast::Union::cast(n.clone()) {
        Some((NodeKind::Type, u.name().map(|x| x.text().to_string()).unwrap_or_default(), false))
    } else {
        None
    };
    if let Some((kind, name, anon)) = seg {
        let segname = cx.name(&scope, &name, anon);
        let id = format!("{scope} › {segname}");
        let span = cx.span(n);
        cx.node(id.clone(), &parent, kind, segname, span)?;
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
        cx.node(id.clone(), &parent, NodeKind::Block, label.to_string(), span)?;
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
    if let Some(l) = ast::LetStmt::cast(parent.clone()) {
        if let Some(ast::Pat::IdentPat(p)) = l.pat() {
            if let Some(n) = p.name() {
                return (n.text().to_string(), false);
            }
        }
    }
    if parent.kind() == SyntaxKind::ARG_LIST {
        if let Some(call) = parent.parent() {
            if let Some(mc) = ast::MethodCallExpr::cast(call.clone()) {
                if let Some(n) = mc.name_ref() {
                    return (n.text().to_string(), true);
                }
            }
            if let Some(ce) = ast::CallExpr::cast(call) {
                if let Some(e) = ce.expr() {
                    let t = e.syntax().text().to_string();
                    return (t.rsplit("::").take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("::"), true);
                }
            }
        }
    }
    ("|…|".into(), true)
}
