# ADR-0005: Annotations are committed files that live with the repo

- **Status:** Accepted; format settled by ADR-0043 (`@insight <keyword>: <value>` parsed mechanically; `tag` first; free text is read by Claude, not parsed)
- **Date:** 2026-10-09
- **Source:** user, discovery session 2 (Q-15 sub-question: where annotations live)

## Context
Annotations (claims and prose, Q-15) could live in a private Insight database or in the repository itself.

## Decision
Annotations are plain files committed to the repo. They can be tracked, diffed, reviewed, and opened without Insight. They may also sit inline in source code as structured doc comments, the way Doxygen works. Whether inline, sidecar files, or both is still open (Q-15).

## Why
- They're versioned with the code they describe, so a commit changes both together and history stays aligned.
- They're readable and editable by humans without the tool. They're documentation, not a cache.
- **They give the deterministic pipeline something to work with.** The compiler and graph builder can read existing annotations directly instead of having an LLM re-parse the whole codebase every time. That's cheaper and faster, and it makes the system less dependent on the LLM.

Rejected alternative: a private database. It's invisible, can't be reviewed, and is lost if you switch machines or tools.

## Consequences
- The annotation format becomes a public, stable spec, a kind of language, so it needs versioning.
- The compiler checks committed annotations just like freshly generated ones, so a human-written claim gets the same verification as an LLM-written one (ADR-0002).
- Stale annotations show up in diffs and can be checked in CI (Q-09).
- The knowledge profile is *not* in the repo (ADR-0004), and recorded runs probably aren't either (Q-17).
