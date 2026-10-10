# ADR-0048: The solver speaks programming concepts. Language knowledge lives only in extractors; no framework knowledge anywhere

- **Status:** Accepted; scope: the rule covers the analysis (extractors, solver, link passes, query tools), not Insight's own implementation stack (its Tauri shell and updater are fine); refined by ADR-0049 and R4-01 (built-ins are external leaves with origin=stdlib)
- **Date:** 2026-10-09
- **Source:** user, approving R3 ("I can't have language-specific things. The solver needs to be able to handle any situation and give Claude what it needs while being useful at what a parser / AST-like thing does. It should handle programming concepts. Programming things. If a language has it, it should cover that concept")
- **Supersedes:** BQ-21 and R2-31 (the Tauri link plugin); the framework-specific parts of ADR-0014 (protocol plugins) and R3-14

## Decision
1. **The standard representation is a vocabulary of programming concepts**, not of any language or framework: module, type, interface/implementation, function, closure, call, **handoff** (a function or value given to other code to run or use), await/suspension, state, read/write/access, binding, branch, loop, exit, literal, and so on. If any supported language has a concept, the vocabulary covers it; new concepts are added when a language needs one.
2. **Language knowledge lives in exactly one place: each language's extractor** (ADR-0034), which translates its syntax and semantics into those concepts, as a compiler does (ADR-0043). For example TSX `<Foo onGraft={g}/>` becomes a handoff of `Foo` and `g` to external code; Rust `.await` and TS `await` become await; an un-awaited async call becomes "callee continues later".
3. **No framework or library knowledge in the solver or the extractors.** Insight never knows what Tauri, React, `setTimeout` or `thread::spawn` *mean*. That's Claude's (ADR-0023/0035).
4. **The generic name link replaces framework links:** a **string literal that exactly equals a symbol's name** produces a `names` link (literal → symbol), across languages and repos in a project. `invoke("set_fletcher_chain")` → `fn set_fletcher_chain` matches by name alone, and so do `getattr(x, "save")`, route tables and event names. Insight states only "this literal equals that name"; whether it's a real connection is Claude's call. Hops through `names` links are *order unknown* (ADR-0042).

## Why
A tool that must work on any codebase can't carry knowledge of any particular framework, which is an endless catalogue (ADR-0023). Programming concepts are finite and shared across languages. Exact name equality is a mechanical fact that covers every dispatch-by-name pattern at once.

## Consequences
- **M1's Tauri hop** is found via the generic `names` link (`path … follow=[…, names]`); the gate (R3-12) lists it as a `names` hop.
- `link/tauri` is dropped from the R3-G internal schema list, and the `names` link is added. Same-literal links (BQ-22) and `names` links are both literal facts.
- R3-14's "every `link` hop is an order break" becomes "every `names` hop is order-unknown". JSX is handled as a handoff by the TS extractor's mapping, not as a React rule.
- Wire links across devices (ADR-0014, R2-38) are the same mechanism: exact literal and name facts, with Claude deciding what's really the same message.
