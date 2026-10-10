# ADR-0039: A fixed tool with a fixed style; free composition inside views; new capabilities go through the user

- **Status:** Accepted; tool list amended by ADR-0041 (+ `compare`) and R2-12 (+ `path`: algorithmic pathfinding that reports cycles and recursion); wishes are appended to the Wishes section of INSIGHT.md (ADR-0044); `path` takes `follow=[…]` chosen by Claude (ADR-0047)
- **Date:** 2026-10-09
- **Source:** user, BQ-27 ("I kind of want this to be a fixed tool where the style is fixed and not constantly changing. I could see myself simply having 'if there's something you wish Insight could do, please let the user know and they will add it and then push to main'. Like everything you have here is good. I more imagine saying 'Claude, show me the flow from source to consumer of this data' and it on the fly builds the hierarchy and what's displayed etc. Simply, the edges and nodes are defined with logical stuff, but what's added and removed from a view and what's grouped can be changed however. If Claude is like 'I want to be able to do X', it tells me, I see if I like it, and then add it")
- **Rejects:** Claude's proposal of hot-loaded "mods" (Claude-written view and tool code inside Insight)

## Decision
- **Insight is a fixed tool:** a fixed toolset and a fixed visual style (ADR-0027). It doesn't reshape itself at runtime. No mods, no Claude-written view code.
- **Inside a view, composition is free.** Within the fixed view vocabulary, Claude decides per question which nodes and edges appear, how they nest (its own hierarchy), what's grouped, what's collapsed or hidden, what's annotated, and the step order. The nodes and edges themselves are logical facts from the solver.
- **New capabilities go through the user.** When Claude wants Insight to do something it can't, it says so ("I wish Insight could X, because…"). The user decides; if yes, it's added to Insight's source and pushed to main like any feature. This rule is part of Insight's MCP instructions.
- **The MVP tool list is the one Claude proposed:** `status`, `load`/`unload`, `build`, `sql` (read-only, 200-row cap with "truncated, N total"), `source`, `show`, `selection`, `tag_coverage`.

## Why
A tool you rely on to check work fast must be stable, and you shouldn't have to relearn it each session. Freedom is needed in *what a view shows*, not in *what the tool is*. Routing capability requests through the user keeps the user in charge of the product while still letting the model's needs shape it over time.

## Consequences
- BQ-27 answered. BQ-28 (the row cap) and BQ-29 (finding things = SQL over `nodes`/`receipts`) follow from ADR-0038. BQ-30 (`source` from the snapshot), BQ-31 (`status` carries the loaded snapshots), and BQ-32 (MCP instructions document the schema, the workflow, and the wish rule) are covered by this list.
- The **fixed view vocabulary** is now the central design question (Round 6): it's exactly what limits or enables "build the hierarchy on the fly".
