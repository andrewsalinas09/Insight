# ADR-0018: Time is explicit, never implied by edges

- **Status:** Accepted; refined by ADR-0042 (time = causal order: ordered vs. "order unknown" at await points and functions handed to outside code; race candidates are a query; timing-kind labels and the swimlane view dropped); also amended by ADR-0033 (language-level async is a plugin fact); see ADR-0048 (no library/framework classification of edges; `setTimeout`/`spawn`/`listen` meanings are the model's)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("Time needs to be very clear, I thought it would be clear with the edges but maybe it's not. That needs to be very explicit")

## Context
A call graph shows *what can reach what*, not *when*. Much of the confusion in the real learning log (`docs/research/2026-10-learning-session-patterns.md`) was temporal: workers polling a table, `await` suspending, a check-and-claim race, a request returning before the background work starts, devices exchanging messages. Drawn as plain arrows, a synchronous call, a spawned task, and an event handled "later" look identical, and that misleads (ADR-0002).

## Decision
1. **Every edge carries its timing kind**, drawn distinctly:
   - *sync call/return* (the caller waits)
   - *async/await* (the caller suspends)
   - *spawn* (runs concurrently; the caller continues)
   - *event/callback* (runs later, triggered elsewhere)
   - *periodic/poll* (repeats on a timer)
   - *message over a wire* (crosses a process or device, with latency, ADR-0014)
2. **Execution contexts are visible.** Threads, tasks, processes, windows, and devices are shown, so "this runs on a worker thread" or "this runs in the pop-out window" is a fact on the graph.
3. **A sequence (swimlane) view is first-class**, alongside the bubble graph: lanes are execution contexts, and time flows down. Step-throughs (ADR-0011) and scenarios (ADR-0010) play in it naturally, so the race in TB-065 becomes two lanes interleaving.

## Why
Ordering and concurrency are exactly where "what does this do?" gets hard and where AI-written code hides bugs (lost errors in background tasks, races, stale state). Implicit time lets a reader assume sync where there's async, which is a false belief created by the picture itself.

## Consequences
- The timing kind is fact-layer data. Plugins must classify edges (`thread::spawn`, `tokio::spawn`, `await`, `setTimeout`, `addEventListener`, `emit`/`listen`, loops with `sleep`). Unclassifiable edges are marked *unknown timing*, never defaulted to sync.
- The Q-08 rendering research must cover sequence views as well as graph layout.
- Expectations (Q-26) can talk about time ("the UI never waits on the network"), so the query language needs timing kinds.
