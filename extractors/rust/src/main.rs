//! Insight's Rust extractor (ADR-0034, R2-27).
//!
//! M0 uses rust-analyzer's own parser crate (`ra_ap_syntax`) to emit containers:
//! files, modules, types, impls, functions (including closures) and blocks.
//! Semantic resolution (calls, reads/writes, macro expansion) arrives at M1 with
//! rust-analyzer's semantic crates. Knows the Rust language only, no frameworks
//! (ADR-0048). Output: NDJSON on stdout (R2-28).
//!
//! Usage: insight-extract-rust --root <dir> --package <dir> [--package …] --schema <N>

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use insight_model::{FACT_SCHEMA_VERSION, End, FileFact, Header, NodeFact, NodeKind, Record, Span};
use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind, SyntaxNode, ast, ast::HasLoopBody, ast::HasName};
use sha2::{Digest, Sha256};

const EXTRACTOR: &str = concat!("rust@", env!("CARGO_PKG_VERSION"));

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
        toolchain: "ra_ap_syntax (pinned in Cargo.lock)".into(),
    }))?;
    for abs in files {
        extract_file(&root, &abs, &mut out)?;
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
}

impl<W: Write> FileCx<'_, W> {
    fn name(&mut self, scope: &str, name: &str, anon: bool) -> String {
        let k = self.ordinals.entry((scope.to_string(), name.to_string())).or_insert(0);
        *k += 1;
        if anon || *k > 1 { format!("{name}#{k}") } else { name.to_string() }
    }

    fn span(&self, n: &SyntaxNode) -> Span {
        let r = n.text_range();
        let (s, e) = (u32::from(r.start()), u32::from(r.end()));
        Span { byte_start: s as u64, byte_end: e as u64, line_start: self.lines.line(s), line_end: self.lines.line(e.saturating_sub(1).max(s)) }
    }

    fn node(&mut self, id: String, parent: &str, kind: NodeKind, name: String, span: Span) -> Result<String> {
        self.out.emit(Record::Node(NodeFact { id: id.clone(), parent: Some(parent.to_string()), kind, name, span }))?;
        Ok(id)
    }
}

fn extract_file<W: Write>(root: &Path, abs: &Path, out: &mut Out<W>) -> Result<()> {
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
    let parse = SourceFile::parse(&text, Edition::Edition2024);
    let mut cx = FileCx { out, lines, ordinals: HashMap::new() };
    for child in parse.tree().syntax().children() {
        visit(&mut cx, &child, &file, &file)?;
    }
    Ok(())
}

/// `scope` = nearest path segment (file/module/type/impl/function); `parent` = nearest container.
fn visit<W: Write>(cx: &mut FileCx<W>, n: &SyntaxNode, scope: &str, parent: &str) -> Result<()> {
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
