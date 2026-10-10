# ADR-0038: Claude queries the graph in SQL; each codebase's snapshot is a read-only SQLite database

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-26 ("I think a query language. Claude writes perfect zero-shot SQL so that's not a problem… I guess I don't actually know how this should be done in practice. I just know what I want")

## Context
BQ-26 offered fixed query tools (`related`, `paths`, `matrix`) or a query language. The fixed tools encode, in advance, which questions can be asked. That's a small piece of intelligence written into the tool.

## Decision
- **Claude queries the graph by writing SQL** against a small, documented schema. Writing the query is the intelligent part (the model); executing it is trivial (the database), per ADR-0035.
- **Schema (first version):**
  - `nodes(id, kind, subkind, name, file, span…)`
  - `edges(id, kind, from_node, to_node, resolution)`
  - `receipts(edge_id, file, byte_start, byte_end, line, col, text, statement)`
  - `tags(node_id, tag)`
  - `diagnostics`
  - `meta` (snapshot keys, Insight version)

  Kinds are the BQ-09 set. Paths use recursive CTEs.
- **Storage (refines BQ-24, decided by Claude):** each codebase's cached snapshot *is* one read-only SQLite database file. A multi-codebase snapshot `ATTACH`es the member databases plus a small link-layer database. Queries are read-only, time-limited, and their results row-capped with a "truncated, N total" signal.
- A few non-query tools remain for things SQL can't do: load and build, build status, read source from a snapshot, show a view in the window, read what you've selected, and tag coverage.

## Why
Claude writes SQL fluently, so a query language costs nothing to "learn". Any question the schema can express becomes askable without new tool code, including ones nobody anticipated. SQLite is embedded, battle-tested, needs no server, and makes the snapshot file and the query engine the same thing.

## Consequences
- BQ-26 is answered. BQ-27 (the tool list) shrinks to the non-query tools above plus `sql`. BQ-28 (result caps) becomes the query row cap. BQ-29 (`find`/`resolve`) becomes plain SQL over `nodes` and `receipts`.
- The schema is a public contract, versioned, and documented in the MCP instructions (BQ-32) so Claude knows the tables up front.
- Datalog or other engines are unnecessary unless SQL proves too slow for path queries at scale.
