# ADR-0002: The prime directive: the graph may be incomplete, never wrong

- **Status:** Accepted; scope narrowed by ADR-0041 (covers Insight's own deterministic facts; what Claude says is interpretation, shown as such)
- **Date:** 2026-10-09
- **Source:** user, discovery session 1 (Q-01; borrowed from Humanity Tech Tree ADR-0015)

## Context
Insight shows people what their code does, and many of them didn't write that code and can't easily check it. If the graph shows a single fabricated edge or a confidently wrong label, users stop trusting all of it. LLMs are central to the product and they hallucinate.

## Decision
Every edge, node, and label Insight shows must be either true or explicitly marked as uncertain. Missing information is always acceptable; false information must be made impossible wherever the system can enforce it. Code analysis owns the structure. LLM output is never shown as fact unless it has been checked against the code (see Q-15), and anything that can't be checked is labeled as interpretation and linked to its source.

Litmus test for any mechanism: "Can this ever *show* something false, or can it only leave something out?"

## Why
In a learning tool, trust is the whole product. A gap prompts the user to ask a question, which is what learning looks like. A falsehood gets learned. Rejected alternative: letting the LLM fill gaps in the static graph (for example, guessing which implementation a `dyn Trait` call reaches). That would make the graph more complete but less trustworthy, and the user couldn't tell which edges were real.

## Consequences
- An ambiguous call (TB-002, TB-006) is drawn as several *possible* edges, each marked possible, and never collapsed to the LLM's guess.
- Every label links back to source spans and/or verified claims.
- Every TESTBED case gets an implicit second check: does the failure mode produce wrongness or only incompleteness?
- A verifier ("compiler") for LLM output becomes a core component (Q-15).
