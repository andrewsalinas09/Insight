# ADR-0037: Defer to the intelligence. Insight never restricts what the model shows; verification is a tool the model uses

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-38 ("no, any value is ok. We defer to the intelligence. We simply give it the tools to check and make sure it does what it wants. If it's trying to make a race condition, let it verify it does in fact race before showing us. If the intelligence is wrong, that simply means we need better models (we don't)") and BQ-47 ("claude -p is extra. I can simply have you test it yourself here. You make the calls and tests from here")
- **Amends:** ADR-0011 ("a value the LLM merely predicted is never displayed") and the gatekeeping parts of ADR-0013 and ADR-0026

## Decision
- **The model may show any value, example, rewrite or claim.** Insight doesn't gate model-authored content. ADR-0026's authored-vs-real styling remains available for the model to use, but isn't enforced.
- **Verification is a tool, not a gate.** Insight gives the model the means to check itself when it wants to: run code with chosen inputs, add log probes (ADR-0015), replay a race under a fixed interleaving (ADR-0011), query the graph exactly. A model that wants to show a race should use them to confirm it races, and that's the model's judgment, not a rule.
- **Insight's own facts stay exact.** The deterministic layer (graph, paths, receipts) is never wrong (ADR-0002). The line is: Insight never states something false; the model is responsible for what it says.
- **Testing Insight is done by Claude in a development session**, calling Insight's MCP tools and checking results directly, plus deterministic automated tests. There are no `claude -p` runs.

## Why
This is ADR-0035 applied to trust. Gatekeeping the model's output means writing judgment into the tool, which is the intelligence the tool shouldn't contain. If the model is wrong, the fix is a better model, not a tool that second-guesses it. Insight's job is to make checking easy.

## Consequences
- The execution and step-through tools (ADR-0011, Q-24) become self-check tools for the model, not a precondition for showing values. They're still post-MVP.
- View validation only checks that references resolve to real nodes. There's no content policing.
