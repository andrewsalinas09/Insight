//! Data flow for Rust (SCHEMA.md §5): state nodes, `read` / `write` / `access`, and the
//! value edges `assign`, `binds`, `result`, `return`.
//!
//! Like C/C++, flow is by declaration, not by program point. State: `let`/parameter/
//! match bindings and `self` (`var`), struct fields (`field`), statics and consts
//! (`static`). Code is read as written; reads inside macro-call arguments are not seen
//! (they are token trees until expanded).

use std::collections::{HashMap, HashSet};
use std::io::Write;

use anyhow::Result;
use insight_model::{EdgeFact, EdgeKind, NodeFact, NodeKind, Record, Resolution, Span};
use ra_ap_hir::{HasCrate, HasSource, ModuleDef, PathResolution, Semantics};
use ra_ap_ide_db::RootDatabase;
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, TextRange, ast, ast::HasArgList, ast::HasName};
use ra_ap_vfs::FileId;

use super::{CallNode, Callee, Global, Indexed, Out, resolve_call, target_of};

/// Emits the state nodes of one file (after all containers, with their own ordinals, so
/// a variable never renames a function) and records them in `g.states`.
pub(crate) fn state_nodes<W: Write>(sema: &Semantics<'_, RootDatabase>, f: &Indexed, g: &mut Global<'_>, out: &mut Out<W>) -> Result<()> {
    let ids: HashSet<&str> = f.containers.iter().map(|c| c.id.as_str()).collect();
    let mut ordinals: HashMap<(String, String), u32> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::new();
    for n in f.tree.descendants() {
        let (name, kind) = match n.kind() {
            SyntaxKind::IDENT_PAT => {
                let Some(p) = ast::IdentPat::cast(n.clone()) else { continue };
                // `None` in a pattern is a variant, not a binding: only real locals are state.
                if sema.to_def(&p).is_none() {
                    continue;
                }
                let Some(nm) = p.name() else { continue };
                (nm.text().to_string(), None)
            }
            SyntaxKind::SELF_PARAM => ("self".to_string(), Some(NodeKind::Var)),
            SyntaxKind::RECORD_FIELD => {
                let Some(nm) = ast::RecordField::cast(n.clone()).and_then(|x| x.name()) else { continue };
                (nm.text().to_string(), Some(NodeKind::Field))
            }
            SyntaxKind::TUPLE_FIELD => {
                let i = n.parent().and_then(|l| l.children().position(|c| c == n)).unwrap_or(0);
                (i.to_string(), Some(NodeKind::Field))
            }
            SyntaxKind::STATIC => {
                let Some(nm) = ast::Static::cast(n.clone()).and_then(|x| x.name()) else { continue };
                (nm.text().to_string(), Some(NodeKind::Static))
            }
            SyntaxKind::CONST => {
                let Some(nm) = ast::Const::cast(n.clone()).and_then(|x| x.name()) else { continue };
                (nm.text().to_string(), Some(NodeKind::Static))
            }
            _ => continue,
        };
        let r = n.text_range();
        let (s, e) = (u32::from(r.start()), u32::from(r.end()));
        let around = |scope_only: bool| {
            f.containers
                .iter()
                .filter(|c| c.start <= s && e <= c.end && (c.scope || !scope_only))
                .min_by_key(|c| c.end - c.start)
                .map(|c| c.id.clone())
                .unwrap_or_else(|| f.file.clone())
        };
        let (parent, scope) = (around(false), around(true));
        let in_fn = f.containers.iter().any(|c| c.is_fn && c.start <= s && e <= c.end);
        let kind = kind.unwrap_or(if in_fn { NodeKind::Var } else { NodeKind::Static });
        let mut next = |anon: bool| {
            let k = ordinals.entry((scope.clone(), name.clone())).or_insert(0);
            *k += 1;
            if anon || *k > 1 { format!("{name}#{k}") } else { name.clone() }
        };
        let mut seg = next(false);
        while ids.contains(format!("{scope} › {seg}").as_str()) || taken.contains(&format!("{scope} › {seg}")) {
            seg = next(true);
        }
        let id = format!("{scope} › {seg}");
        taken.insert(id.clone());
        let span = Span { byte_start: s as u64, byte_end: e as u64, line_start: f.lines.line(s), line_end: f.lines.line(e.saturating_sub(1).max(s)) };
        out.emit(Record::Node(NodeFact { id: id.clone(), parent: Some(parent), kind, name: seg, span: Some(span), anonymous: false, generated: false }))?;
        g.states.insert((f.file.clone(), s, e), id);
    }
    Ok(())
}

