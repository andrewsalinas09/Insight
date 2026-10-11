// Details: everything the snapshot knows about the thing you're on, in a panel docked at
// the bottom or popped out to its own window (R2-10). It follows the trace's current
// stop (or what you hover there), or what you select on the map. Every name is a link
// that goes there, keeping the trace's trail. Facts only: what's written above it, how
// it starts, who calls, reads or writes it, what it calls, what it holds (ADR-0053).
// Loaded before app.js; everything here runs only after the graph has loaded.
"use strict";
let detailOn = null, detailPopped = false, detailTimer = null, detailLast = null;

function detailSubject() {
  if (TR?.lay) return traceHover?.target ?? traceHover?.caller ?? TR.pick ?? TR.S;
  return selected && selected !== root ? selected : null;
}
// Called whenever the bar changes; the panel catches up a moment later.
function queueDetail() { clearTimeout(detailTimer); detailTimer = setTimeout(() => refreshDetail(false), 90); }
async function refreshDetail(force) {
  const n = detailSubject();
  if (!force && n === detailLast) return;
  detailLast = n;
  if (!detailOn && !detailPopped) return;
  if (n?.f && !files.has(n.f)) await ensureFile(n.f);
  const d = n ? detailFor(n) : { kind: "", name: "nothing selected", where: "", html: `<p class="dim">Select something on the map, or step through a trace: its details show here.</p>` };
  if (detailOn) renderDetail($("detail"), d);
  if (detailPopped) window.__TAURI__.event.emit("detail", d).catch(() => {});
}
function renderDetail(el, d) {
  el.querySelector(".dkind").textContent = d.kind;
  el.querySelector(".dname").textContent = d.name;
  el.querySelector(".dwhere").textContent = d.where;
  el.querySelector(".dbody").innerHTML = d.html;
}

// ---------- the facts ----------
const go = (n, label, line) => `<a href="#" data-go="${n.id}"${line ? ` data-line="${line}"` : ""} title="${esc(pathText(n))}">${esc(label ?? n.n)}</a>`;
const srcLines = (f, a, b) => (files.get(f) ?? []).slice(a - 1, b).map((l) => l.replace(/\t/g, "    "));
const pre = (ls) => `<pre>${esc(dedent(ls).join("\n"))}</pre>`;
function dedent(ls) {
  const ind = Math.min(...ls.filter((l) => l.trim()).map((l) => l.length - l.trimStart().length));
  return ls.map((l) => l.slice(Number.isFinite(ind) ? ind : 0));
}
// What's written right above it, up to the first blank line or the previous thing in
// the same box: comments, attributes, annotations, whatever the language puts there.
function linesAbove(n) {
  const src = files.get(n.f);
  if (!src || !n.a) return [];
  const sib = (n.parent?.children ?? []).filter((x) => x.f === n.f && x.b && x.b < n.a).map((x) => x.b);
  const floor = Math.max(n.parent?.f === n.f && n.parent.a && n.parent.a < n.a ? n.parent.a : 0, ...sib);
  const out = [];
  for (let l = n.a - 1; l > floor && out.length < 12; l--) { const t = src[l - 1] ?? ""; if (!t.trim()) break; out.unshift(t); }
  return out;
}
function section(title, body, count) { return body ? `<section><h4>${esc(title)}${count != null ? ` <small>${count}</small>` : ""}</h4>${body}</section>` : ""; }
// A list of functions with lines, grouped by the top box they live in.
function grouped(list, max = 400) {
  if (!list.length) return "";
  const by = new Map();
  for (const x of list) { const top = x.n.path[1] ?? x.n; (by.get(top) ?? by.set(top, []).get(top)).push(x); }
  let shown = 0;
  return [...by].map(([top, xs]) => `<div class="dgrp"><div class="gh">${esc(short(top))} <small>${xs.length}</small></div><ul>${xs.map((x) => (shown++ < max ? `<li>${go(x.n)} <span class="dim">${esc(ctxOf(x.n))}</span>${x.lines?.length ? ` · ${x.lines.slice(0, 5).map((l) => go(x.n, "line " + l, l)).join(", ")}${x.lines.length > 5 ? "…" : ""}` : ""}${x.note ? ` <span class="dim">${esc(x.note)}</span>` : ""}</li>` : "")).join("")}</ul></div>`).join("");
}
function usesOf(F) {
  const out = [];
  for (const e of E) {
    if (!e.src || !e.tgt || !isUnder(e.src, F) || isUnder(e.tgt, F)) continue;
    out.push({ e, line: e.sites[0]?.[0] ?? 0 });
  }
  return out.sort((a, b) => a.line - b.line);
}
function detailFor(n) {
  const d = { kind: KIND[n.k] ?? n.k, name: n.n, where: [pathText(n), n.f ? `${n.f}:${n.a}${n.b && n.b !== n.a ? "–" + n.b : ""}` : ""].filter(Boolean).join(" · ") };
  let h = "";
  if (n.f && n.a && files.has(n.f)) {
    const above = linesAbove(n);
    if (above.length) h += section("written above it", pre(above));
    const decl = STATEK.has(n.k) ? srcLines(n.f, n.a, n.b) : srcLines(n.f, n.a, Math.min(n.b, n.a + 3));
    h += section(STATEK.has(n.k) ? "declared" : "starts", pre(decl));
  }
  if (n.k === "function") {
    const callers = callersOf(n);
    h += section("called by", grouped(callers) || `<p class="dim">nothing in the snapshot calls it</p>`, callers.length);
    const u = usesOf(n);
    const pick = (kinds) => { const m = new Map(); for (const { e, line } of u) if (kinds.includes(e.kind)) { const x = m.get(e.tgt) ?? m.set(e.tgt, { n: e.tgt, lines: [], note: LINE[e.kind].label }).get(e.tgt); if (line) x.lines.push(line); } return [...m.values()]; };
    const calls = pick(["call", "possible", "handoff"]), lib = pick(["lib"]), writes = pick(["write", "access"]), reads = pick(["read"]);
    h += section("calls, in the order written", grouped(calls), calls.length);
    h += section("writes", grouped(writes), writes.length);
    h += section("reads", grouped(reads), reads.length);
    h += section("library calls", lib.length ? `<p>${lib.map((x) => go(x.n, x.n.parent && x.n.parent.k !== "libroot" ? short(x.n.parent) + " › " + x.n.n : x.n.n)).join(", ")}</p>` : "", lib.length);
  } else if (isData(n)) {
    const { writers, readers } = dataUses(n);
    if (n.k === "type") {
      const fields = n.children.filter((c) => STATEK.has(c.k));
      if (fields.length) h += section("holds", `<ul>${fields.map((f) => `<li>${go(f)} <code>${esc((srcLines(f.f, f.a, f.a)[0] ?? "").trim())}</code></li>`).join("")}</ul>`, fields.length);
      const fns = n.children.filter((c) => c.k === "function");
      if (fns.length) h += section("functions inside", `<p>${fns.map((f) => go(f)).join(", ")}</p>`, fns.length);
    } else if (n.parent && n.parent !== root) h += section("inside", `<p>${go(n.parent, prefix(n.parent) + n.parent.n)}</p>`);
    h += section("written by", grouped(writers) || `<p class="dim">nothing in the snapshot writes it</p>`, writers.length);
    h += section("read by", grouped(readers) || `<p class="dim">nothing in the snapshot reads it</p>`, readers.length);
  } else if (LIBK.has(n.k)) {
    const from = new Map();
    for (const e of E) if (e.kind === "lib" && (e.tgt === n || isUnder(e.tgt, n))) { const f = fnOf(e.src); if (f) { const x = from.get(f) ?? from.set(f, { n: f, lines: [] }).get(f); for (const s of e.sites) x.lines.push(s[0]); } }
    h += section("called from your code", grouped([...from.values()]), from.size);
  } else {
    const kids = n.children.filter((c) => !LIBK.has(c.k));
    h += section("contains", `<p>${esc(sizeText(n))}</p><p>${kids.slice(0, 80).map((c) => go(c, prefix(c) + short(c))).join(", ")}${kids.length > 80 ? "…" : ""}</p>`);
  }
  return { ...d, html: h };
}

