# MCP instructions (DRAFT)

The text Insight's MCP server gives an agent up front. It's adapted from the Round 4 runtime-Claude simulation, and gets finalized at M2. This is a draft and not normative: the ADRs and BUILD-QUESTIONS win.

---

Insight is a read-only, deterministic graph of a loaded project snapshot. Its facts are complete and exact; deciding what they mean is your job. Insight has no framework or library knowledge.

1. **Call `status` first.** It lists the loaded snapshots (SQL aliases; the active one is unqualified and stays pinned until you switch), the schema, enum values with counts, the definition of each `follow` token, the shipped views (`callers`, `writers`, `receipt_text`, `race_candidates`), the frozen-source folder, and the **pedantic switches**.
2. **Output is lean by default.** Rarely-relevant detail is off until you ask for it: standard-library handoffs, built-in implicit calls, macro-generated facts, unsequenced groups. Turn it on with the switches when it matters.
3. **Find things** with `sql` (one read-only statement; you set `max_rows` and `timeout_ms` within the ceilings). Use `nid('name')` to resolve names and `reach(from, follow, dir)` for reachability. For text search, Grep and Read the frozen-source folder, not the working tree.
4. **Trace** with `path(from?, to?, follow=[…])`. Nothing is followed unless you name it. Hops are labeled; `possible` and absent candidates, cycles and recursion are reported. Leave one end open for forward or reverse reachability.
5. **Effects** (disk, network, IPC, UI): group external leaves by package, decide which ones mean the effect, then reverse-reach from them. Literal and `names` links are exact text equality; whether they're real connections is your call.
6. **Order:** `await`, handoffs, `names` hops and un-awaited async calls are order-unknown, and `timing` states the real moment for deferred code. Judge `race_candidates` yourself.
7. **`access` means "may write".** Check `via_eid` for the method behind it. Split merged generic fields by `base_nid`.
8. **Read a function in full before explaining it.** A quiet reminder may arrive; read more or carry on, and don't narrate it.
9. **Show:** `show(view)` composes the nine elements, by name or by an embedded `path` spec. The explanation stays in the terminal; the window is the visual guide.
10. **"How do these differ?"** Use `compare`.
11. **Wishes:** if you wish Insight could do X, tell the user and append it to the Wishes section of the global INSIGHT.md.
