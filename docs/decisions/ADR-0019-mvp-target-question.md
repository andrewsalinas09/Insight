# ADR-0019: The MVP is defined by one question: "how does this algorithm work, and how do we know it's right?"

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("My goal is this: load the software, ask a question about an algo and how it works, the data structures it uses and where they're stored etc. 'How do we know that the frequency is correct?'")

## Context
The vision has 20 goals and no cut. Learning wins ties (ADR-0012), but "learning" alone doesn't say what to build first.

## Decision
The MVP is done when a user can **load a codebase and ask about an algorithm** and get a show-then-tell answer (ADR-0010) covering:
1. **How it works**: the flow through the algorithm, with explicit time (ADR-0018).
2. **The data structures it uses and where they live**: owned by which component or struct, whether they cross a boundary, whether they're persisted (Q-20, the Fletcher example).
3. **How we know it's correct**: an **assurance trace** (Q-31). Where the value is validated, defaulted, or clamped; which tests exercise it; what is *not* checked.

**Acceptance test:** Fletcher (TS + Rust, ADR-0016). Ask "how does a parametric filter turn into sound, and how do we know its frequency is correct?" Answers must match a hand-traced key, like the drag-graft example. A real excerpt of that key, traced 2026-10-09:
- validated by clamping: `fc_hz: f.fc_hz.clamp(10.0, 24000.0),` (`app/src-tauri/src/lib.rs`, `chain_of`)
- a silent default when missing: `fc_hz.unwrap_or(1000.0)` (several sites in `lib.rs`), so a missing frequency becomes 1 kHz without complaint, which the trace must surface
- DSP math tested: `fn peaking_hits_its_gain_at_center()`, `fn notch_kills_center_leaves_far_field()`, `fn time_domain_matches_frequency_response()` (`crates/fletcher-core/src/dsp.rs`)

**In the MVP:** static graph for TS + Rust + the Tauri bridge, effects, the data-structure facts above, explicit timing kinds, show-then-tell chat, the assurance trace, and receipts with the proven/inferred tiers.
**Not in the MVP:** step-throughs with values, oversight (system diff, expectations), wire links across devices, the learning journal, live mode. They all follow, in an order decided after the MVP.

## Why
The question bundles the three things a learner most needs (mechanism, data, trust) and exercises the core (deterministic graph, receipts, show-then-tell) without the expensive parts (running code, multi-repo). "How do we know it's correct" is also the bridge to oversight: the assurance trace is the read-only ancestor of expectations.

## Consequences
- The Phase 2 tracer bullet is this question on Fletcher.
- The hand-traced key for the full Fletcher question is the first item of the evaluation set (Q-33).
- Features outside the cut stay designed but unbuilt. Changing the cut means a new ADR.
