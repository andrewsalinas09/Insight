# ADR-0021: The home screen is the whole system plus a question box

- **Status:** Accepted, layout deferred with the UI (ADR-0027); amended by R2-09: the question box is the terminal, and the window offers every tool directly (two-way parity, ADR-0031). Kept: no curated landing page; automatic collapse rules removed by ADR-0047 (any initial view is Claude's `show` or the user's own navigation; counting hidden children is deferred to the frontend session)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 (Q-30: "honestly for the home screen maybe just show the whole thing. The idea is you ask what you want")

## Context
Q-30 asked what to show after loading a repo. Candidates were a trigger list (a table of contents), a container map, and suggested questions.

## Decision
After loading, show **the whole system** (the full graph at its top container level, with sizes, ADR-0008 clarification) and **a question box**. No curated landing page and no wizard. The interaction model is: you ask, and the graph answers (ADR-0010).

## Why
Insight is question-driven. The useful views are the ones an answer produces, so the home screen's job is to orient ("here's everything, roughly how big") and get out of the way. A curated landing page guesses what the user wants, and the question box already handles that.

## Consequences
- The top-level overview must stay readable at any repo size. Containers are collapsed with "+N hidden" (ADR-0013 rule 2), and size is visible.
- Indexing progress is shown on that same overview as it fills in. There's no separate loading screen.
- Trigger lists and suggested questions aren't the home screen, but they can be *answers* ("what can I click?").
