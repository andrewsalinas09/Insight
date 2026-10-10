# ADR-0047: Claude chooses at query time. Tools take parameters, not defaults; the solver keeps every fact unfiltered

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R3-13 ("we need a way to simply let Claude answer these questions on the fly in whatever way it needs / wants to. Remember Claude can 100%… know what counts / what is important. Our job is to give it the context as cheaply and easily as possible")

## Context
Claude kept asking the user to choose *defaults*: which edges `path` follows, which string literals are "noise" (BQ-22's 25-occurrence cutoff). Those are relevance judgments the model makes per question. Fixing them in advance writes intelligence into the tool (ADR-0035) and wastes the user's time.

## Decision
- **Query tools take parameters; Claude chooses them per question.** For example `path(from, to, follow=[call, possible, link, dispatch, literal, handoff, state, …])`. Every hop in a result is labeled with its kind. There are no product-level "default" filters to decide.
- **The solver records every fact and filters nothing.** The BQ-22 hub cutoff (more than 25 occurrences or shorter than 3 characters) is removed: every literal occurrence is linked, and the occurrence count is itself a fact Claude can use to ignore noise like `"main"`.
- **Insight's job is making context cheap:** fast queries, compact results, clear labels, counts and totals so Claude can decide what to fetch next.
- **Process rule for design sessions:** don't ask the user to choose runtime relevance (what to follow, what's important, what to show). Make it a parameter and let Claude decide when answering.

## Why
ADR-0035 applied to tool design: the model knows what matters for each question; a default can only be wrong for some of them. Parameters cost nothing and keep every question answerable.

## Consequences
- R3-13 answered. BQ-22's cutoff is superseded; TB-147's fan-out is handled by Claude choosing whether to follow `literal` hops.
- Any remaining "which default?" design question is resolved the same way: make it a parameter.