/// The state node (or external leaf) for a syntax node of a declaration.
fn state_at_source(g: &mut Global<'_>, node: ra_ap_hir::InFile<&SyntaxNode>, external: impl FnOnce() -> String) -> Option<String> {
    let range = node.original_file_range_rooted(g.db);
    let fid = range.file_id.file_id(g.db);
    match g.paths.get(&(g.ix, fid)) {
        Some(path) => g.states.get(&(path.clone(), u32::from(range.range.start()), u32::from(range.range.end()))).cloned(),
        None => {
            let id = external();
            g.externals.insert(id.clone());
            Some(id)
        }
    }
}

fn local_state(g: &mut Global<'_>, l: ra_ap_hir::Local<'_>) -> Option<String> {
    let src = l.primary_source(g.db);
    let node = src.syntax().clone();
    state_at_source(g, src.source.with_value(&node), String::new).filter(|s| !s.is_empty())
}

fn field_state(g: &mut Global<'_>, fld: ra_ap_hir::Field) -> Option<String> {
    let db = g.db;
    let krate = fld.parent_def(db).module(db).krate(db);
    if !krate.origin(db).is_local() {
        let k = krate.display_name(db).map(|n| n.to_string()).unwrap_or_else(|| "?".into());
        let id = format!("ext:rust:{k}:{}::{}", fld.parent_def(db).name(db).as_str(), fld.name(db).as_str());
        g.externals.insert(id.clone());
        return Some(id);
    }
    let src = fld.source(db)?;
    let node = match &src.value {
        ra_ap_hir::FieldSource::Named(r) => r.syntax().clone(),
        ra_ap_hir::FieldSource::Pos(t) => t.syntax().clone(),
    };
    state_at_source(g, src.with_value(&node), String::new).filter(|s| !s.is_empty())
}

fn item_state(g: &mut Global<'_>, def: ModuleDef) -> Option<String> {
    let db = g.db;
    let (src_node, krate, name) = match def {
        ModuleDef::Static(s) => {
            let src = s.source(db)?;
            (src.map(|v| v.syntax().clone()), s.module(db).krate(db), s.name(db).as_str().to_string())
        }
        ModuleDef::Const(c) => {
            let src = c.source(db)?;
            (src.map(|v| v.syntax().clone()), c.module(db).krate(db), c.name(db).map(|n| n.as_str().to_string()).unwrap_or_default())
        }
        _ => return None,
    };
    if !krate.origin(db).is_local() {
        let k = krate.display_name(db).map(|n| n.to_string()).unwrap_or_else(|| "?".into());
        let id = format!("ext:rust:{k}:{name}");
        g.externals.insert(id.clone());
        return Some(id);
    }
    let node = src_node.value.clone();
    state_at_source(g, src_node.with_value(&node), String::new).filter(|s| !s.is_empty())
}

/// The state an expression names: a local, static/const, or field.
fn expr_state(sema: &Semantics<'_, RootDatabase>, g: &mut Global<'_>, e: &ast::Expr) -> Option<String> {
    match e {
        ast::Expr::PathExpr(pe) => match sema.resolve_path(&pe.path()?)? {
            PathResolution::Local(l) => local_state(g, l),
            PathResolution::Def(d) => item_state(g, d),
            _ => None,
        },
        ast::Expr::FieldExpr(fe) => match sema.resolve_field(fe)? {
            either::Either::Left(fld) => field_state(g, fld),
            either::Either::Right(_) => None,
        },
        _ => None,
    }
}

/// The state written by an assignment's left side: `x`, `s.f`, `a[i]` (the array), `*p` (through p).
fn place(sema: &Semantics<'_, RootDatabase>, g: &mut Global<'_>, e: &ast::Expr) -> Option<(String, Option<&'static str>, TextRange)> {
    match e {
        ast::Expr::PathExpr(_) | ast::Expr::FieldExpr(_) => expr_state(sema, g, e).map(|s| (s, None, e.syntax().text_range())),
        ast::Expr::IndexExpr(ix) => place(sema, g, &ix.base()?),
        ast::Expr::ParenExpr(p) => place(sema, g, &p.expr()?),
        ast::Expr::PrefixExpr(p) if p.op_kind() == Some(ast::UnaryOp::Deref) => place(sema, g, &p.expr()?).map(|(s, _, r)| (s, Some("pointee"), r)),
        _ => None,
    }
}

enum Src {
    State(String),
    Fn(String),
    Result(String),
}

