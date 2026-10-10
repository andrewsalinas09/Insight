# MCP instructions

The text `insight mcp` gives an agent up front (MCP `instructions`). Everything after the first `---` line is served verbatim; it's compiled into `insight.exe` from this file (`crates/insight-mcp`). It was adapted from the Round 4 runtime-Claude simulation and finalized at M2 to describe only what's built. Add a line when a tool lands (`compare`, `show`, `selection`, `tag_coverage`, `reach()`/`nid()`, the `writers` and `race_candidates` views, pedantic switches). The ADRs, BUILD-QUESTIONS and SCHEMA.md win over this text.

---

Insight is a read-only, deterministic graph of a project's code at a snapshot. Its facts are exact but incomplete, and every gap is stated. Deciding what they mean is your job. Insight has no framework or library knowledge: everything is in generic programming concepts (containers, calls, value flow, literals, names).

1. **Call `status` first.** It lists projects and their snapshots, what's loaded, the live schema, enum values with counts, the exact definition of each `follow` token, the shipped views, the frozen-source folder, the ceilings, and what's not built yet.
2. **Load** with `load(project)`. It becomes the unqualified `main` schema and stays pinned until you `load` again without an alias. `load(project, alias=…)` attaches another snapshot for SQL as `<alias>.<table>`. `build` makes a new snapshot of an approved project. It never switches what's loaded, and nothing over MCP can create or approve a project: that's the user's step.
3. **Find things** with `sql`: one read-only SQLite statement. You choose `max_rows` and `timeout_ms` within the ceilings, and the result always reports the total row count. Node ids are paths (`app/src/App.tsx › MainApp › graftNode`); `nodes.path LIKE '%› name'` finds by name. For text search, Grep and Read the frozen-source folder (exactly the snapshot's bytes), not the working tree.
4. **Trace** with `path(from?, to?, follow=[…])`. Nothing is followed unless you name it: `call`, `handoff`, `literal`, `names`. Give both ends for routes between them, only `from` for everything reachable, or only `to` for everything that reaches it. Ends can be lists. Results are labeled hops with receipts. `possible` candidates, cycles and recursion are reported, never hidden. Use `max_occurrences` to skip common strings and `max_depth` to bound route length; pick them per question.
5. **Literal and `names` links are exact text equality** (a string equal to another string, or to a declaration's name). Whether one is a real connection (an RPC name, an event name, a message type) is your call, so check the receipts.
6. **Unresolved is a fact, not an error.** `resolution='unresolved'` edges and `to_nid IS NULL` mark calls the compiler couldn't pin down. Say so; don't guess.
7. **Read before explaining.** Use `source(refs=[…])` (node ids, `file`, `file:line`, `file:start-end`; batched) or the frozen-source folder. Read a function in full before you explain it.
8. **Wishes:** if you wish Insight could do something it can't, tell the user why. If they agree, append it to the Wishes section of the global INSIGHT.md. Don't work around a missing tool by guessing.
