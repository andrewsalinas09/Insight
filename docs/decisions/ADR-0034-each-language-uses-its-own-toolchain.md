# ADR-0034: Each language is analyzed by its own native toolchain, written in its own language, emitting the standard representation

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-05 ("each language can have its own checker and have bindings into Rust. That way the language parsing isn't something I _ever_ need to solve. And this is the power of AI. Each language can be done in its own language with APIs / whatever for Insight to get what it needs")
- **Supersedes:** the BQ-04 ruling (plugins as compile-time Rust crates)

## Context
BQ-05 asked how to read TypeScript: the real TS checker in a Node sidecar, or a Rust-native parser without types. Field-level state (Q-20) and ADR-0033 need real language semantics, and every language has an official toolchain that already has them.

## Decision
- **Each language plugin is an extractor written in that language, using its official tooling:**

  | Language | Tooling |
  |---|---|
  | TypeScript | the TypeScript compiler API (Node), later `tsgo` |
  | Rust | rust-analyzer (in-process, since Insight is Rust) |
  | Python | its `ast` plus a type checker (e.g. pyright or jedi) |
  | Go | `go/packages` + `go/types` |
  | C/C++ | clang (libclang / clangd index) |
  | Kotlin, Swift, C#, … | their own compilers' APIs |

- **Every extractor emits the same standard representation** (ADR-0033, BQ-09) over one versioned protocol. Insight's Rust core never parses a language itself. It runs extractors, collects their facts, then runs the link passes (Tauri, wires, same-string) over the combined facts.
- **Wire format (decided by Claude, delegated):** newline-delimited JSON (one fact per line). It has a versioned schema generated from the Rust `insight-model` types as JSON Schema, so each extractor can generate or validate its own types. JSON is read and written natively by every language. Swap to a binary format only if it becomes a bottleneck.
- An extractor is a command-line program: given a snapshot's source tree (ADR-0029) and its dependencies, it writes facts to stdout.

## Why
Language semantics are the hardest, most unbounded engineering in a tool like this, and each language community already solved them in its own toolchain. Using them gives exact semantics for free and never has to be maintained by the user. Writing an extractor against a toolchain's API is exactly the kind of well-scoped task AI coding does well, one language at a time.

## Consequences
- **BQ-05 answered:** TypeScript is read by the TS compiler API in a Node extractor (`tsgo` swaps in later).
- **Crate layout (BQ-03) adjusts:** `insight-lang-*` crates become thin adapters that launch an extractor and stream its facts. Extractors live in `extractors/<lang>/`, in their own language (e.g. `extractors/ts/` is a TypeScript package). The Rust extractor can run in-process.
- **Runtime dependencies:** analyzing a TS repo needs Node, a Python repo needs Python, and so on. These are present wherever that project is developed. Per the global rule, interpreters are probed by execution, never trusted from `PATH`. A missing toolchain makes that language's facts absent (incomplete, not wrong), with a clear message.
- **Every extractor gets the same conformance test:** small fixtures per rule (BQ-46), so all languages produce the standard representation the same way.
