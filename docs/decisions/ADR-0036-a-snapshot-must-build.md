# ADR-0036: A snapshot needn't be committed, but it must build

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-23 ("b. though a is simple and tbh I'm almost at 'refuse until you build successfully'. I've never had something not build… with AI, building is not a struggle. So that solves a lot of the other stuff. We don't have to worry about things being wrong or not parsing right. It can not have a commit, but it does have to build. This is also to learn and check it matches spec. If it doesn't build, that's not really what this tool is for")
- **Amends:** ADR-0029 (snapshots are per commit) and the BQ-06 and BQ-08 rulings

## Decision
- **Snapshots come in two kinds:** a git commit, or the working copy (`commit + uncommitted changes`, keyed by a content hash). You can ask about what the agent just wrote without committing it.
- **Every snapshot must build.** Before analyzing, Insight runs each language's own check:
  - Rust: `cargo check`
  - TypeScript: `tsc --noEmit` per tsconfig
  - others: their equivalents

  A repo can override the command in `.insight/config.toml`. If the check fails, **Insight refuses** with "this doesn't build: <the first errors>; build first", and builds no graph.

## Why
Insight is for learning code and checking it matches what you wanted. Code that doesn't build isn't a finished thing to learn or check. AI agents get code building easily, so requiring a build costs almost nothing, and it removes a whole class of problems: partial parses, `degraded`/`unresolved` nodes from syntax errors, macro expansion failing, guessing at broken code. Every fact then comes from code the toolchain fully accepts, which directly serves "never wrong" (ADR-0002).

## Consequences
- **BQ-06:** the "expansion off" fallback is dropped. Macro expansion always works on code that builds.
- **BQ-08:** "build anyway with diagnostics" is reversed to refuse.
- **TB-029 and TB-071** (code that doesn't compile) move out of scope.
- `unresolved` stays only for things that really can't be resolved in valid code (e.g. a missing toolchain for one language, or dynamic dispatch with no candidates), never for errors.
- Running the build check is trivial and deterministic (ADR-0035), and it doubles as the dependency-materialization step (BQ-07).
- **A snapshot can span multiple codebases** (user, BQ-24; ADR-0014): a set of (repo, commit or working copy) pairs. Each member must build, each is cached separately, and the cross-repo links are a thin layer on top.