// ---------- docked and popped out ----------
function showDetail(on) {
  detailOn = on;
  try { localStorage.setItem("insight-detail", on ? "1" : "0"); } catch {}
  $("detail").hidden = !on;
  document.body.classList.toggle("docked", on);
  $("detBtn").setAttribute("aria-pressed", String(on || detailPopped));
  if (on) refreshDetail(true);
}
async function popOut() {
  const { WebviewWindow } = window.__TAURI__.webviewWindow;
  const had = await WebviewWindow.getByLabel("detail");
  if (had) { had.setFocus(); return; }
  const w = new WebviewWindow("detail", { url: "detail.html", title: "Insight details", width: 620, height: 860 });
  w.once("tauri://destroyed", () => { detailPopped = false; showDetail(true); });
  detailPopped = true; detailOn = false; $("detail").hidden = true; document.body.classList.remove("docked");
}
function onDetailClick(e) {
  const a = e.target.closest("a[data-go]");
  if (!a) return;
  e.preventDefault();
  jumpTo(byId.get(+a.dataset.go), a.dataset.line ? +a.dataset.line : null);
}
function bindDetail() {
  $("detail").addEventListener("click", onDetailClick);
  $("detBtn").onclick = () => (detailPopped ? popOut() : showDetail(!detailOn));
  $("dPop").onclick = popOut;
  $("dHide").onclick = () => showDetail(false);
  const ev = window.__TAURI__.event;
  ev.listen("detail-ready", () => { detailPopped = true; $("detBtn").setAttribute("aria-pressed", "true"); refreshDetail(true); });
  ev.listen("detail-closed", () => { detailPopped = false; showDetail(true); });
  ev.listen("detail-go", (e) => jumpTo(byId.get(e.payload.id), e.payload.line ?? null));
  let on = true;
  try { on = localStorage.getItem("insight-detail") !== "0"; } catch {}
  showDetail(on);
}
