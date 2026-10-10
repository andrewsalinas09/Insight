//! Data flow for C/C++ (SCHEMA.md §5): state nodes, `read` / `write` / `access`, and
//! the value edges `assign`, `binds`, `result`, `return`, `returns`.
//!
//! Flow is by declaration, not by program point: `b = a` connects the state `a` to the
//! state `b`, wherever it happens. What an expression's value comes from:
//! - a variable, parameter or field it names (through casts, parentheses, arithmetic,
//!   `&x`, `*p`, `p[i]`'s base, and both arms of `?:`);
//! - a function or lambda used as a value;
//! - a call's result (a call's arguments bind to the callee's parameters; they do not
//!   flow into the result directly). A constructor call is transparent: its arguments
//!   flow into the constructed value.

use anyhow::Result;
use clang::{Entity, EntityKind, TypeKind};
use insight_model::{EdgeKind, NodeKind, Receipt};

use super::{Ctx, Extractor, external_id};

/// One end of a data edge, resolved at the end of the stream.
#[derive(Clone, Debug)]
pub(crate) enum Ref {
    Node(String),
    /// A variable, parameter or field by USR; the external leaf if declared outside the root.
    State { usr: String, ext: Option<(String, String)> },
    /// A function by its USRs (declaration, then template).
    Fn { usrs: Vec<String>, ext: Option<(String, String)> },
    Lambda(String),
    /// The `index`-th parameter of a function; the external leaf if the function is external.
    Param { usrs: Vec<String>, index: usize, ext: Option<(String, String)> },
}

pub(crate) struct PendingData {
    pub kind: EdgeKind,
    pub subkind: Option<String>,
    pub from: Ref,
    pub to: Ref,
    pub receipt: Receipt,
}

/// Where a value comes from.
enum Src {
    State(Ref),
    FnValue(Ref),
    Result(Ref),
}

fn is_state(k: EntityKind) -> bool {
    matches!(k, EntityKind::VarDecl | EntityKind::ParmDecl | EntityKind::FieldDecl)
}

fn is_fn(k: EntityKind) -> bool {
    matches!(k, EntityKind::FunctionDecl | EntityKind::Method | EntityKind::FunctionTemplate | EntityKind::Constructor | EntityKind::Destructor | EntityKind::ConversionFunction)
}

/// The tokens between two entities (an operator's spelling).
fn between(a: Entity, b: Entity) -> Vec<String> {
    match (a.get_range(), b.get_range()) {
        (Some(ra), Some(rb)) => clang::source::SourceRange::new(ra.get_end(), rb.get_start()).tokenize().iter().map(|t| t.get_spelling()).collect(),
        _ => Vec::new(),
    }
}

impl Extractor {
    pub(crate) fn state_ref(&self, d: Entity) -> Result<Option<Ref>> {
        let Some(usr) = d.get_usr() else { return Ok(None) };
        let ext = if self.decl_inside(d)? { None } else { Some(external_id(d)) };
        Ok(Some(Ref::State { usr: usr.0, ext }))
    }

    pub(crate) fn fn_ref(&self, d: Entity) -> Result<Ref> {
        let mut usrs: Vec<String> = d.get_usr().map(|u| u.0).into_iter().collect();
        let mut t = d.get_template();
        while let Some(tt) = t {
            usrs.extend(tt.get_usr().map(|u| u.0));
            t = tt.get_template();
        }
        let ext = if self.decl_inside(d)? && !usrs.is_empty() { None } else { Some(external_id(d)) };
        Ok(Ref::Fn { usrs, ext })
    }

    /// Emits the state node for a variable, parameter or field definition.
    pub(crate) fn state_node(&mut self, e: Entity, ctx: &Ctx, kind: NodeKind) -> Result<()> {
        let Some((rel, s, en)) = self.range_in(e.get_range())? else { return Ok(()) };
        if rel != ctx.file {
            return Ok(());
        }
        let (name, anon) = match e.get_name().filter(|n| !n.is_empty()) {
            Some(n) => (n, false),
            None => ("(unnamed)".to_string(), true),
        };
        // Own ordinal namespace: adding a variable never renames a function or lambda.
        let mut seg = self.next_name(&format!("{}\u{0}state", ctx.scope), &name, anon);
        while self.ids.contains(&format!("{} › {seg}", ctx.scope)) {
            seg = self.next_name(&format!("{}\u{0}state", ctx.scope), &name, true);
        }
        let Some(span) = self.span_of(&rel, s, en) else { return Ok(()) };
        let generated = self.generated(e);
        let id = self.node(ctx, &seg, &seg, kind, span, anon, generated)?;
        if let Some(u) = e.get_usr() {
            self.state_by_usr.entry(u.0).or_insert(id);
        }
        Ok(())
    }

