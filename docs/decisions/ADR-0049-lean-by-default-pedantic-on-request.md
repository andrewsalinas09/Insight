# ADR-0049: Lean by default, pedantic on request

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R4-01 ("we don't want to kill context or give Claude so much stuff. If it matters maybe have Claude request it instead of on by default. Like this doesn't matter 99% of the time, right? So default off, Claude knows that this is available and can add it if needed. (We should make this another rule.) Don't be pedantic, but absolutely give Claude a way to be pedantic")
- **Refines:** ADR-0047 (tools take parameters; the solver keeps every fact)

## Decision
- **Storage stays complete** (ADR-0047): the solver records every fact.
- **Tool output is lean by default:** the facts that matter in the large majority of questions. **Pedantic detail is off by default but always one parameter away**, and `status` lists every available switch so Claude knows it exists.
- **Default-off ("pedantic") detail, first set:**
  - order breaks and race candidates arising from handoffs to the language's **standard library** (`map`, `then`, `forEach`…);
  - implicit calls that resolve to standard-library or built-in implementations (P3);
  - macro-generated facts (`generated=1`);
  - unsequenced-evaluation groups;
  - `possible-instance` merge detail.

  Literal/`names`/state hops are already opt-in via `follow` (ADR-0047).
- **"Standard library" is a mechanical fact** (`origin = project | dependency | stdlib`, from the toolchain's own package resolution), not a catalogue of meanings. Built-ins stay external leaves (R4-01, strict); Insight never knows what `map` does.

## Why
Context is Claude's scarcest resource. Flooding answers with facts that almost never matter wastes it and buries the signal. Removing those facts would make Insight incomplete when they *do* matter. Lean defaults plus advertised switches give both.

## Consequences
- Every query tool documents its pedantic switches in `status`.
- New fact kinds are classified at design time as default-on or pedantic. That's a judgment made once by design, not per answer; Claude can always switch.