/// Where an expression's value comes from (arguments of a call bind to its parameters).
fn sources(sema: &Semantics<'_, RootDatabase>, f: &Indexed, g: &mut Global<'_>, e: &ast::Expr, out: &mut Vec<Src>) {
    match e {
        ast::Expr::PathExpr(pe) => {
            if let Some(PathResolution::Def(ModuleDef::Function(func))) = pe.path().and_then(|p| sema.resolve_path(&p)) {
                if let (Some(id), false) = target_of(g, func) {
                    out.push(Src::Fn(id));
                }
            } else if let Some(s) = expr_state(sema, g, e) {
                out.push(Src::State(s));
            }
        }
        ast::Expr::FieldExpr(_) => {
            if let Some(s) = expr_state(sema, g, e) {
                out.push(Src::State(s));
            }
        }
        ast::Expr::ClosureExpr(c) => {
            let r = c.syntax().text_range();
            if let Some(id) = g.fns.get(&(f.file.clone(), u32::from(r.start()), u32::from(r.end()))) {
                out.push(Src::Fn(id.clone()));
            }
        }
        ast::Expr::CallExpr(c) => match resolve_call(sema, &CallNode::Call(c.clone())) {
            Callee::Fn(func) => {
                if let (Some(id), _) = target_of(g, func) {
                    out.push(Src::Result(id));
                }
            }
            // A tuple struct or variant built from its arguments: they flow into it.
            Callee::NotACall => {
                for a in c.arg_list().into_iter().flat_map(|l| l.args()) {
                    sources(sema, f, g, &a, out);
                }
            }
            _ => {}
        },
        ast::Expr::MethodCallExpr(m) => {
            if let Some(func) = sema.resolve_method_call(m)
                && let (Some(id), _) = target_of(g, func) {
                    out.push(Src::Result(id));
                }
        }
        ast::Expr::IndexExpr(ix) => {
            if let Some(b) = ix.base() {
                sources(sema, f, g, &b, out);
            }
        }
        ast::Expr::BlockExpr(b) => {
            if let Some(t) = b.stmt_list().and_then(|l| l.tail_expr()) {
                sources(sema, f, g, &t, out);
            }
        }
        ast::Expr::Literal(_) => {}
        _ => {
            for c in e.syntax().children().filter_map(ast::Expr::cast) {
                sources(sema, f, g, &c, out);
            }
        }
    }
}

struct Emit<'o, W: Write> {
    out: &'o mut Out<W>,
}

