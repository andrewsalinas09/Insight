# ADR-0041: "Never wrong" covers Insight's facts; Claude's words are interpretation; what's checkable gets a tool (read-coverage reminder and `compare`)

- **Status:** Accepted; `compare` normalizes line endings (R3-18); reads count only when bytes match the snapshot (TB-119)
- **Date:** 2026-10-09
- **Source:** user, R2-01 ("I would say a, but mark what Claude says as interpretation, but let Claude use tools to prove it… anything that is checkable is already checked. We trust Claude is reasoning correctly. We don't question its knowledge… I ask Claude 'how do these two codes differ', it says one thing, and then it's like 'I made a mistake, I didn't read the full context'. How can we prevent that? … Can we actually write an algo for this?" then: "yes… a helpful reminder to say hey, did you really check all these lines, they may not be important but they may be… I don't want to make it start saying why it doesn't matter… I trust what it says mainly")
- **Amends:** ADR-0002 (scope), ADR-0039 (tool list)

## Decision
1. **"Never wrong" (ADR-0002) covers Insight's own deterministic facts:** the graph, paths, receipts, reference resolution, and tool outputs such as diffs. What Claude says is **interpretation**, shown as such, and it's Claude's responsibility (ADR-0037). Insight never questions Claude's knowledge or reasoning.
2. **Whatever is mechanically checkable gets a tool:**
   - **`compare(a, b)`:** an exact diff of two functions, files, or snapshots (line and structure). "How do these differ?" is answered from a computed diff, not from two reads held in mind.
   - Calls, reads, writes: SQL (ADR-0038). "This code is from here": references must resolve to real lines (ADR-0037).
3. **The read-coverage reminder:**
   - Insight records which spans Claude has read: through `source`, plus Claude Code's own file reads, reported by a small hook.
   - When `show` or `compare` references code Claude hasn't fully read, Insight returns a **gentle reminder to Claude** ("you referenced `graftNode` (L4941–4972) but only read L4957–4971; the rest may or may not matter"). Claude reads more, or proceeds.
   - The reminder goes to Claude only. It never blocks, and Claude does **not** narrate to the user why unread lines don't matter.

## Why
"Didn't read the full context" isn't a reasoning failure. It's a reading gap, and whether something was read is exact bookkeeping, which is the tool's job by ADR-0035. Comparisons are the most common place where a partial read becomes a wrong answer, and a diff is a trivial algorithm. Instructions alone ("always read everything") help but aren't guaranteed; a reminder triggered by an actual gap is. Keeping it quiet and non-blocking respects the user's trust in the model and avoids noisy self-justification.

## Consequences
- The MVP tool list (ADR-0039) adds `compare`. The read log and its hook come with the MCP layer (M2).
- MCP instructions still say "read the full function before explaining it". The reminder is the safety net.
- The renderer styles Claude-authored content as interpretation by element kind (pending R2-02's exact styling rule).
