# ADR-0013: Code is truth; taking away is free but visible; adding needs a receipt

- **Status:** Accepted; re-prioritized by ADR-0024 (receipts remain so every highlight lands on real code; the detailed compiler mechanism is optional, built only if real use shows it's needed)
- **Date:** 2026-10-09
- **Source:** user, discovery sessions 3–6 (Q-21). It was restated by Claude, validated against `docs/examples/save-note-worked-example.md`, refined three times by the user, and accepted with the caveat "a lot of this is hand-wavy for now"
- **Supersedes:** ADR-0006

## Context
ADR-0006 allowed the LLM only to organize the graph, never to add to it. The user corrected that: the LLM needs real power, but everything it adds must be checked deterministically. The real risk isn't LLM mistakes. It's **doc-to-code drift**.

## Decision
1. **Code is the only source of truth.** Docs, annotations, labels, groupings, and explanations are views of the code and must be re-checkable against it at any time.
2. **Taking away is free, as long as it's visible.** Hiding, collapsing, grouping, and abstracting are allowed without checks, and every omission leaves a visible marker ("+11 more fields") so it can't create a false belief (TB-056).
3. **Adding needs a receipt.** The LLM (or a human) may add an edge or claim only by quoting the code that proves it. Receipts are code quotes, not line numbers, and may be chains across files. The compiler locates each quote in the current code and checks that it means what the claim says. No receipt means it isn't shown.
4. **Three trust levels, set by the checker:** *proven* (fully verified), *inferred* (pieces verified, connection not; drawn distinctly), *rejected* (never shown). The same claim can be proven by a strong language plugin and only inferred by a weak one.
5. **Graph shape follows code shape.** No invented nodes. The LLM may group real nodes, or suggest a refactor that makes a step a real function.
6. **Drift becomes a broken receipt.** On every change, receipts into the changed code are re-checked. Broken claims are hidden and re-anchored, and prose citing them is flagged stale.
7. **Never write down what the code already says.** Committed annotations hold only intent/why plus gap-filling claims. Structure is computed. Explanations are abundant but live in the explanation layer (Q-22), not in annotations.

## Why
It keeps ADR-0002 (never wrong) enforceable *without* crippling the LLM. Power comes with proof, subtraction is cheap, and drift is caught mechanically instead of trusted away. ADR-0006's "no adding at all" was too strict: real codebases have callbacks, dispatch, and framework magic that static analysis can't resolve, and the LLM is usually right about them. The inferred tier lets it help honestly. Rule 7 came from the user's objection that the first design would have documented every line.

## Consequences
- **This is a rule set, not a mechanism. It is unproven** (user: "a lot of this is hand-wavy for now"). The following are all undesigned:
  - how quotes are matched structurally
  - which claim kinds exist and how each is checked
  - how a chain receipt "closes"
  - how re-anchoring works
  - what "+N hidden" means for every view type

  It has only been exercised on paper, in two worked examples. A **Phase 1 spike** must make the compiler real on a small codebase before any of this is treated as solved (ROADMAP).
- If the spike shows a rule can't be implemented as stated, it is superseded by a new ADR. That's expected and not a failure.
- ADR-0006's tool-API idea survives: the LLM still acts through tools (group, collapse, annotate, compose views). The tools now include "add with receipt".
