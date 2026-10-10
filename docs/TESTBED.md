# The Test Bed

The permanent catalog of edge cases. **Every proposed design change must be checked against every case here. Every new edge case gets an entry the moment it's raised — before it's solved.** A design change that breaks any Solved case is rejected or must supersede the relevant ADR.

Statuses: **Solved (design)** means a documented mechanism handles it (cite the ADR or doc). **Partial** means it's mostly handled, with a named gap. **OPEN** means there's no accepted answer yet (it should have a Q-ID in OPEN-QUESTIONS). **Out of scope** means a decision removed the case (cite the ADR).

These are design-time tests today. Once code exists, each becomes an executable fixture: a small real codebase plus the expected graph.

## Status update after the R2 round (authoritative, 2026-10-09)

The cases below keep their original text and answers as history. **Where a case's own Status line disagrees with this table, this table wins.** Cases not listed are unchanged.

| TB | Status now |
|---|---|
| 001 | Solved (design): `call` edges between function containers (BQ-09) |
| 003 | Solved (design): one-hop value flow gives `possible` edges (BQ-15); callbacks into externals get `value-flow` (BQ-16) |
| 004 | Solved (design): the graph stores edges, not paths; `path` reports recursion (BQ-09, R2-12) |
| 005 | Solved (design) for TS/Rust: rust-analyzer with proc macros on (BQ-06) on code that always builds (ADR-0036); TS compiler API (ADR-0034) |
| 006 | Partial: possible edges + same-literal links (BQ-15, BQ-22); computed keys stay `unresolved` (R2-03) |
| 007 | Solved (design): Tauri command link (BQ-21, R2-31) + JSX render calls (BQ-16) |
| 008 | OPEN, post-MVP: route strings may link via same-literal links (BQ-22) |
| 009 | Partial: same-literal links (BQ-22); what a listener means is the model's (ADR-0023) |
| 010 | Solved (design) for connections: `value-flow` into spawn (BQ-16), channel `cell` (BQ-17); ordering = ADR-0042 (may interleave) |
| 011 | Solved (design): block containers with condition text (BQ-18); external globals are state (BQ-20) |
| 012 | Solved (design): block + `exit` edge (BQ-18); naming it is the model's |
| 013 | Solved (design): one `exit` per `?` (BQ-18); grouping via Groups/Collapse (ADR-0040) |
| 014 | Partial: `throw` is an `exit` edge and `catch` is a block (BQ-18); the throw→catch connection isn't an edge |
| 015 | Partial: a block per arm (BQ-18); layout is a frontend question |
| 016 | Partial: loops are blocks (BQ-18); break/continue aren't modeled |
| 017 | Solved (design): blocks, not paths (BQ-09/18); `path` visits each node once (R2-12) |
| 018 | Partial: one external leaf per symbol (BQ-20); expanding into dependencies isn't designed |
| 019 | Solved (design): all projects in a repo (R2-26); several repos = one Insight project (ADR-0045) |
| 020 | Solved (design): everything the build compiles is in the fact layer (BQ-20, R2-17); what to show is curation (ADR-0022) |
| 021 | Partial: JSX props bind handlers (BQ-16); picking in the running app is open (Q-05) |
| 023 | Solved (design): the agent maps the prompt via SQL (ADR-0025, ADR-0035) |
| 024 | Solved (design): labels are the model's notes, shown as interpretation (ADR-0041); the real code is one `source` call away |
| 025 | Solved (design): ADR-0029 |
| 026 | Partial: labeling cost is moot (ADR-0025); build cost and latency are open (Q-12) |
| 027 | Solved (design): complete fact layer (ADR-0022); unreachability is a query |
| 028 | Solved (design): the model's judgment (ADR-0035) |
| 029 | Out of scope (ADR-0036) |
| 030 | Solved (design): views reference only real nodes and edges; bad references return errors (ADR-0040, R2-02, BQ-36) |
| 031 | Solved (design): linked references resolve (BQ-40); plain prose is the model's responsibility (ADR-0037, ADR-0041) |
| 032 | Solved (design): ADR-0029 |
| 040 | Solved (design): there's no add-edge tool (ADR-0039); the model notes it or picks a `possible` edge (R2-03) |
| 041 | Solved (design): declaration-symbol state identity connects reads and writes regardless of injection (BQ-13, ADR-0023) |
| 042 | Solved (design): BQ-13 + BQ-14 (`access` through locks) |
| 043 | Solved (design): TB-007 + TB-042 + ADR-0040 groups and excerpts |
| 044 | Solved (design): no claim compiler (ADR-0024); tags are declared, not checked (ADR-0032, ADR-0043) |
| 046 | Solved (design) for TS/Rust via native toolchains (ADR-0034) |
| 047 | Solved (design): callback candidates come from BQ-15; there's no inferred tier (R2-03) |
| 048 | Solved (design): ADR-0029 |
| 049 | Solved (design): Collapse and abstraction nodes expand to real nodes (ADR-0040) |
| 050 | Solved (design): ADR-0026/0040 abstraction node |
| 051 | Solved (design): exact spans within an immutable snapshot (BQ-12, ADR-0029) |
| 052 | Solved (design): `path` with options (R2-12) |
| 053 | Solved (design): a `possible` edge per assigned value, plus "absent" (BQ-15) |
| 054 | Solved (design): tagging is opt-in per project (ADR-0032); annotations are optional (ADR-0024) |
| 055 | Solved (design): no annotation; the model explains live (ADR-0024) |
| 056 | Deferred to the frontend session (R2-11, "+N hidden") |
| 057 | Solved (design): the model's call, reading code via `source` (ADR-0035) |
| 058 | Out of scope for Insight: the model's teaching judgment (ADR-0035); journal post-MVP |
| 059 | Solved (design): external leaves name their crate or package (BQ-20) |
| 060 | Solved (design): Steps + Example data (ADR-0040) |
| 061 | Solved (design): the model drives a Steps view (ADR-0040, BQ-39) |
| 062 | OPEN, post-MVP (journal) |
| 063 | Accepted risk: rewrites are the model's responsibility (ADR-0037); the real code is one expand away |
| 064 | Solved (design): ADR-0029 |
| 065 | Solved (design): Steps + Example data (ADR-0040), race candidates (ADR-0042); triggering the race is an optional self-check, post-MVP (ADR-0037) |
| 066 | Out of scope (R2-07, ADR-0037) |
| 070 | Post-MVP: the model may verify with the run tool before showing (ADR-0037); not a gate |
| 071 | Out of scope (ADR-0036) |
| 072 | OPEN, post-MVP: the model judges what to stub (ADR-0023); Q-07 |
| 075 | Partial: the query is SQL (ADR-0038); storage and re-run timing open (Q-26, Q-37) |
| 076 | Solved (design): the model reads the two-snapshot diff (ADR-0028, R2-32) |
| 078 | Partial: the count is a SQL query (ADR-0038); post-MVP |
| 080 | Partial: the query is SQL (ADR-0038); storage and re-run timing open (Q-26, Q-37) |
| 081 | Solved (design): BQ-15 gives `onGraft?.()` main / pop-out / none; `hist-cmd` via BQ-22 |
| 082 | Solved (design) for facts (BQ-13); the shape comparison is the model's (ADR-0035), or `compare` (ADR-0041) |
| 083 | Solved (design): Tauri link (BQ-21) + call/write path to `fsx::write_atomic` (R2-12); "system-wide" is the model's |
| 084 | Solved (design): `access` on `hist` (BQ-14) + setter `cell` (BQ-17); the check is a query or the model's |
| 085 | Solved (design): the empty catch is an anonymous function with no calls (ADR-0022); reachability is a query |
| 086 | Solved (design): ADR-0008 clarification + ADR-0022 + BQ-10 |
| 090 | Partial, later: probes (ADR-0015); firmware builds (R2-37) |
| 091 | Solved (design): a model-composed view with Collapse (ADR-0040) |
| 095 | OPEN, post-MVP (ADR-0016 SQL plugin; ADR-0023) |
| 097 | Measured by the BQ-46 oracle report (resolution rate per run) |
| 098 | Solved (design): the model sorts the connected facts, with fallbacks as a neutral row (ADR-0023, Q-31) |
| 099 | Solved (design): causal order (ADR-0042); `await` and handed-off functions are marked *may interleave* |
| 100 | OPEN: the MVP acceptance test; M1 gate restated in R2-12 |
| 102 | Solved (design): unchanged (BQ-17) |
| 105 | Solved (design): code-standing elements must resolve, others are free (R2-02) |
| 106 | Solved (design): Example data + frames (ADR-0040, R2 Round 6 table) |

