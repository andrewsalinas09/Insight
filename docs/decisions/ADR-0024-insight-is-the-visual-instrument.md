# ADR-0024: Insight is the visual instrument for a model that already understands the code

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("remember the model _wrote the entire codebase_, it knows everything. It should be able to annotate whenever it needs to be annotated. Our job is to make the UI and graph and tools it can use. As you saw from QA_NOTES it doesn't need that. It explains in text perfectly fine. But wouldn't it be cool to have a UI that I can visualize in my head and see how things are connected and trace and follow what it says and highlights. It's a tool for that.")

## Context
Over many sessions the design grew heavy machinery for *making the model trustworthy*: compilers, claim vocabularies, assurance inventories, verification tiers. The user's real learning log shows the model already explains code well in text. What's missing isn't understanding. It's a **picture you can hold and follow** while the model explains.

## Decision
Insight's job is three things:
1. **The graph**: the solver connects the real code completely (ADR-0022, ADR-0023).
2. **The UI**: render and navigate that graph (hierarchy, time, lanes) so a human can build a mental model.
3. **The tools**: an API the model uses to *drive* the UI. Highlight, focus, collapse, step through, annotate, and compose views, every one landing on real nodes (ADR-0010).

**Understanding is the model's.** It annotates when it decides annotation is needed (ADR-0005/0009 remain as capabilities, not obligations) and explains in its own words. Insight makes those words *visible and followable* on the real code.

## Why
That's the gap the user actually feels: text alone makes you hold the whole structure in your head, and a graph alone doesn't tell you what matters. Together, the model talking and the graph lighting up where it points, you can trace and follow. Building the best instrument rather than a second brain also keeps Insight useful as models improve.

## Consequences
- **Priority shifts to the UI and the tool API.** These are the product. The solver exists to feed them.
- **Receipts stay, for a simpler reason.** Every highlight must land on real code, so a model's reference ("the clamp in `chain_of`") resolves to an actual node. That's how you *see* when it's wrong. The heavier verification ideas (claim vocabularies, compiler tiers, assurance inventories, ADR-0013's detailed mechanism) are capabilities to build only if real use shows they're needed, not MVP work.
- The MVP (ADR-0019) reads as: the model explains drag-and-drop in Fletcher, and the graph lights up, focuses, and steps along with it.
- Fits ADR-0017: the model can be the user's own external agent (Claude Code, Codex) calling Insight's tools while it talks.