    fn data(&mut self, kind: EdgeKind, subkind: Option<&str>, from: Ref, to: Ref, receipt: Receipt) {
        self.data_edges.push(PendingData { kind, subkind: subkind.map(String::from), from, to, receipt });
    }

    fn receipt_of(&mut self, e: Entity, role: &str, ctx: &Ctx) -> Result<Option<Receipt>> {
        Ok(self.range_in(e.get_range())?.map(|(rel, s, en)| self.receipt(role, &rel, s, en, &ctx.parent)))
    }

    /// What an expression's value comes from (see the module comment).
    fn sources(&mut self, e: Entity, out: &mut Vec<Src>) -> Result<()> {
        use EntityKind as K;
        match e.get_kind() {
            K::DeclRefExpr | K::MemberRefExpr => {
                if let Some(r) = e.get_reference() {
                    if is_state(r.get_kind()) {
                        if let Some(s) = self.state_ref(r)? {
                            out.push(Src::State(s));
                        }
                    } else if is_fn(r.get_kind()) {
                        out.push(Src::FnValue(self.fn_ref(r)?));
                    }
                }
            }
            K::LambdaExpr => {
                if let Some((rel, s, _)) = self.range_in(e.get_range())? {
                    out.push(Src::FnValue(Ref::Lambda(format!("{rel}@{s}"))));
                }
            }
            K::CallExpr => match e.get_reference() {
                Some(r) if r.get_kind() == K::Constructor => {
                    for a in e.get_arguments().unwrap_or_default() {
                        self.sources(a, out)?;
                    }
                }
                Some(r) if is_fn(r.get_kind()) => out.push(Src::Result(self.fn_ref(r)?)),
                _ => {}
            },
            K::ArraySubscriptExpr => {
                if let Some(base) = e.get_children().first() {
                    self.sources(*base, out)?;
                }
            }
            K::ConditionalOperator => {
                for c in e.get_children().into_iter().skip(1) {
                    self.sources(c, out)?;
                }
            }
            K::StringLiteral | K::IntegerLiteral | K::FloatingLiteral | K::CharacterLiteral | K::BoolLiteralExpr | K::NullPtrLiteralExpr => {}
            K::UnaryExpr => {} // sizeof / alignof: no value flows
            _ => {
                for c in e.get_children() {
                    self.sources(c, out)?;
                }
            }
        }
        Ok(())
    }

