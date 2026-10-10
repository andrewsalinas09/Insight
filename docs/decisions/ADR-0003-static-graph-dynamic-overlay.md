# ADR-0003: Hybrid tracing: the static graph is the map, dynamic runs are overlays

- **Status:** Accepted; the dynamic overlay is post-MVP (ADR-0015, ADR-0037)
- **Date:** 2026-10-09
- **Source:** user, discovery session 1 (Q-02: "Both. Static and then dynamic can show the path of _this_ point")

## Context
Static analysis finds every branch but can't always resolve dispatch, callbacks, or cross-language hops. Dynamic tracing is exact but only shows the branches one run actually took. The product needs "all branches" and also "what happened when I clicked *this*".

## Decision
Static analysis builds the full execution graph, which is the map. A dynamic run (a click, a command, or a captured moment, "this point") is recorded and drawn as a highlighted path over that map. Runtime observations can also *confirm* edges that static analysis could only mark as possible.

## Why
Each approach covers the other's blind spot. Static analysis alone can't say which branch ran or which implementation was dispatched. Dynamic analysis alone hides the `!valid` branch you didn't trigger, and showing that branch is the point of the product. It also fits ADR-0002: an edge observed at runtime is *true* (it happened), so dynamic data can only upgrade "possible" to "observed" and can never add something false.

## Consequences
- Two edge provenances: `static` (proven by analysis, or possible) and `observed` (seen at runtime). The UI distinguishes them.
- How a run is captured and how "this point" is defined is an open question (Q-17).
- Runtime capture needs per-language instrumentation, which makes Q-03 (language choice) more expensive.