---

## A. Resolving calls

### TB-001: Plain direct call
`handleSave()` calls `validate()` calls `write()`. This is the baseline: three bubbles and two call edges.
**Stresses:** the basic model. **Status:** OPEN (Q-04).

### TB-002: Trait / interface dispatch
`storage.save()` where `storage: dyn Storage` has three implementations. Static analysis can't know which one runs.
**Stresses:** truth model, static limits. **Answer:** ADR-0002 + ADR-0003. Draw three *possible* edges, never the LLM's guess. A dynamic run upgrades the one that actually ran to *observed*. **Status:** Solved (design).

### TB-003: Callbacks and closures
A function is passed as an argument and invoked somewhere else (`items.map(render)`, `onSuccess: () => …`).
**Stresses:** static resolution. **Status:** OPEN (Q-02).

### TB-004: Recursion and mutual recursion
`walk()` calls `walk()`; `a()` calls `b()` calls `a()`. The graph must stay finite and readable.
**Stresses:** graph model. **Status:** OPEN (Q-04).

### TB-005: Macros, decorators, generated code
Rust `#[derive]` / `#[tauri::command]`, Python decorators, codegen. The real call target doesn't appear in the source text.
**Stresses:** static analysis. **Status:** OPEN (Q-02, Q-03).

### TB-006: String-keyed dispatch
Routing tables, `getattr(obj, name)`, `handlers[event.type](…)`.
**Stresses:** truth model. **Answer:** ADR-0002: if keys can be enumerated statically, draw possible edges; otherwise show an explicit "unresolved dispatch" stub (incomplete, not wrong). Observed runs fill it in (ADR-0003). **Status:** Partial (enumerating keys is per-language work).

## B. Crossing boundaries

