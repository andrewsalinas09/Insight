// The glossary in everyday reading (ADR-0056). Hover any name in any code on screen and a
// card says what it is: its kind and place, the project glossary's terms for it (Claude's
// words, kept with the project), how it's declared and what's written above it. Notes can
// cite a term as [label](term:name); hovering that shows the term. Click a card to go there.
// Loaded before app.js; everything here runs only after the graph has loaded.
"use strict";
let TERMS = [];                 // [{ name, means, names: [{ path, id }] }]
let termsOf = new Map();        // node -> [term]
let termByName = new Map();     // lower-case name -> term
let codeRects = [];             // [node, x, y, w, h] of code drawn this frame
let cardTimer = null, cardFor = null;

async function loadGlossary() {
  try { TERMS = await invoke("glossary"); } catch { TERMS = []; }
  termsOf = new Map(); termByName = new Map();
  for (const t of TERMS) {
    termByName.set(t.name.toLowerCase(), t);
    for (const x of t.names) { const n = x.id != null ? byId.get(x.id) : null; if (n) (termsOf.get(n) ?? termsOf.set(n, []).get(n)).push(t); }
  }
  queueDetail();
}

// The identifier under the pointer in a code box, and what it names.
function codeWordAt(sx, sy) {
  for (let i = codeRects.length - 1; i >= 0; i--) {
    const [n, x, y, w, h] = codeRects[i];
    if (sx < x || sx > x + w || sy < y || sy > y + h) continue;
    const lh = LH * cam.k, top = y + 30 * cam.k, left = x + 12 * cam.k;
    const row = Math.floor((sy - top) / lh), line = n.a + row;
    const text = codeOf(n)[row];
    if (text == null) return null;
    const col = Math.floor((sx - left) / (CW * cam.k));
    const re = /[A-Za-z_][A-Za-z0-9_]*/g;
    let m;
    while ((m = re.exec(text))) if (col >= m.index && col < m.index + m[0].length) return { n, line, word: m[0] };
    return null;
  }
  return null;
}
function resolveWord(n, line, word) {
  // Something used on this very line with this name (the snapshot records where).
  for (const e of E) if (e.tgt && e.tgt.n === word && e.src?.f === n.f && e.sites.some((s) => s[0] === line)) return e.tgt;
  // Else the nearest declaration with this name: in this function, this file, anywhere.
  const f = fnAt(n.f, line);
  let best = null;
  for (const m of byId.values()) {
    if (m.n !== word || LIBK.has(m.k) || m === root) continue;
    const s = f && isUnder(m, f) ? 0 : m.f === n.f ? 1 : 2;
    if (!best || s < best.s) best = { m, s };
  }
  return best?.m ?? null;
}

// ---------- the card ----------
function cardHtml(m, terms) {
  const src = m.f && m.a && files.has(m.f);
  const above = src ? linesAbove(m) : [];
  const decl = src ? srcLines(m.f, m.a, Math.min(m.b, m.a + (STATEK.has(m.k) ? 2 : 1))) : [];
  return `<header><span class="chip">${esc(KIND[m.k] ?? m.k)}</span><b>${esc(m.n)}</b><span class="dim">${esc(ctxOf(m))}</span></header>` +
    terms.map((t) => `<div class="term"><b>${esc(t.name)}</b>: ${prose(t.means)}</div>`).join("") +
    (decl.length ? `<pre>${esc(dedent(decl).join("\n"))}</pre>` : "") +
    (above.length ? `<pre class="above">${esc(dedent(above).join("\n"))}</pre>` : "") +
    `<footer>${m.f ? esc(`${m.f}:${m.a}`) + " · " : ""}click to go there</footer>`;
}
function termCardHtml(t) {
  return `<header><span class="chip">glossary</span><b>${esc(t.name)}</b></header><div class="term">${prose(t.means)}</div>` +
    (t.names.length ? `<footer>${t.names.map((x) => esc(x.path.split(" › ").pop()) + (x.id == null ? " (stale)" : "")).join(", ")}${t.names.some((x) => x.id != null) ? " · click to go there" : ""}</footer>` : "");
}
function showCard(html, px, py, go) {
  const c = $("hovercard"), main = c.parentElement.getBoundingClientRect();
  c.innerHTML = html; c.hidden = false;
  c.onclick = go ? () => { hideCard(); go(); } : null;
  c.style.cursor = go ? "pointer" : "";
  const w = c.offsetWidth, h = c.offsetHeight;
  c.style.left = Math.max(8, Math.min(px + 14, main.width - w - 8)) + "px";
  c.style.top = Math.max(8, Math.min(py + 14, main.height - h - 60)) + "px";
}
function hideCard() { clearTimeout(cardTimer); cardFor = null; $("hovercard").hidden = true; }
// Called on every pointer move over the canvas; the card shows after a short rest.
function cardAt(sx, sy) {
  const hit = codeWordAt(sx, sy);
  const key = hit ? `${hit.n.id}:${hit.line}:${hit.word}` : null;
  if (key === cardFor) return;
  hideCard();
  if (!hit) return;
  cardFor = key;
  cardTimer = setTimeout(async () => {
    const m = resolveWord(hit.n, hit.line, hit.word);
    const t = termByName.get(hit.word.toLowerCase());
    if (!m && !t) return;
    if (m?.f) await ensureFile(m.f);
    if (cardFor !== key) return;
    if (m) showCard(cardHtml(m, [...new Set([...(termsOf.get(m) ?? []), ...(t ? [t] : [])])]), sx, sy, () => jumpTo(m));
    else showCard(termCardHtml(t), sx, sy, termGo(t));
  }, 280);
}
function termGo(t) {
  const n = t.names.map((x) => (x.id != null ? byId.get(x.id) : null)).find(Boolean);
  return n ? () => jumpTo(n) : null;
}
// A term citation in a note: its card above the text, and its code lit on screen.
function termCite(c, el) {
  const t = termByName.get(String(c.term).toLowerCase());
  if (!t) return null;
  const r = el.getBoundingClientRect(), main = $("hovercard").parentElement.getBoundingClientRect();
  showCard(termCardHtml(t), r.left - main.left, r.top - main.top - 180, termGo(t));
  return t.names.map((x) => (x.id != null ? byId.get(x.id) : null)).find(Boolean) ?? null;
}
