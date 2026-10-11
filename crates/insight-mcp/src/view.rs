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
    let mut r = Resolver { l, cache: HashMap::new() };
    let mut notes = Vec::new();
    for n in &v.notes {
        notes.push(json!({"on": n.on.as_ref().map(|x| r.one(x)).transpose()?, "text": n.text}));
    }
    let mut steps = Vec::new();
    for (i, s) in v.steps.iter().enumerate() {
        let step = (|| -> Result<Value> {
            Ok(json!({
                "focus": s.focus.as_ref().map(|x| r.one(x)).transpose()?,
                "highlight": r.many(&s.highlight)?,
                "open": r.many(&s.open)?,
                "note": s.note,
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
        "focus": v.focus.as_ref().map(|x| r.one(x)).transpose()?,
        "notes": notes,
        "steps": steps,
        "play": v.play,
        "layout": {"chain": chain, "order": r.many(&v.layout.order)?, "options": options},
    }))
}

struct Resolver<'a> {
    l: &'a Loaded,
    cache: HashMap<String, i64>,
}

impl Resolver<'_> {
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