impl<W: Write> Emit<'_, W> {
    #[allow(clippy::too_many_arguments)]
    fn edge(&mut self, kind: EdgeKind, subkind: Option<&str>, from: &str, to: &str, resolution: Resolution, f: &Indexed, role: &str, r: TextRange) -> Result<()> {
        self.out.emit(Record::Edge(EdgeFact {
            kind,
            subkind: subkind.map(String::from),
            from: from.to_string(),
            to: Some(to.to_string()),
            resolution,
            group: None,
            receipts: vec![f.receipt(role, u32::from(r.start()), u32::from(r.end()))],
        }))
    }

    /// Value edges into a state (`assign`), a parameter (`binds`) or a returning function (`return`).
    #[allow(clippy::too_many_arguments)]
    fn flow(&mut self, srcs: Vec<Src>, to: &str, into: Into, mode: &str, f: &Indexed, role: &str, r: TextRange) -> Result<()> {
        for s in srcs {
            match (s, into) {
                (Src::Result(fun), _) => self.edge(EdgeKind::ValueFlow, Some("result"), &fun, to, Resolution::Exact, f, role, r)?,
                (Src::State(st), Into::Function) => self.edge(EdgeKind::ValueFlow, Some("return"), &st, to, Resolution::Exact, f, role, r)?,
                (Src::Fn(_), Into::Function) => {}
                (Src::State(x) | Src::Fn(x), Into::Param) => self.edge(EdgeKind::Binds, Some(mode), &x, to, Resolution::Exact, f, role, r)?,
                (Src::State(x) | Src::Fn(x), Into::State) => self.edge(EdgeKind::ValueFlow, Some("assign"), &x, to, Resolution::Exact, f, role, r)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Into {
    State,
    Param,
    Function,
}

fn param_mode(ty: Option<ast::Type>) -> &'static str {
    match ty {
        Some(ast::Type::RefType(r)) if r.mut_token().is_some() => "ref",
        Some(ast::Type::PtrType(_)) => "pointer",
        _ => "value",
    }
}

/// The parameter states of a project function: (self, then each parameter's binding), with modes.
/// A parameter's state and its binding mode.
type Param = Option<(String, &'static str)>;

fn params(g: &mut Global<'_>, func: ra_ap_hir::Function) -> Option<(Param, Vec<Param>)> {
    if !func.krate(g.db).origin(g.db).is_local() {
        return None;
    }
    let src = func.source(g.db)?;
    let list = src.value.param_list()?;
    let mut at = |node: &SyntaxNode| state_at_source(g, src.with_value(node), String::new).filter(|s| !s.is_empty());
    let this = list.self_param().and_then(|sp| {
        let mode = if sp.amp_token().is_some() && sp.mut_token().is_some() { "ref" } else { "value" };
        at(sp.syntax()).map(|s| (s, mode))
    });
    let ps = list
        .params()
        .map(|p| match p.pat() {
            Some(ast::Pat::IdentPat(ip)) => at(ip.syntax()).map(|s| (s, param_mode(p.ty()))),
            _ => None,
        })
        .collect();
    Some((this, ps))
}

/// A call through a local function value: the state it calls (the solver finds the candidates).
pub(crate) fn called_state(sema: &Semantics<'_, RootDatabase>, g: &mut Global<'_>, call: &CallNode) -> Option<String> {
    let CallNode::Call(c) = call else { return None };
    let callee = c.expr()?;
    match &callee {
        ast::Expr::PathExpr(_) | ast::Expr::FieldExpr(_) => expr_state(sema, g, &callee),
        ast::Expr::ParenExpr(p) => expr_state(sema, g, &p.expr()?),
        _ => None,
    }
}

/// Data edges for one file: writes, assignments, bindings, returns, then reads.
pub(crate) fn data_for_file<W: Write>(sema: &Semantics<'_, RootDatabase>, f: &Indexed, _id: FileId, g: &mut Global<'_>, out: &mut Out<W>) -> Result<()> {
    let mut em = Emit { out };
    let mut writes: HashSet<TextRange> = HashSet::new();
    let fn_of = |r: TextRange| f.innermost(u32::from(r.start()), u32::from(r.end()), true);
    for n in f.tree.descendants() {
        let r = n.text_range();
        if let Some(l) = ast::LetStmt::cast(n.clone()) {
            let from = fn_of(r);
            let srcs_init = || l.initializer();
            for ip in l.pat().into_iter().flat_map(|p| p.syntax().descendants().filter_map(ast::IdentPat::cast).collect::<Vec<_>>()) {
                let ir = ip.syntax().text_range();
                let Some(target) = g.states.get(&(f.file.clone(), u32::from(ir.start()), u32::from(ir.end()))).cloned() else { continue };
                if let Some(init) = srcs_init() {
                    em.edge(EdgeKind::Write, None, &from, &target, Resolution::Exact, f, "write", r)?;
                    let mut srcs = Vec::new();
                    sources(sema, f, g, &init, &mut srcs);
                    em.flow(srcs, &target, Into::State, "value", f, "write", r)?;
                }
            }
        } else if let Some(b) = ast::BinExpr::cast(n.clone()) {
            let Some(ast::BinaryOp::Assignment { op }) = b.op_kind() else { continue };
            let (Some(lhs), Some(rhs)) = (b.lhs(), b.rhs()) else { continue };
            let Some((target, sub, at)) = place(sema, g, &lhs) else { continue };
            writes.insert(at);
            let from = fn_of(r);
            em.edge(EdgeKind::Write, sub, &from, &target, Resolution::Exact, f, "write", r)?;
            if op.is_some() {
                em.edge(EdgeKind::Read, None, &from, &target, Resolution::Exact, f, "write", r)?;
            }
            let mut srcs = Vec::new();
            sources(sema, f, g, &rhs, &mut srcs);
            em.flow(srcs, &target, Into::State, "value", f, "write", r)?;
        } else if let Some(rx) = ast::ReturnExpr::cast(n.clone()) {
            if let Some(e) = rx.expr() {
                let mut srcs = Vec::new();
                sources(sema, f, g, &e, &mut srcs);
                em.flow(srcs, &fn_of(r), Into::Function, "value", f, "return", r)?;
            }
        } else if let Some(func) = ast::Fn::cast(n.clone()) {
            if let Some(tail) = func.body().and_then(|b| b.stmt_list()).and_then(|l| l.tail_expr()) {
                let fid = g.fns.get(&(f.file.clone(), u32::from(r.start()), u32::from(r.end()))).cloned();
                if let Some(fid) = fid {
                    let mut srcs = Vec::new();
                    sources(sema, f, g, &tail, &mut srcs);
                    em.flow(srcs, &fid, Into::Function, "value", f, "return", tail.syntax().text_range())?;
                }
            }
        } else if let Some(c) = ast::ClosureExpr::cast(n.clone()) {
            if let (Some(body), Some(fid)) = (c.body(), g.fns.get(&(f.file.clone(), u32::from(r.start()), u32::from(r.end()))).cloned()) {
                let mut srcs = Vec::new();
                sources(sema, f, g, &body, &mut srcs);
                em.flow(srcs, &fid, Into::Function, "value", f, "return", body.syntax().text_range())?;
            }
        } else if let Some(re) = ast::RecordExprField::cast(n.clone()) {
            // `S { f: x }`: x flows into the field.
            if let (Some((fld, _, _)), Some(e)) = (sema.resolve_record_field(&re), re.expr())
                && let Some(target) = field_state(g, fld) {
                    let mut srcs = Vec::new();
                    sources(sema, f, g, &e, &mut srcs);
                    em.flow(srcs, &target, Into::State, "value", f, "write", r)?;
                }
        } else if ast::CallExpr::can_cast(n.kind()) || ast::MethodCallExpr::can_cast(n.kind()) {
            let (call, func) = if let Some(c) = ast::CallExpr::cast(n.clone()) {
                let call = CallNode::Call(c);
                match resolve_call(sema, &call) {
                    Callee::Fn(func) => (call, func),
                    _ => continue,
                }
            } else {
                let m = ast::MethodCallExpr::cast(n.clone()).expect("checked");
                let Some(func) = sema.resolve_method_call(&m) else { continue };
                (CallNode::Method(m), func)
            };
            let from = fn_of(r);
            let (to_ext, ps) = match params(g, func) {
                Some(p) => (None, Some(p)),
                None => (target_of(g, func).0, None),
            };
            let mut args: Vec<ast::Expr> = Vec::new();
            let mut targets: Vec<Option<(String, &'static str)>> = Vec::new();
            match (&call, &ps) {
                (CallNode::Method(m), Some((this, list))) => {
                    args.extend(m.receiver());
                    targets.push(this.clone());
                    args.extend(call.args());
                    targets.extend(list.iter().cloned());
                }
                (CallNode::Call(_), Some((this, list))) => {
                    args.extend(call.args());
                    if this.is_some() {
                        targets.push(this.clone());
                    }
                    targets.extend(list.iter().cloned());
                }
                (CallNode::Method(m), None) => {
                    args.extend(m.receiver());
                    args.extend(call.args());
                }
                (CallNode::Call(_), None) => args.extend(call.args()),
            }
            for (i, a) in args.iter().enumerate() {
                let ar = a.syntax().text_range();
                let mut srcs = Vec::new();
                sources(sema, f, g, a, &mut srcs);
                let mutable_borrow = matches!(a, ast::Expr::RefExpr(re) if re.mut_token().is_some());
                let (to, mode) = match (&ps, &to_ext) {
                    (Some(_), _) => match targets.get(i).cloned().flatten() {
                        Some((t, m)) => (t, m),
                        None => continue,
                    },
                    (None, Some(ext)) => (ext.clone(), if mutable_borrow { "ref" } else { "value" }),
                    _ => continue,
                };
                if mode == "ref" || mutable_borrow {
                    for s in &srcs {
                        if let Src::State(st) = s {
                            em.edge(EdgeKind::Access, None, &from, st, Resolution::Exact, f, "arg", ar)?;
                        }
                    }
                }
                let srcs: Vec<Src> = if ps.is_none() { srcs.into_iter().filter(|s| !matches!(s, Src::Fn(_))).collect() } else { srcs };
                em.flow(srcs, &to, Into::Param, mode, f, "arg", ar)?;
            }
        }
    }
    // Reads: every name of a state not in a write position.
    for n in f.tree.descendants() {
        let Some(e) = ast::Expr::cast(n.clone()) else { continue };
        if !matches!(e, ast::Expr::PathExpr(_) | ast::Expr::FieldExpr(_)) || writes.contains(&n.text_range()) {
            continue;
        }
        if let Some(s) = expr_state(sema, g, &e) {
            let r = n.text_range();
            em.edge(EdgeKind::Read, None, &fn_of(r), &s, Resolution::Exact, f, "read", r)?;
        }
    }
    Ok(())
}
