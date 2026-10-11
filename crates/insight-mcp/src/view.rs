//! `show(view)`: what Claude puts on screen (ADR-0040), checked and resolved to
//! node ids before the window sees it. Only references to code must resolve; text
//! is never policed (ADR-0037). The layout section lets Claude steer where boxes
//! go (ADR-0050): chains laid left to right in a row, an order within a box, and
//! layout options passed to the layout engine.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use anyhow::{Context, Result, bail};
use insight_solver::path::{Follow, Options, PathResult};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::Loaded;

/// A reference to code: a node id, a path suffix (as `path` and `source` take them),
/// or a `path` search whose route stands in for the list.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Ref {
    Id(i64),
    Name(String),
    Path { path: PathSpec },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PathSpec {
    #[serde(default)]
    from: Vec<String>,
    #[serde(default)]
    to: Vec<String>,
    follow: Vec<String>,
    max_depth: Option<usize>,
    max_occurrences: Option<usize>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViewIn {
    title: Option<String>,
    #[serde(default)]
    open: Vec<Ref>,
    #[serde(default)]
    highlight: Vec<Ref>,
    focus: Option<Ref>,
    #[serde(default)]
    notes: Vec<NoteIn>,
    #[serde(default)]
    steps: Vec<StepIn>,
    #[serde(default)]
    layout: LayoutIn,
    #[serde(default)]
    play: bool,
    trace: Option<TraceIn>,
    /// A detour: shown on top of the current view, which the user returns to (view and
    /// step) with one click.
    #[serde(default)]
    aside: bool,
}

/// Walk a function's calls in the order written, like stepping in a debugger: the
/// call stack (outermost first; the last is the function shown) and the line to start at.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceIn {
    stack: Vec<Ref>,
    line: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteIn {
    on: Option<Ref>,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepIn {
    focus: Option<Ref>,
    #[serde(default)]
    highlight: Vec<Ref>,
    #[serde(default)]
    open: Vec<Ref>,
    note: Option<String>,
    trace: Option<TraceIn>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LayoutIn {
    /// Each chain is laid out left to right in this order, on one row where it can be.
    #[serde(default)]
    chain: Vec<Vec<Ref>>,
    /// These come first within their box, in this order (top to bottom).
    #[serde(default)]
    order: Vec<Ref>,
    /// Layout engine options (ELK layered), e.g. spacing or node placement.
    #[serde(default)]
    options: BTreeMap<String, Value>,
}

/// Options the window's own drawing depends on.
const FIXED_OPTIONS: &[&str] = &["elk.algorithm", "elk.hierarchyHandling"];

pub(crate) fn resolve(l: &Loaded, raw: &Value) -> Result<Value> {
    let v: ViewIn = serde_json::from_value(raw.clone()).context("the view doesn't match the `show` schema")?;
    let mut r = Resolver { l, cache: HashMap::new(), cites: Vec::new() };
    let focus = v.focus.as_ref().map(|x| r.one(x)).transpose()?;
    let trace = v.trace.as_ref().map(|t| r.trace(t)).transpose()?;
    // What `:line` counts in, view-wide: the traced thing, else the focus.
    let shown = trace.as_ref().and_then(|t| t["stack"].as_array()?.last()?.as_i64()).or(focus);
    let mut notes = Vec::new();
    for (i, n) in v.notes.iter().enumerate() {
        let on = n.on.as_ref().map(|x| r.one(x)).transpose()?;
        let text = r.cite(&n.text, on.or(shown)).with_context(|| format!("note {}", i + 1))?;
        notes.push(json!({"on": on, "text": text}));
    }
    let mut steps = Vec::new();
    for (i, s) in v.steps.iter().enumerate() {
        let step = (|| -> Result<Value> {
            let focus = s.focus.as_ref().map(|x| r.one(x)).transpose()?;
            let trace = s.trace.as_ref().map(|t| r.trace(t)).transpose()?;
            let here = trace.as_ref().and_then(|t| t["stack"].as_array()?.last()?.as_i64()).or(focus).or(shown);
            let note = s.note.as_ref().map(|t| r.cite(t, here)).transpose()?;
            Ok(json!({
                "focus": focus,
                "highlight": r.many(&s.highlight)?,
                "open": r.many(&s.open)?,
                "note": note,
                "trace": trace,
            }))
        })()
        .with_context(|| format!("step {}", i + 1))?;
        steps.push(step);
    }
    let mut chain = Vec::new();
    for c in &v.layout.chain {
        let ids = r.route(c)?;
        if ids.len() > 1 {
            chain.push(ids);
        }
    }
    for k in v.layout.options.keys() {
        if FIXED_OPTIONS.contains(&k.as_str()) {
            bail!("layout option `{k}` is fixed by the window");
        }
        if !k.starts_with("elk.") {
            bail!("layout option `{k}` is not an ELK option (they start with `elk.`)");
        }
    }
    let options: BTreeMap<&String, String> = v.layout.options.iter().map(|(k, x)| (k, x.as_str().map_or_else(|| x.to_string(), String::from))).collect();
    Ok(json!({
        "title": v.title,
        "open": r.many(&v.open)?,
        "highlight": r.many(&v.highlight)?,
        "focus": focus,
        "notes": notes,
        "steps": steps,
        "play": v.play,
        "aside": v.aside,
        "trace": trace,
        "layout": {"chain": chain, "order": r.many(&v.layout.order)?, "options": options},
        "cites": r.cites,
    }))
}

struct Resolver<'a> {
    l: &'a Loaded,
    cache: HashMap<String, i64>,
    /// Every citation in the view's text, in order; notes point at them as `#cN`.
    cites: Vec<Value>,
}

impl Resolver<'_> {
    /// Citations in note text: `[label](ref)`, where ref is a code reference (as anywhere
    /// in a view), `file:line` / `file:start-end`, or `:line` / `:start-end`, a line of
    /// `here` (the thing the note is about). Each must resolve; the text comes back with
    /// `[label](#cN)` pointing into the view's `cites`.
    fn cite(&mut self, text: &str, here: Option<i64>) -> Result<String> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(open) = rest.find('[') {
            let after = &rest[open + 1..];
            let Some(close) = after.find("](") else { break };
            let label = &after[..close];
            let tail = &after[close + 2..];
            let Some(end) = tail.find(')') else { break };
            if label.contains('[') || label.is_empty() {
                out.push_str(&rest[..open + 1]);
                rest = after;
                continue;
            }
            let target = tail[..end].trim();
            let cite = self.cite_target(target, here).with_context(|| format!("citation `[{label}]({target})`"))?;
            out.push_str(&rest[..open]);
            out.push_str(&format!("[{label}](#c{})", self.cites.len()));
            self.cites.push(cite);
            rest = &tail[end + 1..];
        }
        out.push_str(rest);
        Ok(out)
    }

    fn cite_target(&mut self, target: &str, here: Option<i64>) -> Result<Value> {
        let lines = |s: &str| -> Option<(i64, i64)> {
            let (a, b) = s.split_once('-').unwrap_or((s, s));
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        };
        // `:52` or `:52-60`: lines of the thing the note is about.
        if let Some(rel) = target.strip_prefix(':') {
            let (a, b) = lines(rel).context("`:` is followed by a line or a range, like `:52` or `:52-60`")?;
            let id = here.context("`:line` needs something shown to count in: a `trace`, a `focus`, or the note's `on`")?;
            let (file, s, e): (Option<String>, Option<i64>, Option<i64>) =
                self.l.db.conn().query_row("SELECT file, line_start, line_end FROM main.nodes WHERE nid = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            let (Some(file), Some(s), Some(e)) = (file, s, e) else { bail!("what the note is about has no source lines") };
            anyhow::ensure!(a <= b && a >= s && b <= e, "lines {a}–{b} are outside lines {s}–{e} of what the note is about ({file})");
            return Ok(json!({"file": file, "a": a, "b": b}));
        }
        // `path/file.ext:52`: lines of a snapshot file.
        if let Some((file, rng)) = target.rsplit_once(':')
            && let Some((a, b)) = lines(rng)
        {
            let text = crate::window::file_text(self.l, file).with_context(|| format!("`{file}` is not a file of this snapshot (or the deny rules hide it)"))?;
            let n = text.lines().count() as i64;
            anyhow::ensure!(a >= 1 && a <= b && b <= n, "`{file}` has {n} lines");
            return Ok(json!({"file": file, "a": a, "b": b}));
        }
        let r = match target.parse::<i64>() {
            Ok(id) => Ref::Id(id),
            Err(_) => Ref::Name(target.to_string()),
        };
        Ok(json!({"node": self.one(&r)?}))
    }

    fn one(&mut self, r: &Ref) -> Result<i64> {
        match r {
            Ref::Id(id) => {
                let found = self.l.db.conn().query_row("SELECT 1 FROM main.nodes WHERE nid = ?1", [id], |_| Ok(())).is_ok();
                anyhow::ensure!(found, "no node has id {id}");
                Ok(*id)
            }
            Ref::Name(s) => {
                if let Some(id) = self.cache.get(s) {
                    return Ok(*id);
                }
                let id = insight_solver::path::resolve(self.l.db.conn(), s)?;
                self.cache.insert(s.clone(), id);
                Ok(id)
            }
            Ref::Path { .. } => bail!("a `path` search stands for a list; use it where a list goes (highlight, open, chain, order)"),
        }
    }

    /// A list; a `path` search adds every node on it.
    fn many(&mut self, refs: &[Ref]) -> Result<Vec<i64>> {
        let mut out = Vec::new();
        for r in refs {
            match r {
                Ref::Path { path } => out.extend(self.search(path)?.nodes.keys().copied()),
                _ => out.push(self.one(r)?),
            }
        }
        let mut seen = HashSet::new();
        out.retain(|x| seen.insert(*x));
        Ok(out)
    }

    /// A sequence; a `path` search adds one shortest route through it, in order.
    fn route(&mut self, refs: &[Ref]) -> Result<Vec<i64>> {
        let mut out = Vec::new();
        for r in refs {
            match r {
                Ref::Path { path } => out.extend(shortest(&self.search(path)?)),
                _ => out.push(self.one(r)?),
            }
        }
        out.dedup();
        Ok(out)
    }

    fn trace(&mut self, t: &TraceIn) -> Result<Value> {
        let stack = self.route(&t.stack)?;
        anyhow::ensure!(!stack.is_empty(), "a `trace` needs a `stack`: at least the function to show");
        Ok(json!({"stack": stack, "line": t.line}))
    }

    fn search(&self, p: &PathSpec) -> Result<PathResult> {
        let follow: Vec<&str> = p.follow.iter().map(String::as_str).collect();
        anyhow::ensure!(!follow.is_empty(), "a `path` needs `follow`: choose the hop kinds (ADR-0047)");
        let opts = Options { follow: Follow::parse_list(&follow.join(","))?, max_occurrences: p.max_occurrences, max_depth: p.max_depth, schemas: vec!["main".into()] };
        let r = insight_solver::path::path(self.l.db.conn(), &p.from, &p.to, &opts)?;
        anyhow::ensure!(!r.nodes.is_empty(), "the `path` search found no route");
        Ok(r)
    }
}

/// One shortest route from a start to a target, through the result's hops.
fn shortest(r: &PathResult) -> Vec<i64> {
    let mut next: HashMap<i64, Vec<i64>> = HashMap::new();
    for h in &r.hops {
        next.entry(h.from).or_default().push(h.to);
    }
    let goals: HashSet<i64> = r.targets.iter().copied().collect();
    let mut prev: HashMap<i64, Option<i64>> = r.starts.iter().map(|s| (*s, None)).collect();
    let mut q: VecDeque<i64> = r.starts.iter().copied().collect();
    let mut end = None;
    while let Some(x) = q.pop_front() {
        if goals.contains(&x) || (goals.is_empty() && next.get(&x).is_none_or(Vec::is_empty)) {
            end = Some(x);
            break;
        }
        for y in next.get(&x).into_iter().flatten() {
            if !prev.contains_key(y) {
                prev.insert(*y, Some(x));
                q.push_back(*y);
            }
        }
    }
    let Some(mut x) = end else { return r.nodes.keys().copied().collect() };
    let mut out = vec![x];
    while let Some(Some(p)) = prev.get(&x) {
        out.push(*p);
        x = *p;
    }
    out.reverse();
    out
}
