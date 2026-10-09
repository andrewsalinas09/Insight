# ADR-0008: Bubbles are code's own containers; control edges are calls, branches, returns, throws

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 3 (Q-04, partial: "structs classes functions blocks packages files, they all feel logical… control yes calls branches returns throws")

## Context
The graph needs a node model that works for every language (ADR-0007) and gives a hierarchy without inventing one.

## Decision
**Containers (bubbles):** package › file › type (struct/class/enum/interface/trait) › function › block. Each language plugin maps its constructs onto these. For example, Rust `impl` methods belong to their type, and a JS module is a file. Containers nest, so the code's own nesting *is* the base hierarchy.

**Control edges:** call, branch (one edge per outcome: `valid` / `!valid`), return, throw.

Data and boundary edge families are not decided yet (Q-20).

## Why
These are the units programmers already think in, and every mainstream language has some form of them. Using the code's own containment means the base hierarchy is deterministic and therefore never wrong (ADR-0002). The LLM's groupings sit on top of it (ADR-0006) and never replace it.

## Consequences
- "Block" needs a definition: probably only blocks that matter for control flow (branch arms, loop bodies, try/catch), not every `{}` (Q-04).
- Some constructs don't fit cleanly (closures, Rust traits with default methods, JS prototypes, Python modules acting as singletons). Each plugin documents its mapping, and odd cases go to TESTBED.
- A type bubble holds fields as well as methods, which is what Q-20's data edges will point into.
- Clarification (2026-10-09, user, from the Fletcher example): **every container level always exists, including files in a monolith.** Even a 350 KB single-file UI is a real boundary and the user's map of where to look. How big a container is (how much it holds) should be visible, because "this should be split up" is itself an insight. Containers nest deeper inside it (closures in components); they never replace it.
