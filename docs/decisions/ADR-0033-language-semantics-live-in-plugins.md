# ADR-0033: Language semantics live in one place, the language plugin, which parses each language to a standard

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-02c follow-up ("it's not _just_ tags though. I _am_ granting semantics in one place, the ability to parse a language to standard. So it can reason about the code itself when making graphs and what not. Tags are extras to define something for some reason, mainly for efficiency / annotation")
- **Amends:** ADR-0023 (which said the solver never understands anything) and ADR-0032 (which framed tags as the main carrier of meaning)

## Context
ADR-0023 put "anything requiring understanding" on the model and described the solver as purely mechanical. Taken literally, that would forbid even knowing what a method call resolves to. The user clarified where the line actually is.

## Decision
Semantics come in three layers:
1. **Language semantics → the language plugin (granted).** Each plugin parses its language **to a standard**: one language-agnostic representation (BQ-09) that every query works on. The plugin may use everything the language itself defines: scopes, types and inference, method and trait resolution, mutability (`&mut`, `const`), `async`/`await`, closures, destructuring, macros and JSX desugaring. This lets the solver reason about the code itself when building graphs.
2. **Meaning beyond the language → the model.** Library, framework and domain meaning ("this writes a file", "this is a 400 ms debounce", "this is the logger") is the model's job, as in ADR-0023.
3. **Tags → declared extras** (ADR-0032), for efficiency and annotation: shortcuts the model or user writes so meaning can be queried directly.

The test for "is this allowed in the solver?": **is it defined by the language spec, or by a library or domain?** Language spec means plugin. Library or domain means model or tag.

## Why
A real "standard" representation needs real language understanding. You can't connect `onto.children.push(…)` to `HistNode.children` without types. What must stay out of the solver is the unbounded part: every library's and domain's meaning. Languages are finite and well specified; their ecosystems aren't.

## Consequences
- BQ-05 / BQ-06 (TS and Rust front ends) should favor engines with real type and resolution semantics.
- Language-level timing facts (`await`, `async fn`, `async` closures) become plugin facts. Library-level timing (`setTimeout`, `thread::spawn`, `tokio::spawn`, `useEffect`) stays model or tag territory.
- `std` items are part of a language's standard library, which is borderline. Rule for now: std is a library (model or tag), so the plugin stays focused on the language itself.
