# ADR-0009: Annotations are inline in source first; sidecar files are optional

- **Status:** Accepted; format settled by ADR-0043 (`@insight <keyword>: <value>` parsed mechanically; `tag` first; free text is read by Claude, not parsed); re-prioritized by ADR-0024 (a capability, not an obligation)
- **Date:** 2026-10-09
- **Source:** user, discovery session 3 ("primary inline, but also separate if you want / need")

## Context
ADR-0005 put annotations in the repo but left open whether they go inline as doc comments or in sidecar files.

## Decision
Annotations go **inline by default**, as structured doc comments next to the code they describe (Doxygen-style). **Sidecar files are supported** as an option, for when the user wants clean source or the language or location doesn't suit inline comments. The compiler reads both, and they're the same format.

## Why
Inline sits next to the code, so drift is visible in the same diff, the anchor (Q-21) is physically adjacent, and anyone reading the source sees the docs. Sidecars cover the cases inline handles badly: generated or vendored files, languages without doc-comment conventions, teams that don't want annotation noise in source, and annotations that span many files (custom views, Q-19).

## Consequences
- The annotation format needs an inline syntax per language comment style (`///`, `/** */`, `#`), which is defined per plugin (ADR-0007), plus a sidecar syntax.
- If an inline and a sidecar annotation describe the same target, there must be a precedence rule (Q-15).
- Insight writing inline annotations means Insight edits source files, so it has to be careful and diff-friendly.
