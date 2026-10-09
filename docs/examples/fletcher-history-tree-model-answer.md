# Worked example: Claude answers as "the model", on Fletcher's history tree

The user's dogfood test (session 8): "*You* are the model. How does the tree get held in memory of the history? How do you modify that tree? How would you want to show me?" Traced from Fletcher `app/src/App.tsx` at commit `cd5d6ff`. This records the answer and, more importantly, **which views the model wanted to draw**. Those are requirements for the tool API (ADR-0024, ADR-0026).

## The answer, in brief
- **Held:** one mutable object in `MainApp`, `hist = useRef<{ nodes: Map<number, HistNode>; current; next }>`. Nodes are flat in a `Map`; the shape comes from `parent` + `children` stored on each node (both directions). Each node's `snap` is a full `ChainSnap` copy, not a delta. The root `0` ("start") can't be deleted. Companions: `rail` (the undo/redo track: ancestry, then latest-child descent) and `histVersion` (a counter bumped after every mutation, because React doesn't watch refs).
- **Modified by eight functions:** `commitGesture` (adds a child of current, no-op if unchanged), `graftNode` (adds a copy under any node), `moveTo` / `undo` / `redo` / `jumpTo` (move `current`, which changes the sound), `deleteNode` (prunes a subtree, moving current to the parent if it was inside), `editNode` (metadata only), `initHistory` / `installHistory` (replace everything; adds a "resumed" node if the live chain drifted). Undo then edit **branches**; it never discards.
- **Observations:**
  - The two-way `parent`/`children` link is a hand-maintained invariant. `installHistory` copies `children` from disk without checking it against `parent`.
  - Every mutator ends with `setHistVersion`, but nothing enforces it.

## The views the model wanted (four)
1. **Runtime data shape:** organized by data, not code. `hist` → `nodes` / `current` / `next`, beside `rail` and `histVersion`, plus an *example instance*: a small tree drawn from the Map, showing both link directions, with "full chain copy" on `snap`.
2. **Mutation matrix:** functions × state they write (`nodes`, `children`, `current`, `rail`, sound, `histVersion`). The checkmarks are solver facts (write connections). The row grouping (adds / moves / removes / edits / replaces) is authored.
3. **Before → after step-through** on example data: undo (9 → 5 along the rail), then edit, which creates a new branch, with node 9 still present.
4. **Unenforced rules:** every mutator's final `setHistVersion` highlighted, the parent/children update pairs highlighted, and `installHistory` noted as the place that trusts the file.

## What this taught the design
1. **Teaching views must draw runtime data, not just code.** Views 1 and 3 show *instances* (a Map holding a tree, with ids, links, and values) labeled *example*. A code graph alone can't show "this Map is a tree". → A new authored element kind, the **example instance diagram**, mapped to the type and the code that builds it (ADR-0026 extension; TB-106).
2. **The mutation matrix is the solver's most useful raw output here.** "Which functions write which state" was the backbone of the whole answer, and it falls straight out of ADR-0023 write connections plus field-level state (Q-20). The model only added the grouping.
3. **Before/after diagrams** are how mutations are best explained. Step-throughs (ADR-0011) need a "state snapshot" frame, not just "current node + note".
4. **Invariants are the payoff of the solver.** "All eight mutators bump the version" and "both link directions are updated" are checkable as *all writers of X also write Y*, a pure structural query (Q-34), which the model then explains.
