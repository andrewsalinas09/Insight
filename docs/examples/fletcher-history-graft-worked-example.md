# Worked example (real code): "How does Fletcher's history graph work, especially drag and drop?"

The first example on a **real** codebase: the user's open-source (Apache-2.0) Fletcher app, `C:\Users\andre\RustroverProjects\Fletcher`, which is a Tauri 2 + React app. It's a good test subject because it's vibe-coded at scale: `app/src/App.tsx` is ~350 KB (the whole UI in one file) and `app/src-tauri/src/lib.rs` is ~180 KB.

Everything below was traced by hand from the source on 2026-10-09 (Fletcher commit `cd5d6ff`). The quotes are real and act as **receipts** (ADR-0013). Line numbers are given for the reader's convenience; Insight itself would anchor by quote. The ASCII views show what Insight should produce. The tracing is real; the rendering is illustrative.

The question, as the user would ask it:

> "How does the history graph work, specifically the drag and drop? Separate the data structures for UI and what not."

---

## Answer part 1: the data structures, separated by layer (show, ADR-0010)

Insight's first move is a **custom view** (Q-19) that groups every type and piece of state the answer touches by what it's *for*. It uses grouping, not invented nodes (ADR-0013 rule 5).

```
┌─ DOMAIN: the undo tree itself (main window only) ─────────────────────────────────────┐
│  hist : useRef<{ nodes: Map<number, HistNode>; current; next }>        App.tsx:4775   │
│     └─ HistNode { id, parent, children[], snap: ChainSnap, label, ts, note?, pinned? }│
│  rail : useRef<number[]>   — the fixed undo/redo track through the tree    :4849      │
│  histVersion : useState(0) — a counter; bumping it is how React learns     :4776      │
│                              the (mutated-in-place) tree changed                      │
├─ SOUND: what a node actually *is* ────────────────────────────────────────────────────┤
│  ChainSnap = { enabled, kind, fcHz, gainDb, q }[]  — a full EQ snapshot     :157      │
│     every node stores the WHOLE chain, not a delta ("nodes are full snapshots")       │
├─ TRANSPORT: copies made for other places ─────────────────────────────────────────────┤
│  treeData : useMemo<HistTreeData>  — plain copy, field by field            :5174      │
│     ─► emit("hist-sync")      to the pop-out window                         :5198     │
│  serializeHistory() ─► JSON   ─► invoke("history_save") ─► disk            :5041/5126 │
│  HistTreeNode / HistTreeData  — the type the canvas receives               :158–168   │
├─ UI-ONLY: lives and dies inside the HistoryTree canvas ───────────────────────────────┤
│  dragStart : useRef<{ id, sx, sy }>     — where the press began             :488      │
│  ghost     : useState<{ id, x, y, target }> — the dragged circle + target   :490      │
│  suppressClick : useRef(false)          — eats the click that ends a drag   :489      │
│  svgElRef, pos (layout), zoom, pan      — geometry only                               │
└───────────────────────────────────────────────────────────────────────────────────────┘
  +4 more UI-only fields hidden (armed, selId, baseSel, pickMode)   ← ADR-0013 rule 2
```

**Tell:** The tree's truth is one mutable object in the main window, `hist`. Nobody else owns a copy that matters. The canvas, the pop-out, and the saved file all get *copies* made on each change. The drag-and-drop state (`dragStart`, `ghost`, `suppressClick`) never leaves the canvas component: it's pure gesture bookkeeping, and the domain only hears about a drag when it's over and successful.

## Answer part 2: the drag, as a step-through (ADR-0011)

Scenario set up by the AI: in the tree `#4 → #5 → #6 → #7`, drag `#7` onto its great-grandparent `#4`. **The ids are illustrative.** In a real step-through they'd come from an executed harness; `graftNode` is nearly pure except for `applySnap`, whose Rust call would be stubbed as *assumed*.