### TB-007: UI click → Tauri IPC → Rust
A React `onClick` calls `invoke("save_file", …)`, which reaches a `#[tauri::command] fn save_file` in Rust. That's a cross-language hop, linked only by a string. Insight analyzing itself would hit this immediately.
**Stresses:** cross-language edges. **Status:** OPEN (Q-02, Q-03).

### TB-008: Frontend fetch → backend route
`fetch("/api/users")` reaches the route handler in a different service or repo.
**Stresses:** cross-process edges, scope. **Status:** OPEN.

### TB-009: Events and pub-sub
`emit("saved")` is handled by a listener registered somewhere else. There's no call edge in the source.
**Stresses:** implicit edges. **Status:** OPEN (Q-02).

### TB-010: Async, spawned tasks, channels
`tokio::spawn`, promises, messages sent over a channel and received in another task. Execution continues somewhere that isn't a callee.
**Stresses:** graph model. **Status:** OPEN (Q-04).

### TB-011: Config- and environment-driven branching
Feature flags, `cfg!(target_os)`, `.env` values. A branch exists in the code but is decided outside it.
**Stresses:** branch labeling. **Status:** OPEN.

## C. Branches

### TB-012: `if !valid` early return
The user's canonical example: two edges, `valid` continues and `!valid` returns an error. The LLM labels the second one "error handling".
**Stresses:** branch edges, labels. **Status:** OPEN (Q-04).

### TB-013: Rust `?` operator
Every `?` is a hidden early-return branch. Ten `?`s in one function means ten error edges. Show them all, or group them?
**Stresses:** hierarchy, noise. **Status:** OPEN (Q-04).

### TB-014: Throw deep, catch far up
An exception thrown five calls deep is caught in the top-level handler. That's a non-local edge.
**Stresses:** graph model. **Status:** OPEN (Q-04).

### TB-015: Match with many arms
A `match` with 15 arms, some of which share a body.
**Stresses:** layout, grouping. **Status:** OPEN (Q-04, Q-08).

### TB-016: Loops
Is a loop body one bubble with a back-edge, or unrolled? What about `break` and `continue`?
**Stresses:** graph model. **Status:** OPEN (Q-04).

### TB-017: Path explosion
Thirty sequential independent `if`s give 2³⁰ paths. The graph must represent branches, not enumerate paths.
**Stresses:** scalability of the model itself. **Status:** OPEN (Q-04).

## D. Scope

