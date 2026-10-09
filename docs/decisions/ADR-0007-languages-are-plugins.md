# ADR-0007: Language support is pluggable; the core is language-agnostic

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 3 ("This is not just rust… it must be able to have things for any language like a simple plugin / parser / whatever you may need")

## Context
Insight is written in Rust/Tauri, but it has to understand codebases in any language. Vibe-coded projects are often multi-language (a TS frontend over a Rust or Python backend).

## Decision
The core graph, the compiler, the tool API, and the UI know nothing language-specific. Each language is a **plugin** that maps its own constructs onto the generic graph model (ADR-0008). A plugin provides a parser, name/call resolution, claim checks for the compiler (Q-15), and optionally a runtime tracer (ADR-0003). Cross-language links (TB-007) are plugins too, for example a "Tauri bridge" plugin that pairs `invoke("x")` with `#[tauri::command] fn x`.

## Why
Hard-coding languages would cap the product at whatever we had time to build. A common model with per-language mapping is the same approach that LSP, tree-sitter, and code indexes like SCIP use. Rejected alternative: having the LLM read any language directly. That gives no deterministic fact layer (ADR-0006), so it can't be held to ADR-0002.

## Consequences
- Plugins vary in strength. A dynamically typed language resolves fewer calls statically, so its graph has more *possible* edges, which is honest (ADR-0002). The UI should show how well a plugin could resolve the code.
- Plugin interface design is Phase 1 work. Candidate building blocks (tree-sitter grammars, LSP servers, SCIP indexers) are researched in Q-08 and Q-03.
- Which languages ship first is still Q-03.