| Step | Where | What happens | Receipt |
|---|---|---|---|
| 1 | `nodeDown(7)` | Left button on a node (not a delete glyph): remember the start point, capture the pointer | `dragStart.current = { id, sx: e.clientX, sy: e.clientY };` |
| 2 | `nodeMove(7)` | Moves under 6 px are ignored (still a possible click) | `Math.hypot(e.clientX - d.sx, e.clientY - d.sy) < 6) return;` |
| 3 | `dropTarget` | Screen → SVG coordinates (divide by zoom), then the nearest *other* node within 48 units is the target | `let best = 48;` |
| 4 | render | `ghost` set, so a small circle follows the pointer and a ring marks `#4` | `className="graft-target"` |
| 5 | `nodeUp(7)` | Clear the gesture state, then **re-decide the target from the up event**, not from the ghost | `// Decide from the UP event itself — never from possibly-stale ghost state.` |
| 6 | `nodeUp(7)` | Mark the click to be swallowed, then call `onGraft(7, 4)` | `suppressClick.current = true;` |
| 7 | **fork: which window?** | Main window: `onGraft={graftNode}` directly. Pop-out window: the event `hist-cmd` `{type:"graft", id, base: onto}` goes to the main window, whose bus calls `graftNode(p.id, p.base)` | `onGraft={(id, onto) => emitTo("main", "hist-cmd", { type: "graft", id, base: onto })}` |
| 8 | `graftNode(7, 4)` | Walk up from `#7`'s parent: #6 (1), #5 (2), #4 (3) → `d = 3` | `if (c === ontoId) {` |
| 9 | `graftNode` | New node under `#4` with a **deep copy** of `#7`'s snapshot, labeled "3 edits in one" | ``label: d ? `${d} edit${d === 1 ? "" : "s"} in one` : `from #${src.id}`,`` |
| 10 | `graftNode` | It becomes current, so **`applySnap(node.snap)` makes the sound change** | `applySnap(node.snap);` |
| 11 | `applySnap` → `pushChain` | → `invoke("set_fletcher_chain")` → Rust renders and atomically writes `fletcher.txt` into Equalizer APO's config | `fsx::write_atomic(&install.config_path.join("fletcher.txt"), &text)` |
| 12 | `setHistVersion(v+1)` | Fan-out: `treeData` recomputed → `hist-sync` to the pop-out; 400 ms later `history_save` writes the tree JSON; `diff-sync` refreshes the compare view | `invoke("history_save", { preset: presets.active ?? "chain", data })` |

Values a learner would see at step 9 (illustrative): `d = 3`, `node.id = h.next` (e.g. 12), `node.parent = 4`, `label = "3 edits in one"`. The original `#5–#7` branch is untouched, which matches the code comment "history never lies".

```
 GRAPH at step 7 (the fork), both wirings drawn, the active one lit:
   [canvas node #7] ─pointerup─► nodeUp ─► onGraft ═══════════════════════════► graftNode   (main window)
                                                 ╲
                                                  ─ ─► emitTo("main","hist-cmd") ─link─► bus.cmd ─► graftNode   (pop-out)
```

## Answer part 3: things a learner wouldn't guess, and an overseer should care about