### TB-018: Calls into dependencies / std
`serde_json::to_string`, `react-query`, `fs.writeFile`. These are leaves by default and expandable on request (the user's ruling in the pitch; not yet an ADR).
**Stresses:** project scope, expansion. **Status:** Partial (Q-09 for how expansion works).

### TB-019: Monorepo / multiple projects
Several packages in one repo, where one calls into another. Is that inside scope?
**Stresses:** scope definition. **Status:** OPEN.

### TB-020: Vendored or generated code checked in
Code that sits in the repo but the user didn't really "add".
**Stresses:** scope definition. **Status:** OPEN.

## E. Triggers

### TB-021: Trigger is a UI element
The user clicks the "Save" button. How is that mapped to `onClick={handleSave}`, especially through component props and wrappers?
**Stresses:** trigger mapping. **Status:** OPEN (Q-05).

### TB-022: Trigger is a CLI command
`mytool sync --force` has to resolve through the argument parser (clap, argparse, commander) to the subcommand handler.
**Stresses:** trigger mapping. **Status:** OPEN (Q-05).

### TB-023: Trigger is a prompt
"What happens when a user logs in?" maps to several entry points (login form submit, OAuth callback, session refresh).
**Stresses:** LLM mapping, truth model. **Status:** OPEN (Q-01, Q-05).

## F. The LLM layer

### TB-024: Wrong label
The LLM labels a retry loop as "error handling". The user must be able to see the source behind every label and correct it.
**Stresses:** truth model. **Answer:** ADR-0002: the label is prose, so it's displayed as interpretation with linked source. The user can correct it. A structured claim would be checked by the compiler (Q-15). **Status:** Partial (Q-15).

### TB-025: Code changes after the graph is built
A function is renamed or moved, or a branch is added.
**Stresses:** freshness. **Status:** OPEN (Q-09).

### TB-026: Huge codebase
1M+ lines. What does tracing cost, and what does labeling cost?
**Stresses:** performance, cost. **Status:** OPEN (Q-12).

## G. Vibe-coded reality

### TB-027: Dead code
Functions that are never reachable from any trigger. These are common in AI-written code.
**Stresses:** what the graph says about absence. **Status:** OPEN.

### TB-028: Near-duplicate functions
`formatDate`, `formatDate2`, `fmtDate`: three almost-identical helpers generated in different sessions.
**Stresses:** labeling, a possible insight feature. **Status:** OPEN.

### TB-029: Code that doesn't compile or type-check
A half-broken vibe-coded project should still produce a (partial) graph.
**Stresses:** resilience of analysis. **Status:** OPEN (Q-02).

## H. Annotations and the compiler

### TB-030: The LLM claims a call that doesn't exist
The annotation for `handleSave` says `calls: backupFile`, but there is no such edge in the static graph.
**Stresses:** hallucination removal. **Answer:** the compiler rejects the claim; it's never displayed (ADR-0002, Q-15). **Status:** Partial (the claim vocabulary in Q-15 is still open).

### TB-031: Prose mentions a function that doesn't exist
The summary says "then it retries via `retryWithBackoff`", but no such identifier exists in the code.
**Stresses:** checking prose. **Status:** OPEN (Q-15).

### TB-032: The claim is true now, but the code changes
`returns Err when: !valid` is verified, then the developer removes the check. The annotation must not keep asserting the claim.
**Stresses:** freshness, live mode. **Status:** OPEN (Q-09, Q-15, Q-18).

### TB-033: The claim is true statically but never happens in practice
A static edge exists, but its guard is always false at runtime (dead branch). That's legal: the static graph says *possible*, not *happens*.
**Stresses:** wording of the two provenances. **Status:** Solved (design): ADR-0003 provenance.

## I. Dynamic runs

### TB-034: A run takes a path that static analysis marked "possible"
**Answer:** the edge is upgraded to *observed* (ADR-0003). **Status:** Solved (design).

### TB-035: A run takes a path that static analysis missed entirely
For example, reflection or eval, or a gap in the analyzer. The runtime shows an edge that isn't on the map.
**Stresses:** what's true when the two sources disagree. **Answer (proposed):** the observed edge is true, so add it with `observed` provenance and file it as an analyzer gap. **Status:** OPEN (Q-17).

### TB-036: Nondeterministic run
The same click takes different paths on different runs (timing, network, random).
**Stresses:** overlay model. **Status:** OPEN (Q-17).

## J. User context

### TB-037: "I know React, not Rust"
Explanations of the TS side should skip the basics, and the Rust side should explain `?` and ownership.
**Stresses:** knowledge profile. **Answer:** ADR-0004: free-form notes, which the LLM interprets. **Status:** Solved (design).

### TB-038: Knows the concept, not this language's version
The profile says "I know multithreading, but only Rust's, not C++'s". Explaining a C++ `std::mutex` should skip concurrency theory and bridge from Rust's `Mutex<T>` (which owns its data, unlike a C++ mutex, which guards data separately).
**Stresses:** knowledge profile granularity. **Answer:** ADR-0004: nuance a matrix can't hold is exactly what notes can. **Status:** Solved (design).

### TB-039: Same claim, two users
Two users with different profiles view the same function. The prose differs, but the claims and structure must be identical.
**Stresses:** personalization vs. truth. **Answer:** ADR-0002 + ADR-0004. **Status:** Solved (design).

## K. The LLM tool layer and views

### TB-040: The LLM tries to "add" a missing edge
While organizing, the LLM is convinced A calls B (through a callback the analyzer missed).
**Answer:** ADR-0006: there is no add-edge tool. It can note the suspicion as prose ("may also call B via callback, unverified"), or the user can record a run to observe it (ADR-0003). **Status:** Solved (design).

### TB-041: Framework-injected state
A Tauri command receives `State<T>`, which the framework injects from `app.manage(...)` at setup. There's no call edge from setup to the command, yet that's where the data comes from.
**Stresses:** data edges, framework knowledge. **Status:** OPEN (Q-20).

### TB-042: A member reached through a lock chain
A command reads a field via managed state → `Mutex<Core>` → a nested model struct → a collection → an element. The user asks "where does this value come from?"
**Stresses:** access-path edges, lock visibility. **Status:** OPEN (Q-20).

### TB-043: "Show me how clicking Start reaches the core state"
A UI button → IPC command → the app's layered state hierarchy (managed state → core → nested model). The LLM composes a custom view with hierarchy and code snippets.
**Stresses:** custom views, cross-language trigger (TB-007), data edges. **Status:** OPEN (Q-19, Q-20).

### TB-044: An inline annotation is edited by hand into a wrong claim
A developer edits a doc-comment claim to `calls: notify`, but the code doesn't call it.
**Answer:** ADR-0005 + ADR-0002: human-written claims are compiled the same way. It's rejected and flagged, never displayed as true. **Status:** Solved (design).

## L. Languages and anchors

### TB-045: Dynamically typed language
In Python or plain JS, `obj.save()` can't be resolved without types. The plugin must produce *possible* edges, or an unresolved stub, rather than guess.
**Stresses:** plugin strength. **Answer:** ADR-0007 + ADR-0002. **Status:** Solved (design).

### TB-046: Methods defined away from their type
Rust `impl Foo` blocks in another file, C# partial classes, JS prototype assignment. The method must still nest inside its type bubble.
**Stresses:** container mapping. **Answer:** ADR-0008: the plugin maps methods to their type. **Status:** Partial (per-plugin).

### TB-047: The LLM adds a callback edge
The LLM claims "this `onDone` callback is `refreshList`", anchored at the call site. The compiler can prove the site and the target exist, but not the connection.
**Stresses:** the inferred tier. **Status:** Partial: ADR-0013 rule (mechanism unproven): show as *inferred*).

### TB-048: Annotated code is deleted or rewritten
An annotation's anchor now points at code that no longer matches.
**Stresses:** drift. **Status:** Partial: ADR-0013 rule (mechanism unproven): the broken receipt triggers a re-check, and the claim is hidden until re-anchored).

