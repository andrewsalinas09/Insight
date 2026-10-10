# ADR-0020: Insight is open source under Apache-2.0

- **Status:** Accepted; the business-model half of Q-14 was closed by ADR-0025
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("this is apache 2.0")

## Context
Q-14 (distribution and business model) was open. Insight's sibling project Fletcher is Apache-2.0.

## Decision
Insight is licensed under the Apache License 2.0. The `LICENSE` file at the repo root is the unmodified standard text.

## Why
It matches the user's other projects. It's permissive with an explicit patent grant, which suits a tool other agents and companies should integrate with (ADR-0017's MCP surface), and it keeps plugin authors (ADR-0007, ADR-0016) unencumbered.

## Consequences
- Contributions are under Apache-2.0. Whether a CLA or DCO is needed can be decided when there are outside contributors.
- Q-14's business-model half (hosted features, pricing with bring-your-own LLM accounts) stays open.