    /// The state an assignment writes: `x`, `o.f`, `a[i]` (the array), `*p` (through p).
    fn write_target<'t>(&mut self, e: Entity<'t>) -> Result<Option<(Ref, Option<&'static str>, Entity<'t>)>> {
        use EntityKind as K;
        match e.get_kind() {
            K::DeclRefExpr | K::MemberRefExpr => match e.get_reference().filter(|r| is_state(r.get_kind())) {
                Some(r) => Ok(self.state_ref(r)?.map(|s| (s, None, e))),
                None => Ok(None),
            },
            K::ArraySubscriptExpr => match e.get_children().first() {
                Some(b) => self.write_target(*b),
                None => Ok(None),
            },
            K::UnaryOperator => {
                let deref = e.get_range().is_some_and(|r| r.tokenize().first().is_some_and(|t| t.get_spelling() == "*"));
                match e.get_children().first() {
                    Some(c) => Ok(self.write_target(*c)?.map(|(s, sub, at)| (s, if deref { Some("pointee") } else { sub }, at))),
                    None => Ok(None),
                }
            }
            K::ParenExpr | K::UnexposedExpr => match e.get_children().first() {
                Some(c) => self.write_target(*c),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Value edges from `srcs` into `to` (a state, or a function receiving a returned value).
    fn flow_into(&mut self, srcs: Vec<Src>, to: Ref, into_function: bool, receipt: &Receipt) {
        for s in srcs {
            match s {
                Src::State(st) if into_function => self.data(EdgeKind::ValueFlow, Some("return"), st, to.clone(), receipt.clone()),
                Src::State(st) => self.data(EdgeKind::ValueFlow, Some("assign"), st, to.clone(), receipt.clone()),
                Src::FnValue(f) if into_function => self.data(EdgeKind::ValueFlow, Some("returns"), to.clone(), f, receipt.clone()),
                Src::FnValue(f) => self.data(EdgeKind::ValueFlow, Some("assign"), f, to.clone(), receipt.clone()),
                Src::Result(f) => self.data(EdgeKind::ValueFlow, Some("result"), f, to.clone(), receipt.clone()),
            }
        }
    }

    /// `x = …`, `x += …`, `x++`: a write (and, for compound forms, a read) plus value edges.
    pub(crate) fn assignment(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        use EntityKind as K;
        let kids = e.get_children();
        let Some(receipt) = self.receipt_of(e, "write", ctx)? else { return Ok(()) };
        let from = Ref::Node(ctx.func.clone());
        match e.get_kind() {
            K::BinaryOperator | K::CompoundAssignOperator if kids.len() == 2 => {
                let op = between(kids[0], kids[1]).into_iter().find(|t| !t.is_empty()).unwrap_or_default();
                let assigns = op == "=" || (op.len() >= 2 && op.ends_with('=') && !matches!(op.as_str(), "==" | "!=" | "<=" | ">="));
                if !assigns {
                    return Ok(());
                }
                if let Some((target, sub, at)) = self.write_target(kids[0])? {
                    let p = self.pos(at)?;
                    self.skip_read.insert(p);
                    self.data(EdgeKind::Write, sub, from.clone(), target.clone(), receipt.clone());
                    if op != "=" {
                        self.data(EdgeKind::Read, None, from, target.clone(), receipt.clone());
                    }
                    let mut srcs = Vec::new();
                    self.sources(kids[1], &mut srcs)?;
                    self.flow_into(srcs, target, false, &receipt);
                }
            }
            K::UnaryOperator if kids.len() == 1 => {
                let toks: Vec<String> = e.get_range().map(|r| r.tokenize().iter().map(|t| t.get_spelling()).collect()).unwrap_or_default();
                let step = |t: Option<&String>| t.is_some_and(|t| t == "++" || t == "--");
                if step(toks.first()) || step(toks.last()) {
                    if let Some((target, sub, at)) = self.write_target(kids[0])? {
                        let p = self.pos(at)?;
                    self.skip_read.insert(p);
                        self.data(EdgeKind::Read, None, from.clone(), target.clone(), receipt.clone());
                        self.data(EdgeKind::Write, sub, from, target, receipt);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `T v = init;`: a write of `v` plus value edges from the initializer.
    pub(crate) fn initializer(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        let Some(init) = e.get_children().into_iter().rev().find(|c| c.is_expression()) else { return Ok(()) };
        let Some(target) = self.state_ref(e)? else { return Ok(()) };
        let Some(receipt) = self.receipt_of(e, "write", ctx)? else { return Ok(()) };
        self.data(EdgeKind::Write, None, Ref::Node(ctx.func.clone()), target.clone(), receipt.clone());
        let mut srcs = Vec::new();
        self.sources(init, &mut srcs)?;
        self.flow_into(srcs, target, false, &receipt);
        Ok(())
    }

    /// `return expr;`: the function's result comes from these states, functions and calls.
    pub(crate) fn return_stmt(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        if ctx.func == ctx.file {
            return Ok(());
        }
        let Some(expr) = e.get_children().into_iter().next() else { return Ok(()) };
        let Some(receipt) = self.receipt_of(e, "return", ctx)? else { return Ok(()) };
        let mut srcs = Vec::new();
        self.sources(expr, &mut srcs)?;
        self.flow_into(srcs, Ref::Node(ctx.func.clone()), true, &receipt);
        Ok(())
    }

    /// A call's arguments bind to the callee's parameters (or to the external callee); an
    /// argument passed by non-const reference or as `&x` is a may-write (`access`).
    pub(crate) fn bind_args(&mut self, e: Entity, callee: Entity, ctx: &Ctx) -> Result<()> {
        let args = e.get_arguments().unwrap_or_default();
        if args.is_empty() {
            return Ok(());
        }
        let params = callee.get_arguments().unwrap_or_default();
        let Ref::Fn { usrs, ext } = self.fn_ref(callee)? else { return Ok(()) };
        for (i, a) in args.iter().enumerate() {
            let Some(receipt) = self.receipt_of(*a, "arg", ctx)? else { continue };
            let pty = params.get(i).and_then(|p| p.get_type());
            let mode = match pty.map(|t| t.get_kind()) {
                Some(TypeKind::LValueReference) if !pty.and_then(|t| t.get_pointee_type()).is_some_and(|p| p.is_const_qualified()) => "ref",
                Some(TypeKind::Pointer) => "pointer",
                _ => "value",
            };
            if params.get(i).is_none() && ext.is_none() {
                continue; // variadic tail of an analyzed function: no parameter to bind to
            }
            let to = match &ext {
                Some((id, name)) => Ref::Node(self.external(id, name)),
                None => Ref::Param { usrs: usrs.clone(), index: i, ext: None },
            };
            let mut srcs = Vec::new();
            self.sources(*a, &mut srcs)?;
            let address_of = a.get_range().is_some_and(|r| r.tokenize().first().is_some_and(|t| t.get_spelling() == "&"));
            for s in srcs {
                match s {
                    Src::State(st) => {
                        if mode == "ref" || (mode == "pointer" && address_of) {
                            self.data(EdgeKind::Access, None, Ref::Node(ctx.func.clone()), st.clone(), receipt.clone());
                        }
                        self.data(EdgeKind::Binds, Some(mode), st, to.clone(), receipt.clone());
                    }
                    Src::FnValue(f) => self.data(EdgeKind::Binds, Some(mode), f, to.clone(), receipt.clone()),
                    Src::Result(f) => self.data(EdgeKind::ValueFlow, Some("result"), f, to.clone(), receipt.clone()),
                }
            }
        }
        Ok(())
    }

    /// A read of a variable, parameter or field (not one in a write position).
    pub(crate) fn read(&mut self, e: Entity, ctx: &Ctx) -> Result<()> {
        let Some(r) = e.get_reference().filter(|r| is_state(r.get_kind())) else { return Ok(()) };
        let p = self.pos(e)?;
        if self.skip_read.contains(&p) {
            return Ok(());
        }
        let Some(target) = self.state_ref(r)? else { return Ok(()) };
        let Some(receipt) = self.receipt_of(e, "read", ctx)? else { return Ok(()) };
        self.data(EdgeKind::Read, None, Ref::Node(ctx.func.clone()), target, receipt);
        Ok(())
    }

    fn pos(&mut self, e: Entity) -> Result<(String, usize)> {
        Ok(self.range_in(e.get_range())?.map(|(rel, s, _)| (rel, s)).unwrap_or_default())
    }

    /// Resolves an edge end to a node id (state, function, parameter, lambda or external leaf).
    pub(crate) fn resolve(&mut self, r: &Ref) -> Option<String> {
        match r {
            Ref::Node(id) => Some(id.clone()),
            Ref::State { usr, ext } => self.state_by_usr.get(usr).cloned().or_else(|| ext.as_ref().map(|(id, name)| self.external(id, name))),
            Ref::Fn { usrs, ext } => usrs.iter().find_map(|u| self.fn_by_usr.get(u)).cloned().or_else(|| ext.as_ref().map(|(id, name)| self.external(id, name))),
            Ref::Lambda(pos) => self.lambda_by_pos.get(pos).cloned(),
            Ref::Param { usrs, index, ext } => usrs
                .iter()
                .find_map(|u| self.fn_params.get(u).and_then(|p| p.get(*index)))
                .cloned()
                .or_else(|| ext.as_ref().map(|(id, name)| self.external(id, name))),
        }
    }
}