### TB-049: The LLM hides three trivial wrapper layers
`onClick → handleClick → doSave → saveImpl`, where the middle two only forward. The LLM collapses them into one visible step.
**Stresses:** subtraction. **Status:** Partial: ADR-0013 rule (mechanism unproven): allowed, because it's incomplete rather than wrong, and expandable back to the real functions).

### TB-051: An unrelated edit above an annotated function
Twenty lines are inserted at the top of the file. Every line-number receipt below would shift and "break", which is false drift.
**Stresses:** anchor stability. **Status:** Partial: ADR-0013 rule 3 (mechanism unproven): receipts are code quotes matched within the annotated container, not line numbers; see `docs/examples/save-note-worked-example.md`).

### TB-052: Transitive effect
`save_note` no longer calls `fs::write` directly. It calls `write_atomic`, which does. Is "save_note writes to disk" still true?
**Stresses:** claim vocabulary. **Status:** OPEN (proposed answer: explicit `via` claims, checked by reachability in the fact layer; Q-15).

### TB-053: A callback field with more than one possible value
`on_saved` is built as `reindex` in prod and as a no-op in test setup, or it's reassignable (Python `self.on_saved = …`). The claim "calls reindex" is true in prod only.
**Stresses:** exhaustiveness proofs, plugin strength. **Status:** OPEN (proposed answer: chain receipts; the plugin proves "only value" where the language allows it (proven), otherwise each candidate is a *possible* edge and the LLM's pick is *inferred*; a recorded run marks the one actually *observed*).

### TB-054: Annotation bloat
Asked to "document the codebase", the LLM writes a claim for every branch and call, so every line gets documented.
**Stresses:** density, duplication-driven drift. **Status:** Partial: ADR-0013 rule 7 (mechanism unproven). Restated facts are linted as redundant, and only intent/why plus gap-filling claims are committed).

### TB-055: A trivial function
`fn is_empty(&self) -> bool { self.items.len() == 0 }`. Nothing to add beyond the code.
**Stresses:** density. **Status:** OPEN (proposed answer: no annotation; any explanation is generated live).

## M. Learning (from a real learning log; see `docs/research/2026-10-learning-session-patterns.md`)

### TB-056: A partial view creates a false belief
A view shows a state container with only one field expanded. The learner concludes "this wrapper is pointless". In reality it holds a dozen siblings.
**Stresses:** subtraction vs. ADR-0002. Omission can mislead. **Status:** Partial: ADR-0013 rule 2 (mechanism unproven): every collapsed or filtered view shows what's hidden, e.g. "+11 more fields", and teaching excerpts name their omissions).

### TB-057: Docs say one thing, code says another
The README says the worker pool has N threads; the code says M. The learner asks how many.
**Stresses:** code-first. **Status:** OPEN (proposed answer: answer from the code, and surface the stale doc as a finding: Q-21 code is truth).

