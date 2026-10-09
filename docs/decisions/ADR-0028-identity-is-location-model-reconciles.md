# ADR-0028: Node identity is the location, rebuilt every build; the model reconciles renames and moves

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 (Q-35: "we can use git. But this feels difficult. I'm guessing it should be a whole new thing and it should be recognized on build, and then a diff is shown for the LLM to reconcile or something. ANY time something is semantic it is the model's job to fix if it needs to. Also why does this even matter?")

## Context
Q-35 asked how a function keeps its identity across renames and moves. Claude proposed solver-side matching (by body similarity, git renames). The user asked what problem it solves. Honest answer: it only matters for things that **persist and point at code** (saved teaching views, journal entries, expectations) and for a **readable system diff** (a rename shouldn't look like delete + add). The MVP (ADR-0019, ADR-0025) has neither: questions are answered live and nothing is saved.

## Decision
- **Identity = location**, a deterministic path like `app/src/App.tsx › MainApp › graftNode` (anonymous functions named by position, ADR-0022). It's recomputed on every build. The solver does **no** rename or move matching.
- **On each build, the solver emits a structural diff** against the previous build (nodes and connections added or removed), using git as input for what changed.
- **The model reconciles when it needs to**: "`graftNode` → `graftOnto` is a rename, same body", re-pointing a saved view, collapsing a refactor in the system diff. That's semantic, so it's the model's job (ADR-0023).

## Why
Matching identities is a pile of heuristics (similarity thresholds, split and merged functions), which is exactly the edge-case catalogue ADR-0023 keeps out of the solver. Location IDs are trivial and exact; the model reads a diff and recognizes a rename easily. And since the MVP saves nothing, building matching now would solve a problem we don't have yet.

## Consequences
- Resolves Q-35. TB-076 (the pure refactor) is answered by the model reading the structural diff, not by the solver.
- Inline annotations (ADR-0009) need nothing: they travel with the code.
- When saved views or expectations arrive, broken references show up in the build diff for the model to re-point. That's designed then, not now.
