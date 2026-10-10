# ADR-0029: A graph is a snapshot of one git commit; explanations are snapshots too

- **Status:** Accepted; amended by ADR-0036 (a snapshot is a commit *or* the working copy, and either must build)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("the graph is generated on build and you ask questions about it. There's not _really_ a concept of things changing. We could just make it track git though and each git commit is its own graph. Two graphs are just two commits… explanations are snapshots in time, not really designed to be carried over (but again we can use git for this + LLM if it's tricky)")

## Context
Several designs assumed a living graph that changes underneath saved things:
- drift detection and re-anchoring (ADR-0013 rules 3 and 6);
- explanations flagged "may be outdated" (Q-22, TB-064);
- incremental updates (Q-09);
- identity across versions (ADR-0028).

The user's model is simpler.

## Decision
- **A graph is built for one git commit and never changes.** It's an immutable snapshot keyed by the commit hash.
- **Comparing means two commits.** The system diff (Q-25) is graph(A) vs. graph(B). Reconciling renames and moves between them is the model's job, with git's own diff as input (ADR-0028, ADR-0023).
- **Explanations, teaching views, and journal entries are snapshots too.** Each one is pinned to the commit it was made on. They aren't carried forward or kept in sync. On a newer commit they're simply "from commit X". If needed, the model plus git can tell whether they still hold.

## Why
It matches how the tool is used: build, then ask. And git already *is* the versioning system. Pinning everything to a commit removes whole mechanisms (re-anchoring, staleness tracking, incremental sync) without losing anything the user cares about. A good explanation of commit X stays a true explanation of commit X forever, which is also the ADR-0002 property.

## Consequences
- **Simplifies or retires:**
  - ADR-0013 rule 6 (drift → broken receipt) becomes unnecessary: receipts are exact within their commit.
  - Q-09 (incremental freshness) is closed.
  - TB-025, TB-032, TB-048, and TB-064 are answered: everything is pinned to a commit.
  - ADR-0028's "previous build" becomes "another commit".
- **Uncommitted work:** an agent's in-progress edits aren't a commit yet. The obvious extension is to treat the working tree as a temporary snapshot (`commit + dirty`), so you can ask questions before committing. That detail is open, in Q-18.
- Graphs can be cached per commit hash, so rebuilding a commit you've already built costs nothing.
