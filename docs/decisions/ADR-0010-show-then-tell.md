# ADR-0010: Show, then tell: every answer drives the graph as well as the prose

- **Status:** Accepted; amended by R2-07 (the "tell" usually lives in the terminal as Claude's plain text; the window is the visual guide you follow along with; sentence links are available in window notes and steps, never required or flagged). Claude can do whatever it wants
- **Date:** 2026-10-09
- **Source:** user, discovery session 5 ("when I asked why not AtomicBool, it could highlight instantly why on the app for me to spell it out + the prose… because the graph is navigable it is an _excellent_ companion")

## Context
In the learning log (`docs/research/2026-10-learning-session-patterns.md`), every answer was prose plus hand-drawn text diagrams. Insight already has a live, navigable graph of the same code. A text-only answer next to it wastes the graph.

## Decision
Every explanation is a **pair: a view state plus prose**, produced together.
- **Show**: the answer drives the graph through the tool API (ADR-0006). It focuses the relevant region, highlights the nodes and edges the answer depends on, dims the rest (with "+N hidden" markers), and steps through sequences where order matters.
- **Tell**: the prose explains it. **Each sentence links to the graph elements it relies on** (its receipts). Hovering a sentence highlights them; clicking a highlighted node scrolls to the sentence that discusses it.
- The user can navigate freely from the answer's view (expand, follow edges, ask the next question from there). The answer is a starting position, not a picture.

## Why
Seeing the check and the write as two separate highlighted steps, with two workers stepping through them, *is* the explanation of why one atomic flag doesn't help. The prose then only has to name it. Linking sentences to elements also enforces grounding: a sentence with nothing to highlight is a smell (ADR-0002). Rejected alternative: chat in a side panel that only references the graph by name. That leaves the user to do the mapping, which is exactly the work Insight exists to remove.

## Consequences
- Answers can show **scenarios**: hypothetical sequences such as "what if both workers read before either writes". These are a third overlay kind beside static and observed (ADR-0003), and they're always visibly marked *hypothetical*, never mistaken for what the code does (TB-065).
- Journal entries (Q-22) store the view state with the prose, so reopening an entry restores the picture.
- The explain-differently ladder (Q-23) gets visual angles: a different focus, a step-through, a side-by-side of the two alternatives.