Each of these is something Insight can find **deterministically** (they're fact-layer findings, not opinions). Each one maps onto a feature we've designed.

1. **A drag-and-drop changes what you hear, system-wide.** The path from `nodeUp` reaches an **effect**: a file write into Equalizer APO's config directory (step 11), plus a preset save. Nothing in the canvas code hints at this; it's eight hops away. → Effects (Q-20); "what does this gesture actually *do*?"
2. **One component, two wirings.** `HistoryTree` gets `onGraft` from two different mount sites: a direct function in the main window, and an IPC event in the pop-out. Fletcher's own CLAUDE.md has a rule for exactly this (rule 10, "Every feature ships to the satellites too… props actually passed in `PopoutHistory`… a `hist-cmd` payload + main-listener case"). The author had to write a *process rule* because the two wirings were invisible. Insight draws both, so a missing pop-out case would be a visible missing edge. → TB-081, sibling comparison (Q-25).
3. **The same shape is declared twice.** `HistNode` (inside the main component) and `HistTreeNode` (module level) have identical fields, and `treeData` copies them **field by field** (`label: n.label, ts: n.ts, snap: n.snap, note: n.note, pinned: n.pinned,`). Add a field to `HistNode` and forget the copy, and the pop-out and canvas silently never see it. → TB-082; a good standing expectation (Q-26): "every HistNode field reaches HistTreeNode".
4. **Saving the tree fails silently.** `invoke("history_save", …).catch(() => {})`. If the write fails, neither the user nor a logger hears about it. That's the user's "you forgot to feed the error to the logger" story, found in real code. → TB-085, error reachability (Q-25).
5. **The tree is mutated in place; a counter tells React.** `hist` is a `useRef` (React doesn't watch it), so every mutation must be followed by `setHistVersion(v => v + 1)`. That's a pattern a React learner will trip over, and a rule an agent can break (mutate, forget to bump, and the UI goes stale). → TB-084; explanation-ladder candidate (Q-23); a standing expectation: "every write to `hist` is followed by a version bump".
6. **A name doing double duty.** The pop-out sends the drop target as `base` (`{ type: "graft", id, base: onto }`), a field other commands use to mean "compare base". It works, but a reader of the bus code sees `graftNode(p.id, p.base)` and has to work out that `base` means "onto" here. → Prose observation, labeled as such.

## Likely follow-up questions (Insight supplies the next one, per the learning log)

- "Can a graft create a cycle, e.g. dragging a node onto its own descendant?" → **No, and it's provable from the code.** The graft always creates a *new* node with a fresh id (`id: h.next++`) under the target and never moves `src`, so no existing edge changes. (This is a nice *proven* claim, ADR-0013 tier 1.)
- "Why does `nodeUp` recompute the target instead of using `ghost.target`?" → React state updates are async. `ghost` may lag the final pointer position, and the comment says exactly that. Good for a "why" explanation plus the scenario where a stale ghost would drop on the wrong node.
- "What makes a click not count as a drag?" → The 6 px threshold, in both `nodeMove` and `nodeUp`.

---

## What this example taught the design

1. **Files always stay containers, even monoliths, and a monolith's size is itself information.** (Corrected by the user. A first draft called the file "useless" here.) A file is a clear logical boundary. It's how you remember *where to go* ("history lives in App.tsx and lib.rs"). And a 350 KB file being visibly huge is a *finding*: it shows where splitting would help, which matters to both the learner and the overseer. Inside it, the plugin must still go deep: components, hooks, and closures nested in one component (`graftNode` is a `const` inside `MainApp`) are containers too (ADR-0008). → TB-086.
2. **"Separate the data structures for UI and what not" is a first-class question** with a deterministic core. Classify each piece of state by where it lives (component-local ref/state vs. top-level domain), whether it crosses a boundary (emit, invoke, serialize), and whether it's persisted. Labels like "domain" and "UI-only" are organization (prose), but the facts under them (lives in `HistoryTree`, never emitted, never saved) are provable. → Q-20 data edges: *crosses-boundary* and *persisted* are needed edge kinds.
3. **Window identity is a branch.** `IS_HISTORY_WINDOW` is decided by a URL query parameter, so the same bundle has different personalities. That's config-driven branching (TB-011) in real code, and it decides which `onGraft` wiring exists.
4. **Effects at the end of UI gestures are the headline for both purposes.** A learner is surprised; an overseer wants to know. Effects belong in v1 (Q-20).
5. **Fletcher is a strong tracer-bullet candidate** (Phase 2): real, open, Tauri, the user's own, a monolith, and it already has a docs system Insight can read for the *why*.
