// Condensed views (ADR-0040): a view can draw only a subset of the graph (`nodes`), in
// Claude's own named boxes (`groups`), with some boxes folded (`collapse`). What's left
// out is never silent: every box that lost children says "+N hidden", and
// double-clicking it brings them back (for now; the next view starts fresh).
// "Whole system" leaves the subset. Loaded before app.js; runs after the graph loads.
"use strict";
let SUB = null;          // { chosen, groups, groupOf, kids, drawn, hidden, collapsed }
let showAll = false;     // "Whole system" pressed: ignore the view's subset
const revealed = new Set(); // boxes the user opened up again

function computeSub() {
  const v = V;
  if (!v || showAll || !((v.nodes ?? []).length || (v.groups ?? []).length)) { SUB = null; return; }
  const chosen = new Set(viewNodes(v.nodes));
  const groups = [], groupOf = new Map();
  (function make(list, parent) {
    for (const g of list ?? []) {
      const o = { id: "g" + groups.length, k: "group", n: g.name, children: [], parent: parent ?? root };
      o.path = (parent ? parent.path : [root]).concat(o);
      o.depth = o.path.length - 1;
      groups.push(o);
      if (parent) parent.children.push(o);
      for (const m of viewNodes(g.members)) { if (!groupOf.has(m)) { o.children.push(m); groupOf.set(m, o); } chosen.add(m); }
      make(g.groups, o);
    }
  })(v.groups, null);
  for (const r of revealed) if (chosen.has(r) || [...chosen].some((c) => isUnder(c, r))) for (const c of r.children) if (!LIBK.has(c.k) || LIBK.has(r.k)) chosen.add(c);
  const kids = new Map(), drawn = new Set([root]);
  const add = (p, c) => { const l = kids.get(p) ?? kids.set(p, []).get(p); if (!l.includes(c)) l.push(c); };
  for (const g of groups) { drawn.add(g); add(g.parent, g); }
  for (const n of chosen) {
    drawn.add(n);
    const g = groupOf.get(n);
    if (g) { add(g, n); continue; }
    // Ungrouped: it sits in its real boxes, which hold only what's chosen.
    let child = n;
    for (let d = n.path.length - 2; d >= 0; d--) {
      const a = n.path[d];
      add(a, child);
      if (drawn.has(a)) break;
      drawn.add(a);
      child = a;
    }
  }
  // Real boxes keep their own order (kind, then position in the file).
  for (const [p, l] of kids) if (p.k !== "group") l.sort((a, b) => (ORDER[a.k] ?? 6) - (ORDER[b.k] ?? 6) || (a.a ?? 0) - (b.a ?? 0) || a.n.localeCompare(b.n));
  const hidden = new Map();
  for (const [p, l] of kids) if (p !== root && p.k !== "group") { const n = p.children.length - l.length; if (n > 0) hidden.set(p, n); }
  const collapsed = new Set();
  for (const c of v.collapse ?? []) {
    if (c.group != null) { const g = groups.find((x) => x.n === c.group); if (g) collapsed.add(g); }
    else { const n = byId.get(c.node); if (n) collapsed.add(n); }
  }
  SUB = { chosen, groups, groupOf, kids, drawn, hidden, collapsed };
}
// Where something sits in this view: its parent box as drawn (a group, else its real box).
const subParent = (x) => SUB.groupOf.get(x) ?? x.parent;
// The innermost drawn box holding both: where ELK expects a line between them (it gives
// a line's coordinates relative to that box).
function subLca(a, b) {
  const up = new Set();
  for (let x = a; x; x = x === root ? null : subParent(x)) up.add(x);
  for (let x = b; x; x = x === root ? null : subParent(x)) if (up.has(x) && x !== a && x !== b) return x;
  return root;
}
function liftCollapsed(r) {
  let top = r;
  for (let x = r; x && x !== root; x = subParent(x)) if (SUB.collapsed.has(x)) top = x;
  return top;
}
// The box that stands for a node: the deepest drawn box holding it.
function subRep(n) {
  for (let d = n.path.length - 1; d >= 1; d--) if (SUB.drawn.has(n.path[d])) return liftCollapsed(n.path[d]);
  return root;
}
// Lines run only between chosen things: inside a chosen function counts as the function;
// a box that merely holds chosen things doesn't collect everyone else's lines.
function subRepEdge(n) {
  for (let d = n.path.length - 1; d >= 1; d--) if (SUB.chosen.has(n.path[d])) return liftCollapsed(n.path[d]);
  return null;
}
// Double-click in a condensed view: fold or unfold a group; open a box's hidden contents.
function subToggle(n) {
  if (SUB.collapsed.has(n)) { SUB.collapsed.delete(n); relayout(n); return true; }
  if (n.k === "group") { SUB.collapsed.add(n); relayout(n); return true; }
  if (SUB.hidden.get(n) || (SUB.chosen.has(n) && !isCode(n) && n.children.length && !SUB.kids.has(n))) {
    revealed.add(n);
    const keep = SUB.collapsed;
    computeSub();
    SUB.collapsed = keep;
    relayout(n);
    return true;
  }
  return false;
}