### TB-058: "Makes sense"
The learner says "makes sense, continue", but two steps later is lost on the same concept.
**Stresses:** the learning model. **Status:** OPEN (Q-22: the journal never records understanding as reached unless it's demonstrated).

### TB-059: Same name, different origins
Two calls named `spawn` on one path: one is the project's own function, one is the standard library's thread spawn. The learner can't tell which is which.
**Stresses:** symbol origin. **Status:** OPEN (proposed answer: deterministic origin badges (ours / std / framework / dependency) on every symbol; this is fact-layer data).

### TB-060: The learner's proposed alternative is subtly wrong
"Why not just make the one flag atomic?" The flag write is atomic, but check-then-claim isn't.
**Stresses:** counterfactual explanations. **Status:** OPEN (Q-23: show the failing interleaving step by step).

### TB-061: "I don't even know what to ask"
**Stresses:** learning UX. **Status:** OPEN (proposed answer: guided walkthrough mode. Pick a concrete action, step through it in small excerpts, and supply the next question; Q-13).

### TB-062: An earlier explanation turns out wrong
Step 9 discovers that a helper swallows errors, contradicting step 6's explanation of the error path.
**Stresses:** the journal's honesty. **Status:** OPEN (proposed answer: the earlier entry gets a visible correction linking to the later one, never a silent rewrite; Q-22).

### TB-063: A "simplified equivalent" that isn't equivalent
To teach an iterator chain, the LLM writes an equivalent plain loop, but it changes the semantics (ordering, a missed filter).
**Stresses:** ADR-0002 applied to teaching rewrites. **Status:** OPEN (Q-23: label rewrites as simplified; check them where possible, e.g. by the compiler or by running both on the same inputs).

### TB-064: Explanation of code that later changes
A journal entry explains a function, then the function is refactored.
**Stresses:** drift in explanations. **Status:** OPEN (proposed answer: the entry's receipts break, so it's flagged "may be outdated"; Q-21 + Q-22).

### TB-065: "Why not just make the flag atomic?", shown on the graph
The learner proposes an alternative the code doesn't contain. Insight highlights the real check step and the real claim step under the lock, then plays a **hypothetical** step-through: worker A reads "free", worker B reads "free", A writes "taken", B writes "taken", and both start the same job.
**Stresses:** scenarios vs. ADR-0002. The interleaving never happens in the real code. **Answer:** ADR-0010: scenario overlays are a distinct, always-labeled kind. The real nodes are true; the sequence is marked hypothetical. **Status:** Partial (the scenario format and how to check that a scenario only uses real nodes are open, Q-23). ADR-0011 adds an upgrade path: run the alternative in a scratch copy under a deterministic scheduler, so the race is executed rather than narrated (TB-069).

### TB-066: A prose sentence with nothing to highlight
An explanation contains a sentence that links to no graph element.
**Stresses:** grounding. **Status:** Out of scope (R2-07, ADR-0037: links are optional; nothing is flagged). *Earlier proposal:* (allowed for general concepts, e.g. "a mutex grants one holder at a time", but a sentence that makes a claim *about this code* with no link is flagged; ADR-0010, ADR-0002).

## N. Step-throughs (ADR-0011)

### TB-067: The path touches hardware or the network
Stepping through "save settings to the device" requires a real device that isn't present.
**Answer:** stub the boundary. The stub's return value is shown as *assumed* ("assuming the device replies OK"). Everything downstream is executed with that assumption, which is stated. **Status:** Partial (stub generation, Q-24).

### TB-068: Predicted value ≠ executed value
The AI (or the learner) expects `count == 5` at step 7, but the run gives 6.
**Answer:** the executed value wins and is shown. The mismatch is surfaced as a teaching moment ("you expected 5; here's why it's 6"), never hidden. **Status:** Solved (design).

### TB-069: Stepping through a race
Show two workers interleaving on purpose, with real values.
**Stresses:** deterministic scheduling. **Status:** OPEN (Q-24: deterministic schedulers; the alternative design runs in a scratch copy).

### TB-070: The AI's scenario misses its target
Asked to show the `!valid` path, the AI picks inputs that turn out to be valid.
**Answer:** the test step catches it, because the run didn't reach the target. The AI adjusts the inputs before handover. An unverified step-through is never shown. **Status:** Solved (design).

### TB-071: The project doesn't build
A half-broken vibe-coded app can't run, so no harness can execute it.
**Stresses:** fallbacks. **Status:** OPEN (proposed answer: fall back to the deterministic interpreter for the reachable pure parts. Otherwise offer a *scenario* (hypothetical, labeled) with no values, never invented ones; Q-24, TB-029).

### TB-072: Running the user's code is dangerous
The harness would call code that deletes files or sends requests.
**Stresses:** sandboxing. **Status:** OPEN (Q-24: effects (Q-20) tell us which boundaries to stub before running).

## O. Oversight of AI-written code (ADR-0012)

### TB-073: An agent quietly adds a network call to the save path
A coding agent "improves" save by syncing to a cloud endpoint. The line diff is 40 lines across 3 files and looks plausible.
**Stresses:** system diff. **Status:** OPEN (proposed answer: Q-25 surfaces "new effect: network on path Save → …", highlighted).

### TB-074: An agent swallows an error
A `catch` that used to rethrow now logs and returns `undefined`. Errors no longer reach the UI.
**Stresses:** error-edge reachability in the diff. **Status:** OPEN (Q-25; an expectation "every backend error reaches the UI" would also catch it, Q-26).

### TB-075: A stated expectation is violated
Expectation: "saving never touches the network". After TB-073's change, the check fails.
**Answer:** the violating path is highlighted (ADR-0010), and the expectation is a deterministic query, so this is not an LLM opinion. **Status:** Partial (query language, Q-26).

### TB-076: A pure refactor
An agent renames functions, moves files, and extracts helpers, with no behavior change. The line diff is huge.
**Stresses:** diff noise. **Status:** OPEN (proposed answer: the system diff reports "no behavioral change" plus a structural summary; Q-25 node identity across renames).

### TB-077: The expectation is mistranslated
"Only the worker pool writes to disk" is translated into a query that also allows a logger's file writes. That's wrong, or at least not what was meant.
**Stresses:** the LLM in the loop for expectations. **Status:** OPEN (proposed answer: the user confirms the query, and Insight shows example paths it accepts and rejects before saving it; Q-26).

### TB-078: The agent forgot the logger on one path
A new command's errors reach the UI but not the logger. Every other command does both.
**Stresses:** seeing absences. **Status:** OPEN (proposed answer: deterministic sibling comparison, "4/5 reach logger"; the count is fact and the suggestion is labeled; Q-25. See `docs/examples/oversight-loop-worked-example.md`).

### TB-079: A gap that is structural, not a one-off
Five handwritten catch blocks each repeat "toast + log", so the next one will eventually forget.
**Stresses:** surfacing design-level causes. **Status:** OPEN (proposed answer: a query over all error paths shows the repeated pattern; the user directs a by-design refactor (Q-27), and the system diff confirms 6/6 afterwards).

### TB-080: An agent later reintroduces the gap
After the error-sink refactor, a new agent adds a command with its own catch block that skips the sink.
**Answer:** the expectation "every backend error reaches UI and logger" (Q-26) fails on that prompt's system diff, before human review. **Status:** Partial (query language, Q-26).

## P. From real code: Fletcher (`docs/examples/fletcher-history-graft-worked-example.md`)

### TB-081: One component, two wirings by window
`HistoryTree.onGraft` is a direct function in the main window and an IPC event (`hist-cmd`) in the pop-out window. The same bundle picks its personality from a URL query parameter.
**Stresses:** per-mount-site prop resolution, config-driven branching (TB-011), links (Q-20). **Status:** OPEN (proposed answer: draw both wirings as edges from the prop, each conditioned on its mount site; sibling comparison flags a command handled in one wiring but not the other).

### TB-082: The same shape declared twice, copied field by field
`HistNode` and `HistTreeNode` are identical, and `treeData` copies each field explicitly. A new field added to one silently never reaches the other side.
**Stresses:** near-duplicate types, data edges. **Status:** OPEN (proposed answer: deterministic shape comparison plus field-reachability; a natural standing expectation, Q-26).

### TB-083: A UI gesture with a system-wide effect
Dropping a node in a canvas rewrites an Equalizer APO config file eight hops later, so the user's audio changes system-wide.
**Stresses:** effect reachability across IPC. **Status:** OPEN (Q-20: effects through links).

### TB-084: Mutable ref + version counter
The domain lives in a `useRef` mutated in place; React learns of changes only via `setHistVersion(v => v + 1)`. Forgetting the bump means a stale UI.
**Stresses:** framework-semantics knowledge in the plugin; explanation; expectation "every write to `hist` is followed by a bump". **Status:** OPEN.

### TB-085: Persistence error swallowed
`invoke("history_save", …).catch(() => {})`: a failed save reaches neither UI nor logger.
**Stresses:** error reachability (Q-25). This is the user's own "forgot the logger" story, in real code. **Status:** OPEN.

### TB-086: A 350 KB single-file UI
The file holds the whole UI. Most of the useful units are components, hooks, and closures nested inside one component (`graftNode` is a `const` inside `MainApp`).
**Stresses:** container depth (ADR-0008), performance. **Status:** OPEN (proposed answer: the file stays a container. It's a real logical boundary and the user's navigation memory ("which file do I open?"), and its visible size/weight is a split-me signal. The TS plugin also goes deep: nested functions and closures are containers inside it).

## Q. Multi-codebase workspaces and the wire (ADR-0014)

### TB-087: Sensor → microcontroller → radio → frontend → display
Three codebases: C firmware, the radio/protocol layer, and a web or desktop frontend. "Show me the trace of a reading from the sensor to the chart, focusing on the processing algorithms."
**Stresses:** wire links, data lineage, lenses. **Status:** OPEN (ADR-0014 decided; Q-28, Q-29, Q-19 lenses).

### TB-088: The two sides disagree on a field name
The firmware sends `{"ax": …}`; the frontend reads `msg.accelX`.
**Answer:** no link is drawn (it doesn't match). The send's `ax` has no receiver and the read's `accelX` has no sender, so both are shown as dangling. That's a visible oversight finding, never a guessed connection (ADR-0002). **Status:** Solved (design).

### TB-089: Binary protocol, no field names
The firmware packs a C struct into bytes; the frontend unpacks by offsets.
**Stresses:** link proof. **Status:** OPEN (proposed answer: proven if both sides derive from a shared header/schema; otherwise inferred by matching layout, labeled; Q-28).

### TB-090: No tracer on the device
The microcontroller can't run Insight's tracer.
**Stresses:** observed data. **Answer:** ADR-0015: the LLM adds tagged log probes to the firmware (scratch copy), the user flashes it and runs the scenario, and the serial/RTT output is parsed onto the graph. Packet capture covers the wire. **Status:** Partial (reflash UX; correlation across devices, Q-28).

### TB-091: "Focus on the algorithms"
The full trace has ~60 nodes. Most are transport and serialization.
**Answer (proposed):** a lens keeps the filter / FFT / smoothing nodes and collapses plumbing to "+N hidden" (ADR-0013 rule 2). The lens's choice is labeled organization. **Status:** OPEN (Q-19).

### TB-092: Processing on both sides
A low-pass filter on the microcontroller, then a moving average in the frontend. The learner should see that the data is filtered twice, and in what order.
**Stresses:** lineage across the wire. **Status:** OPEN (Q-29).

### TB-093: Units change across the wire
The device sends raw ADC counts; the frontend assumes g. Or the device scales, *and* the frontend scales again.
**Stresses:** lineage with units and representation; a classic cross-device bug. **Status:** OPEN (Q-29: show every hop where the value's scale or type changes).

## R. Tracing strategy (ADR-0015)

### TB-094: A probe is left behind
The run finishes, but a probe log line stays in the code and gets committed.
**Answer:** probes live in a scratch copy or branch. Removal is automatic; keeping one is an explicit user choice. A leftover probe tag in the working tree is itself flagged. **Status:** Partial (mechanism).

### TB-095: SQL inside a string
`sqlx::query!("UPDATE notes SET …")` or a Python f-string query. The SQL plugin must parse SQL embedded in another language and attach it as an effect (DB write to table `notes`) with its own lineage.
**Stresses:** embedded languages, effects, lineage into tables. **Status:** OPEN (Q-03, Q-20).

### TB-096: The probe changes the behavior
Adding a log line inside a tight loop or an interrupt handler shifts timing, and the race being investigated disappears (a heisenbug).
**Stresses:** observation honesty. **Answer (proposed):** observations made with probes on timing-sensitive paths are labeled "probed run, timing perturbed". Races are better shown with deterministic schedulers (Q-24). **Status:** OPEN.

### TB-097: Is static really ~99%?
The user's estimate. On Fletcher's `App.tsx`, how many call sites resolve statically? (Props like `onGraft` with two mount sites, `hist-cmd` events, and `useRef` callbacks are the hard cases.)
**Stresses:** the ADR-0015 premise. **Status:** OPEN (measure in the compiler spike / tracer bullet. Each plugin reports its resolution rate).

## S. The MVP question and time (ADR-0018, ADR-0019)

### TB-098: A silent default on the value being asked about
In Fletcher, a filter with no frequency becomes 1 kHz without complaint: `fc_hz.unwrap_or(1000.0)` appears at several sites in `app/src-tauri/src/lib.rs`. Asked "how do we know a filter's frequency is correct?", an answer that lists only the clamp (`clamp(10.0, 24000.0)` in `chain_of`) and the DSP tests (`dsp.rs`) would be incomplete in a misleading way.
**Answer (user ruling):** fallbacks appear as a **neutral fact on their own row** ("Silent fallbacks"), next to Validation and Tests, with receipts. They are never labeled a bug. **Status:** Partial (Q-31 mechanism).

### TB-099: Async drawn as sync
A `.then()` continuation, a `tokio::spawn`, and a direct call are drawn as the same arrow, and the learner assumes the caller waits for the result.
**Answer:** ADR-0018 + ADR-0023: time is always explicit on screen. The solver supplies the connection and the quoted code (`.then(`, `tokio::spawn`, the direct call); the model names the timing kind with that receipt. Unknown timing is shown as unknown, never defaulted to sync. **Status:** Partial (model labeling mechanism).

### TB-102: A state change triggers work nobody calls
`setHistVersion((v) => v + 1)` in `graftNode` → the autosave `useEffect(…, [histVersion, …])` → `setTimeout(…, 400)` → `invoke("history_save")`. No call edge links them.
**Answer:** ADR-0023: the setter and `histVersion` come from the same `useState`, and the dependency array reads `histVersion`, so the solver connects write → read with no React-specific rule. That "it's a 400 ms debounce that restarts on each change" is the model's understanding, citing `}, 400);` and `clearTimeout(t)`. **Status:** Solved (design).

### TB-100: The MVP acceptance question
On Fletcher: **"How does dragging a node onto another node work?"** The answer must match the hand-traced key (`docs/examples/fletcher-history-graft-worked-example.md`): the pointer flow, the two wirings (main and pop-out), `graftNode`, the system-wide audio effect, the data structures by layer and where each lives, timing kinds (ADR-0018), and what assures it (validation, fallbacks, tests, and the gaps, e.g. whether any test reaches `graftNode` at all).
**Stresses:** everything in the ADR-0019 cut. **Status:** OPEN (the key exists; its assurance section is still to be traced).

### TB-101: "How do we know the sensor data's frequency is correct?"
The user's original assurance question, about a sensor stream across devices (ADR-0014). The answer has to tie together: the configured sample rate (requested), what the device reports back, what the timestamps actually show (measured intervals, gaps, drift), where the host checks any of these, and what nothing checks.
**Stresses:** assurance trace across firmware + wire + host; lineage (Q-29); observed data via probes (ADR-0015). **Status:** OPEN. Outside the MVP code scope (needs C/C++ and wire links), but it's the motivating question for Q-31.

## T. Teaching views (ADR-0026)

### TB-103: Regroup a container's fields by role
A state container with ~20 fields. The model presents them as four role groups (core model, feature state, caches, operation gates), none of which exist in the code.
**Answer:** allowed as authored groups over real field nodes, visibly authored. Each field links to its declaration. **Status:** Solved (design).

### TB-104: Loop shown as pseudocode
A 60-line worker loop is shown as one authored node with six lines of pseudocode.
**Answer:** allowed as an authored abstraction, labeled pseudocode, mapped to the real loop's nodes. One click expands to the real code. **Status:** Solved (design).

### TB-105: An authored element with nothing behind it
The model creates a "retry manager" group, but none of the nodes it maps to involve retries, or it maps to nothing at all.
**Answer (R2-02):** groups, collapses, abstractions and excerpts must resolve to real code, so an unmapped one is returned as an error; notes and example data may stand alone. A *mismapped* one is visible, because expanding it shows the real nodes don't match. That's the model being wrong where you can see it, not hidden. **Status:** Partial (mapping check is structural only).

### TB-106: "How is the tree held in memory?"
Fletcher's undo tree is a flat `Map<number, HistNode>` whose tree shape exists only in `parent` / `children` fields. The code graph shows a Map; the answer needs a *tree*.
**Answer (proposed):** an authored **example instance diagram** (ids, links, values, labeled *example*), mapped to `HistNode` and the functions that build it (ADR-0026). Plus the solver's mutation matrix (functions × fields written). See `docs/examples/fletcher-history-tree-model-answer.md`. **Status:** OPEN (tool API needs an instance-diagram element and a before/after frame).

### TB-050: The LLM wants a node for an inline chunk of logic
Forty lines of validation inline in a handler. The LLM thinks it's a distinct step.
**Answer (ADR-0026):** in a teaching view it may create an authored "validation" node mapped to those lines. The fact layer is unchanged. **Status:** Solved (design). *Earlier answer, kept for history:* no invented node. It can be a *block* if it's a control block, or the LLM suggests extracting it into a function, which then appears deterministically).
