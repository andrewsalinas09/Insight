# ADR-0042: Time is causal order. The solver knows what's ordered and where order breaks; race candidates are a query

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R2-04 ("I think I meant time as in causal. Aka ordered code is ordered. Async shows it's ambiguous / there's a way to show race conditions are possible. (Maybe Claude can use this tool even to find race conditions, idk)")
- **Refines:** ADR-0018 (time is explicit), replacing the "timing kinds" idea with causal order

## Decision
**Time = happens-before.** Insight shows whether one thing definitely happens before another, or whether their order is unknown.

1. **Ordered (derived, no new facts needed).** Within a function, statements run in source order: receipts carry byte positions, and blocks (BQ-18) carry branches and loops. A direct `call` completes before the caller's next statement.
2. **Order breaks, recorded mechanically:**
   - **`await`** (TS `await`, Rust `.await`): language syntax, recorded by the extractor as an order-break point within the function (ADR-0033).
   - **Handing a function to outside code**, i.e. a `value-flow` of a function into an `external` leaf (BQ-16): callbacks to `setTimeout`, `listen(...)`, `useEffect`, `thread::spawn`, and so on. The solver doesn't need to know what the external does. Being handed off is enough to mark **order unknown relative to the caller**. No library catalogue (ADR-0023/0035).
3. **Race candidates are a query:** two code regions whose relative order is unknown, both accessing the same state (`read`/`write`/`access`, field-level per BQ-13), with at least one `write` or `access`. This is pure SQL over existing facts (ADR-0038), documented in the MCP instructions as a standard query. Whether a candidate is a real race (a lock guards it, the timer only reads) is **Claude's judgment**. Later, the run tools let Claude prove a race by triggering it (ADR-0037).
4. **Display:** ordered code reads as a sequence; edges across an order break are marked *may interleave*. Claude may still add meaning in notes ("runs ~400 ms later, debounced").

## Why
This is what "explicit time" really needs: not labels like "timer" or "spawn" (library meaning, the model's), but the mechanical fact of where certainty about order ends. That fact makes race-hunting a cheap query instead of a reading exercise, which is exactly the kind of tool that helps a genius model do its job.

## Consequences
- Supersedes ADR-0018's "timing kind on every edge" and the separate swimlane view. Lanes, if ever wanted, are a wish.
- Fact schema: extractors emit `await` points (an `exit`-like marker inside the function, or a node subkind), decided in the schema work. Order within a function comes from receipt positions.
- TB-010 (async and spawns), TB-036 (nondeterministic runs) and TB-065/069 (races) get their deterministic half from this.
- A standing example query, "race candidates on state X", goes into the MCP instructions.
