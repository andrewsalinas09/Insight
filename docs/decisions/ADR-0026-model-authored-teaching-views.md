# ADR-0026: The model authors teaching views: annotations, regrouping, re-hierarchy, abstraction, simplified code

- **Status:** Accepted; amended by ADR-0037 and R2-02 (the window styles by element kind automatically; only elements that stand in for code must resolve to real code; notes and example data needn't point anywhere); vocabulary fixed by ADR-0040
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("I feel like annotations is huge. We need to let it change hierarchy / abstraction / grouping / smaller simpler code as needed for demonstration. Use the QA notes to see what I mean")

## Context
The user's real learning log (generalized in `docs/research/2026-10-learning-session-patterns.md`) shows the explanations constantly *reshaped* the code rather than only pointing at it:
- **Regrouping:** a state container's ~20 fields presented as four role tables: core model and services, feature state, caches, operation gates.
- **Re-hierarchy:** a data-focused nesting (container → engine → saved model / live run table) that skips code structure, later widened when the partial view misled.
- **Abstraction:** a worker thread's loop shown as six lines of pseudocode.
- **Simpler code:** "simplified teaching version, styling and the stop branch omitted", then the real code.
- **Expansions:** `?` → `match`, a method chain → named variables, an iterator chain → an explicit loop.
- **Illustrative tables and timelines:** a sample schedule table, and a two-worker interleaving.

ADR-0013 rule 5 ("no invented nodes") and ADR-0024's highlight-only tools can't express any of this.

## Decision
There are three layers:
1. **Fact layer (solver).** Complete, mechanical, never invented (ADR-0022, ADR-0023). ADR-0013 rule 5 applies **here only**.
2. **Teaching views (model-authored).** The model can freely:
   - **annotate** any node, edge, or group, at any length;
   - **group** real nodes under names it chooses ("caches", "operation gates");
   - **re-hierarchy**: present its own nesting (by data, by role, by time) instead of code containment;
   - **abstract**: replace a span of real nodes with one authored node ("worker loop"), with pseudocode;
   - **show simpler code**: excerpts with omissions named, expansions (`?` → `match`), and equivalent rewrites (chain → loop);
   - **add illustrative content**: tables, example values, timelines.
3. **UI.** Renders a teaching view over the fact layer.

**Two invariants keep views honest (ADR-0002):**
- **Everything authored is visibly authored**: a distinct style and a "teaching view" marker. Simplified or rewritten code is labeled *simplified* or *rewritten*, and illustrative values are labeled *example*.
- **Everything authored maps to real nodes.** Every group, abstraction, and code excerpt links to the real nodes and quotes it stands for, and one click expands it back to the real code. An authored element with no mapping is rejected.

## Why
Demonstration needs the freedom the log shows: the best explanations drop, merge, reorder, and rewrite. Real structure is the thing being explained, not the best way to explain it. Keeping the fact layer pure and the view layer free, with every authored element mapped back, gives both clarity and honesty. You can always ask "show me the real thing" and get it.

## Consequences
- **Amends ADR-0013 rule 5** (scoped to the fact layer) and **expands ADR-0024's tool list**: the tool API gains annotate, group, re-hierarchy, abstract, code excerpt / expansion / rewrite, and table / timeline, alongside focus, highlight, step, find, and clear.
- TB-050 (the "validation" node for inline logic) is now allowed as an authored abstraction mapped to the real lines. TB-063 (a "simplified equivalent" that isn't equivalent) stays a live risk, mitigated by the *rewritten* label and the one-click link to the real code.
- Q-19 (custom views) is largely answered: a custom view *is* a teaching view.
- ADR-0005/0009 committed annotations stay a separate, optional thing. Teaching views are explanations, not repo documentation; where they're stored is Q-37.
