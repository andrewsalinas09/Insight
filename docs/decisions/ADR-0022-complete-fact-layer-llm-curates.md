# ADR-0022: The fact layer is complete (every function is a bubble); the LLM curates what you see

- **Status:** Accepted; "default views must collapse aggressively" removed by ADR-0047 (no default view policy; Claude chooses collapse per view)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("every function, and it's the LLM's job to figure out what to show you. Remember this is mainly LLM first and we have a really really solid back end")

## Context
In real TS code like Fletcher's `App.tsx`, functions come in three kinds: named closures (`const graftNode = …`), anonymous JSX handlers (`onPointerDown={(e) => …}`), and anonymous callbacks (`rows.map((r) => …)`). We had to decide which ones become bubbles.

## Decision
**Every function is a bubble in the fact layer**, named or anonymous, including `.map`/`.filter` callbacks and inline handlers. Anonymous ones get a deterministic name from their position (e.g. `HistoryTree › <g>.onPointerDown`, `HistoryTree › rows.map#2`). **What is shown is the LLM's job**: views, lenses, and answers select and collapse (ADR-0010, ADR-0013 rule 2). Completeness lives in the backend; curation lives in the LLM.

## Why
"LLM first, really solid backend." A fact layer that drops anonymous functions would be incomplete in exactly the places where vibe-coded UI logic hides (inline handlers), and the LLM can't show what the backend never recorded. Deciding granularity at index time hard-codes one presentation. Deciding it at answer time lets every question get its own level of detail.

## Consequences
- The graph is large, so performance and storage (Q-12, Q-08) must assume every function is a node.
- Deterministic naming of anonymous functions is part of node identity (Q-35).
- Default views must collapse aggressively, with "+N hidden" (ADR-0013 rule 2), since raw completeness is unreadable.
- Supersedes nothing. It refines ADR-0008's "function" container to mean *every* function.
