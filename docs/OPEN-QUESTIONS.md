# Open Questions

Unresolved design debates. Each gets an ID; when settled, mark `Resolved → ADR-XXXX` and write the ADR. Do not silently re-debate these in chat. Update this file instead.

Source: the original pitch (`docs/archive/2026-10-09-original-pitch.md`) and the kickoff session.

## Current status of every question (authoritative; updated after the R4 round, 2026-10-09; see ADR-0047/0048/0049)

The entries below keep their discussion history. **Where a "Status:" line inside an entry disagrees with this table, this table wins.**

| Q | Status |
|---|---|
| Q-01 | Resolved → ADR-0002, scoped by ADR-0041 |
| Q-02 | Resolved → ADR-0003 (dynamic overlay post-MVP) |
| Q-03 | Resolved → ADR-0016 (set and order); tooling → ADR-0034 / R2-15 |
| Q-04 | Largely resolved by BQ-09/BQ-18 and R2-30. Remaining: how bubble inputs/outputs are drawn (frontend session) |
| Q-05 | Partly resolved: mapping a prompt to entry points is the model's job via SQL (ADR-0035, ADR-0038). Open: pointing at an element in the running app (post-MVP) |
| Q-06 | Closed by ADR-0025 |
| Q-07 | Open, reduced by ADR-0025. Known so far: the build check runs build scripts and proc macros (ADR-0036, BQ-06); builds run only on Insight's frozen copy with approval fingerprints, declared install/check kinds and a tripwire (R3-01, R3-02); secrets: input-set-only `source`, mirrored deny rules, `insight purge` (R3-03); the MCP surface is a local named pipe (BQ-42/43); the signing key never leaves its store (ADR-0046) |
| Q-08 | Partly resolved: analysis engines (ADR-0034); app shell = standalone app on Tauri 2 (ADR-0031, R2-13). Open: rendering and layout library (frontend session) |
| Q-09 | Closed by ADR-0029 |
| Q-10 | Resolved → ADR-0017 + ADR-0025: Insight never edits code (the agent does); it is the instrument beside the editor |
| Q-11 | Resolved → ADR-0012 + ADR-0025 |
| Q-12 | Model cost closed by ADR-0025 (Insight calls no model). Open: build and query latency targets (BQ-25, R2-19). There's no deterministic "assurance inventory" (ADR-0023) |
| Q-13 | Resolved: teaching moves are the model's (ADR-0035); Insight provides Steps and play mode (ADR-0040, BQ-39) |
| Q-14 | Closed by ADR-0025 |
| Q-15 | Resolved → ADR-0043 |
| Q-16 | Resolved → ADR-0004 (lives in INSIGHT.md, ADR-0044) |
| Q-17 | Open, post-MVP; runs are self-check tools for the model (ADR-0015, ADR-0037) |
| Q-18 | Resolved by R2-10/10b and R2-17: snapshots ride the project's own successful builds (agent-agnostic after-build hook, plus Snapshot now); a working copy is exactly what built |
| Q-19 | Resolved → ADR-0026 + ADR-0040 (views pinned to their snapshot, ADR-0029; storage in Q-37). A "lens" is a Claude-composed view; Insight supplies facts and counts only, with no relevance classifier (ADR-0047) |
| Q-20 | Resolved → ADR-0023 + BQ-09/13/14/17 (Round 3) |
| Q-21 | Resolved → ADR-0013, later amended by ADR-0024/0026/0029/0037 and R2-03 |
| Q-22 | Open, post-MVP (journal). Entries pinned to their snapshot (ADR-0029); sharing is low priority (ADR-0025) |
| Q-23 | Resolved: angles are the model's (ADR-0035, ADR-0037); visual angles = ADR-0040 elements |
| Q-24 | Open, post-MVP; amended by ADR-0037 (self-check tools, not a gate). Non-building code is out of scope (ADR-0036) |
| Q-25 | Open, post-MVP. Node matching → ADR-0028 (model reconciles); a diff is two snapshots in SQL (R2-32); prompt baseline → R2-34; presentation → frontend session. The diff returns all fact changes; summarizing "no behavioral change" is Claude's job (ADR-0028/0047), and "effects" are the model's reading (ADR-0023) |
| Q-26 | Open, post-MVP. Query language = SQL (ADR-0038); storage and re-run timing open (with Q-37) |
| Q-27 | Resolved → ADR-0017 (sub-questions: no hosting, ADR-0025; the agent queries mid-task via `sql`) |
| Q-28 | Open, later. Wire links are the generic literal and `names` facts (ADR-0048); there's no per-protocol knowledge. "Same message" is Claude's judgment, shown as a view note or group, never an edge (R2-03, R2-38) |
| Q-29 | Open, later. Def-use reach = one-hop value flow + declaration identity (BQ-13/15), `binds` with modes (R4 P6), `base_nid`; cross-repo via literal and `names` facts (ADR-0048) |
| Q-30 | Resolved → ADR-0021, amended by R2-09 (question box = terminal; layout deferred) |
| Q-31 | Open (MVP output is the model's): only "everything connected" is deterministic; the model sorts it into validation, fallbacks, tests and gaps (ADR-0023) |
| Q-32 | Open, low priority |
| Q-33 | Mostly answered by BQ-45/46/47 + ADR-0037: generic CI fixtures, local untracked testbeds, an oracle report, and Claude tests from a dev session. There's no "rejected claims" metric (no claim checker, ADR-0024/0037) |
| Q-34 | Resolved → ADR-0038; extended by R4-B (`reach()`/`nid()` table functions, shipped views). Effects and timing are the model's (ADR-0023/0042) |
| Q-38 | Open, later: embedded SQL (R4 P13). Insight attaches SQL facts to a string literal only if it parses fully and matches the project's own schema; ask the user when the SQL extractor is built |
| Q-39 | Open, later: concurrency syntax, linkage/FFI, entry points, memory-mapped registers and unions (R4 P12, P14), for Go, Java, C#, Swift and C/C++ |
| Q-35 | Resolved → ADR-0028 |
| Q-36 | Open (frontend session / later) |
| Q-37 | Open (frontend session). MVP: views aren't saved; the app holds the current view (BQ-33), and the loaded projects are restored on restart (R2-22) |

## Q-01: The truth model — **Resolved → ADR-0002**
The graph may be incomplete, never wrong. Code analysis owns the structure; LLM output is verified or labeled as interpretation.

## Q-02: Static vs. dynamic vs. hybrid tracing — **Resolved → ADR-0003**
Hybrid. The static graph is the map, and dynamic runs are highlighted overlays that can also upgrade "possible" edges to "observed".

## Q-03: Language support for v1 — **Resolved → ADR-0016** (the set and the build order; the history below is kept for context)
Architecture settled: languages are plugins (ADR-0007). Embedded C/C++ and protocol plugins are needed eventually for multi-device workspaces (ADR-0014).

**Committed set (user, session 7):** C/C++, Rust, Python, TypeScript (and therefore JavaScript), Go, SQL.

**Suggested additions (Claude, pending user ruling):**
- *Mobile and app front ends*: Kotlin/Java (Android, JVM backends), Swift (iOS/macOS; the phone side of BLE devices), C# (Unity, .NET, Windows tools).
- *Definition languages* (not executed, but they define wire links and make them **proven**): Protocol Buffers, OpenAPI, GraphQL, JSON Schema.
- *Glue that wires processes together*: shell scripts, Dockerfile/compose, CI workflows (YAML).
- *UI markup*: JSX/TSX, HTML, and component templates. Needed to map "click this button" to its handler (Q-05). Covered by the TS plugin for React; needed separately for Vue/Svelte/HTML.
- *Embedded flavors*: Arduino (`.ino` is C++), ESP-IDF/Zephyr/FreeRTOS conventions inside the C/C++ plugin.

**Proposed order:** TypeScript + Rust first (Fletcher tracer bullet, and Insight itself) → Python, C/C++, Go, SQL → mobile languages → definition and glue languages wherever a wire link needs them.

Status: language set committed; additions and order open. One candidate is TypeScript/React plus Rust plus the Tauri bridge, since Insight is itself a Tauri app and could analyze itself. Status: partially resolved.

## Q-04: The graph model
Settled: containers (package › file › type › function › block) and control edges (call, branch, return, throw) (ADR-0008). Still open: which blocks count; how inputs and outputs are shown on a bubble; how recursion is drawn; data and boundary edge families (Q-20). Becomes `SCHEMA.md` when frozen. Status: partially resolved.

## Q-05: Trigger kinds and how each maps to entry points
Possible triggers are a UI element (JSX `onClick` → handler), a CLI command (argument parser → subcommand fn), a function picked in code, or a prompt (the LLM maps natural language to one or more entry points). How does a user point at a UI element: by clicking it in the running app, or by picking it from source? Status: open.

## Q-06: LLM provider access and auth — **Closed by ADR-0025** (no in-app model; the user's terminal agent runs on its own subscription)
Options include bring-your-own API key, Anthropic and OpenAI account sign-in for third-party software (the user reports both recently opened something like this, but it hasn't been researched or verified yet), and local models. Status: research pending (user: "we'll do more research about this later").

## Q-07: Privacy, security, and code leaving the machine
The user's code goes to a cloud LLM. Do we offer local-only mode, redaction, or per-project consent? Also covered here:
- secrets in repos reaching prompts;
- step-throughs and probes *running* user code (TB-072);
- the MCP surface letting agents drive Insight (ADR-0017).

Status: open.

## Q-08: App stack
Tauri 2 is proposed (Rust core, web UI). Also open: the graph rendering library (React Flow, Cytoscape, ELK for layout, Sigma/WebGL for scale, a custom renderer), and the analysis engine (tree-sitter, rust-analyzer, TS compiler API). Status: proposed, not ruled.

## Q-09: Freshness and incremental updates — **Closed by ADR-0029** (a graph is an immutable snapshot of one commit; nothing is kept in sync)
The code changes after the graph is built. How do we re-trace incrementally, keep labels from going stale, and keep the user's notes and annotations attached to code that moved? Status: open.

## Q-10: Read-only explorer, or a real IDE?
"This is the new IDE" and "you would be able to add more to it." Does Insight edit code (for example, "add a branch here" → an AI writes it), annotate the graph, or both? Does it replace the editor or sit next to it? Leaning (ADR-0012): "debugging and IDE for AI-first written code". It's where you direct and verify agents at the system level, beside or above a text editor rather than replacing line editing. Status: open, leaning IDE.

## Q-11: Who is it for?
**Resolved by ADR-0012 + ADR-0025:** one person, the author, learning (~60%) and overseeing AI-written code (~40%) on their own codebases. Teams and onboarding are out of scope.

## Q-12: LLM cost and latency
Labeling every edge and bubble in a large codebase is expensive. Options: lazy labeling (only what's on screen), caching by content hash, cheap models for labels and strong models for chat. **Latency:** the user wants highlights to be "instant". Deterministic answers (graph, queries, assurance inventory) can be; LLM prose streams after. Set a budget per answer type. Status: open.

## Q-13: What "learn iteratively" means as a UX
Options: chat grounded on the selection, guided walkthroughs of a flow, quizzes and "predict what happens", remembering what the user already understands, progressive disclosure. **Evidence:** `docs/research/2026-10-learning-session-patterns.md`, from a real multi-day learning log. It identifies 16 question shapes and 10 teaching moves that worked. The strongest finding: "follow one action in small steps, and the AI supplies the next question" beat subsystem overviews, for a user who said "I don't even know what questions to ask." Status: open, now evidence-backed. See also Q-22 and Q-23.

## Q-14: Distribution and business model — **Closed by ADR-0025** (a personal tool, Apache-2.0 per ADR-0020, no business model)

## Q-15: A defined annotation format, plus a "compiler" that removes hallucinations — **Resolved → ADR-0043** (`@insight <keyword>: <value>`, parsed mechanically)
User idea: a clearly defined documentation style that is generated automatically ("like doxygen but way better"), then run through a compiler that removes hallucinations. The proposed shape (not ruled) splits every annotation into two parts:
- **Checkable claims**: structured statements such as `calls: validate, write`, `returns Err when: !valid`, `writes: fs`, `reads: config.theme`. The compiler checks each one against the static graph, and a claim that fails is rejected rather than displayed (ADR-0002).
- **Prose**: free-text explanation that can't be fully verified. It must cite the claims and source spans it relies on, and it's displayed as interpretation. A cheap check flags any identifier mentioned in the prose that doesn't exist in the code.

Settled: annotations are committed repo files, possibly inline like Doxygen (ADR-0005), and humans can write them too. The LLM writes them through the tool API (ADR-0006).

Still open:
- ~~Inline vs. sidecar~~: resolved → ADR-0009 (inline first, sidecar optional). Open: precedence when both exist.
- Density (proposed in Q-21 rule 6): annotations hold only intent/why and gap-filling claims, never restated structure.
- The claim vocabulary: which claim kinds exist (calls, branches/returns-when, reads/writes, owns/borrows, side effects…).
- What happens to an annotation when its code changes (re-verify, then mark stale; see Q-09).

Status: partially resolved. The format is open and high priority.

## Q-16: "Levels": what the user already knows — **Resolved → ADR-0004**
Reading (a): a free-form knowledge profile the LLM interprets. Reading (b), abstraction zoom, belongs to Q-04 and Q-19.

## Q-17: Capturing a dynamic run ("this point")
ADR-0003 needs a way to record a run and to define "this point". ADR-0011 adds an AI-driven route: the AI generates the run with chosen inputs (Q-24), with no manual clicking needed. ADR-0015 makes **LLM-placed log probes** the default capture mechanism on every platform, microcontrollers included. Options include: record from app launch until the user clicks Stop; a breakpoint-like marker ("trace from here"); attaching to a running process; or recording a test case. Per-language mechanisms include a debugger/DAP, instrumentation via compile-time injection, tracing spans, and the JS runtime's own hooks. Status: open.

## Q-18: Live mode (while developing) vs. after-the-fact mode
*Reframed by ADR-0029:* "live" now just means building a snapshot of the working tree (`commit + dirty`) so you can ask before committing. Open: whether that's automatic on file save or on request.

User: "a faster way to view code while developing, not just after the fact, but we want to support both if possible." Live mode means the graph updates as code changes, which needs incremental analysis and cheap re-verification. Q-09 becomes a core requirement rather than a nice-to-have. Status: open (both are wanted; the question is what v1 does first). User leaning (session 2): "where I'm developing, instantly see code structure", so live mode next to the editor. Also see Q-10.

## Q-19: AI-composed custom views — **Largely resolved → ADR-0026** (a custom view is a model-authored teaching view; remaining: storage, Q-37)
User: "you can ask the AI to show this and it can make a custom view with that hierarchy. It can have code snippets." Example request: "show me how clicking Start reaches the app's core state hierarchy, and how this function gets its members from there." The LLM composes a view using the tools (ADR-0006): it picks the nodes, the hierarchy, and what's collapsed, and embeds code snippets. Open:
- Are views saved and committed like annotations, or temporary?
- Do views update when the code changes?
- Are code snippets live excerpts (always current) or copies?

The user marked this as tentative ("not 100% sure, just giving thoughts"). Status: open.

**Lenses** (session 7): "…and focus on the algorithms that do data processing of XYZ." A lens is a focus rule applied to a view. It emphasizes nodes that *transform* the tracked data (math, loops over samples, filters) and collapses plumbing (serialization, transport, UI glue) to "+N hidden" (ADR-0013 rule 2). ~~Deterministic signals can support the classification (touches the tracked field, arithmetic density, loops over buffers).~~ *(Superseded by ADR-0047: Insight supplies facts and counts only; deciding what's "algorithm vs. plumbing" is entirely Claude's.)* The "algorithm vs. plumbing" label itself is organization (prose).

## Q-22: The explanation layer and the learning journal
User, session 4: "we need lots of annotation and stuff like answering the question of 'what does this do', and then if you still don't get it a different way." Proposed shape:
- **Explanations are abundant**, generated on demand per user (ADR-0004), attached to graph nodes, and **anchored with receipts** like annotations. When the code under them changes, they're flagged "may be outdated" (Q-21 drift machinery).
- **A learning journal** records each question in the user's words, the confusion, the answer, how we got there (including corrections), where it lives (receipts), and what's still open. This is the format of the user's own Q&A notes. It's personal and doubles as the evidence the knowledge profile grows from.
- **Annotations ≠ explanations.** Annotations are minimal, shared, and committed (Q-21 rule 6). Explanations are many, personal, and kept in the journal.

Settled (session 5): explanations **can be shared** (user: "it could be sharable"), and each explanation is view state + prose (ADR-0010).

Open: the sharing mechanism (committed to the repo, exported file, or link), and whether a shared explanation is re-personalized for the reader's profile or shown as written. Where does the personal journal live (app data, or a private file next to the repo)? Status: partially resolved.

## Q-23: The "explain it differently" ladder
If an explanation doesn't land, Insight tries a *different angle*, not the same thing louder. Candidate angles from the learning log:
- a concrete example with labeled illustrative values
- a simplified version of the code, then the real code
- an expansion (`?` → `match`, chain → loop)
- a step back to a map of the names
- a bridge from a known language
- the counterfactual ("what breaks if…", shown as an interleaving)
- the design rule behind the choice
- a diagram or custom view
- **visual angles (ADR-0010)**: a different focus on the graph, a step-through of a sequence, or a side-by-side of the actual design and the proposed alternative as a *hypothetical scenario*

Open:
- How does Insight pick the next angle? From the profile, from what's already been tried (the journal), or by asking the user?
- How are teaching rewrites kept honest? A "simplified equivalent" written by an LLM could be subtly wrong (TB-063).

Status: open.

## Q-24: How a step-through gets its values
ADR-0011 requires values from execution or deterministic evaluation, never LLM prediction. Candidate mechanisms, probably all of them, chosen per case:
- **Generated harness**: the AI writes a small test that calls the entry point with the chosen inputs, and the run is recorded (tracer per plugin, ADR-0007). Real and exact, but needs the code to build and run.
- **Existing tests as scenarios**: the project's own tests are ready-made step-throughs.
- **Deterministic interpreter / symbolic evaluation** over the fact graph for pure logic. No build needed, but limited on I/O, FFI, and macros.
- **Stubs at boundaries** (network, hardware, time, randomness), each one labeled *assumed*.
- **Deterministic concurrency schedulers** (model checkers that control thread interleavings) to replay a specific race on purpose.
- **Log probes** (ADR-0015, the default): the LLM inserts tagged log statements in a scratch copy, the scenario runs, and the logs supply the values.

Open: sandboxing (running user code is a security boundary), how vibe-coded projects that don't build are handled (TB-029), and the cost and latency of "test before handover". Status: open.

## Q-25: The system-level diff
For oversight (ADR-0012): compare the fact graph before and after a change (an agent session, a commit, a PR) and show *behavioral* differences:
- added or removed paths from a trigger
- new or removed **effects** ("Save now makes a network call")
- error edges that no longer reach the UI
- new shared state or locks
- new links across languages

~~Pure refactors (renames, moves, extractions with the same behavior) should collapse to "no behavioral change" so the signal isn't buried.~~ *(Superseded by ADR-0028/0047: the diff returns all fact changes, and Claude reconciles and summarizes.)* Open:
- How to match nodes across versions (identity through renames).
- How to tell "same behavior" apart from "different".
- How to present it on the graph (ADR-0010).

Session 6 (user): **the unit of review is a prompt**: "you give it a prompt and then you can see what it did". So the diff is scoped to an agent turn or session and linked to its prompt, which also serves as the intent behind everything in it. Also new: **deterministic sibling comparison** to surface absences ("errors reach the logger in 4 of 5 commands; export_notes is the exception"). The count is fact; "probably a bug" is labeled suggestion. Worked example: `docs/examples/oversight-loop-worked-example.md`.

Status: open, core to the oversight purpose.

## Q-26: System-level expectations
The user states what the system must do in plain language, for example "saving a note never touches the network", "every backend error reaches the UI", or "only the worker pool writes to disk". The LLM translates each one into a **deterministic query over the fact graph**. The user confirms the translation (it's shown as a query, not trusted blindly), and the query re-runs on every change, like a test. A violation is shown on the graph with the offending path highlighted.

This is the "make sure it does what we want at the system level" piece. Open:
- The query language.
- Expressiveness (effects and reachability are easy; ordering and "eventually" are hard).
- Where expectations live (committed next to annotations, ADR-0005).
- Whether step-throughs (ADR-0011) can act as executable expectations.

Session 6: **ad hoc queries graduate into expectations**. "Show me every error path that could skip the logger" asked once becomes "every backend error reaches the UI and the logger", checked forever. One query language serves both.

Status: open, core to the oversight purpose.

## Q-27: Directing the agent from the graph — **Resolved → ADR-0017** (agents live outside; Insight is their MCP tool and the human's window; embedded agent optional later)
In the oversight loop, the user spots a problem on the graph and tells the coding agent to fix it "by design". Insight should hand the agent the selection (nodes, paths, query results, violated expectations) as context, ideally over MCP (ADR-0006), so the instruction is grounded in the graph instead of retyped. Open:
- Does Insight host the agent conversation, or hand off to an external agent (Claude Code, Codex, …)?
- How is the agent's result linked back to the prompt for the next system diff?
- Can the agent query Insight itself mid-task, e.g. "what paths does my change affect?"

Status: open.

## Q-28: Wire-link matching
*(Amended by ADR-0048: there are no protocol plugins. Wire links are the generic literal and `names` facts; "same message" is Claude's call.)* Originally: ADR-0014 needs protocol plugins that pair *send* with *receive* across codebases. Open:
- **Matching keys per protocol**: JSON field names, endpoint paths, BLE service/characteristic UUIDs, MQTT topics, binary layouts.
- **Binary protocols with no names** (packed structs). The link is proven only via a shared header or schema, otherwise inferred from layout (TB-089).
- **Versioning**: firmware v1.2 vs. app v3. Which pair is "the" link?
- **Correlating observed messages** across devices for overlays (clock skew, no shared ids).

Status: open.

## Q-29: Field-level data lineage
"Show me the data from the sensor to the display" asks about a *value*: read → filter → scale → pack → send → parse → smooth → plot. Lineage follows def-use chains within each codebase and JSON or field mappings across wire links. Open:
- How far static def-use can go (through arrays, structs, closures).
- How to show where units and representation change (raw counts → g, int16 → float), which is a classic bug site (TB-093).
- How lineage interacts with the step-through (real values at each hop, ADR-0011).

Status: open. It extends Q-20's "value flow" row from "partly" to "required".

## Q-30: The first five minutes (home screen and cold start) — **Resolved → ADR-0021** (the whole system plus a question box)
What you see after loading a repo, before asking anything. User: "yeah, there needs to be things." Candidate: a **table of contents of triggers** (every UI element with a handler, CLI command, route, `main`, scheduled job), plus the container map with sizes (ADR-0008 clarification), plus "suggested first questions". Also open: indexing time on large repos and what's shown while it runs. Status: open.

## Q-31: The assurance trace ("how do we know X is correct?")
Part of the MVP (ADR-0019). For a value or algorithm, collect deterministically:
- validation points (clamps, asserts, type constraints);
- **silent fallbacks** (defaults, `unwrap_or`, `?? 0`, swallowed errors);
- tests that reach it, and what they assert;
- what is *not* covered.

~~The LLM explains; the inventory is fact.~~ Under ADR-0023, the solver returns everything connected to the value, and the model sorts it into validation / fallbacks / tests / gaps, citing each connection. Ruled (session 8): fallbacks are a neutral row, never labeled a bug (TB-098). The motivating case is sensor data, "how do we know the frequency is correct?" (TB-101). Open: how "a test exercises this value" is established (static reachability from test functions vs. coverage from a probed run, ADR-0015); how to present gaps without implying bugs. It's the read-only ancestor of expectations (Q-26). Status: open, MVP-critical.

## Q-32: What worked in similar tools (narrow prior-art pass)
The user is confident Insight can be much better and isn't interested in competitive analysis. Scope this only to *techniques that worked*:
- Sourcetrail's navigation UX;
- CodeSee's review maps;
- Swimm's code-coupled docs and drift detection (closest to receipts);
- CodeQL/Joern/Glean/Kythe query and index designs;
- LLM repo-wiki generators.

Status: open, low priority. Do it when designing the matching feature.

## Q-33: Evaluation harness
How we know Insight's answers are right and getting better. Gold set:
- the Fletcher keys (drag-graft, ADR-0019's frequency question);
- the other worked examples;
- generalized questions from the user's learning log.

Metrics: correct receipts, no rejected claims shown, coverage of the key's points. It plays the same role as HTT's retrieval harness. Status: open.

## Q-34: The graph query language — **Resolved → ADR-0038** (SQL over per-snapshot SQLite)
The hidden backbone: lenses, sibling comparison, "every error path that skips X", the system diff, expectations, and the assurance trace are all graph queries the LLM translates into and the engine runs deterministically. It needs reachability, effects, timing kinds (ADR-0018), and data lineage. Candidates to study: Datalog, CodeQL's QL, Cypher. Status: open, core technical bet.

## Q-35: Node identity across versions — **Resolved → ADR-0028** (identity = location, rebuilt every build; the solver emits a structural diff; the model reconciles renames and moves. Only matters once things are saved; not MVP)
The system diff, receipts, annotations, and journal entries need "the same function after a rename or move". Candidates: content plus structural fingerprints; the receipt quote itself; git rename detection. Status: open, foundational for SCHEMA.

## Q-36: Editor sync
"Beside the editor" needs two-way sync with RustRover / VS Code: clicking a node opens the code, and the editor cursor highlights the node. Probably small per-editor plugins, or the LSP protocol. Status: open.

## Q-37: Where teaching views live
ADR-0026 views (groups, abstractions, simplified code, annotations) need a home. Options:
- ephemeral (gone when the window closes);
- saved locally per repo, outside the repo (app data);
- saved in a git-ignored `.insight/` folder inside the repo;
- committed (like ADR-0005 annotations).

Also: what happens to a saved view when the real code it maps to changes (broken mappings get flagged, like broken receipts). Status: open.

## Q-38: Loading an API's documentation next to the code
User (2026-10-10): "We should be able to load not a repo, but an API doc or something, and Claude can make those fake nodes for the API endpoints … so we can see how our code interacts with the API. 100% needs to work. I give it my codebase and an API doc. I should see the API calls and responses and everything from that side." (TB-152)

Claude's proposal, not yet decided. There are two cases, split by whether the doc is machine-readable:
- **Machine-readable spec** (OpenAPI/JSON Schema, GraphQL SDL, protobuf/gRPC, AsyncAPI, a C header for a device protocol). A spec has its own grammar, so under R4-02 it is a *language*.
  - An extractor turns it into real facts:
    - each endpoint/operation becomes a declaration;
    - request and response bodies become types with fields;
    - the doc's line ranges become receipts.
  - Code links to it with generic, exact link rules. A URL or route literal in the code matches an endpoint's path template (`/users/{id}` ↔ `` `/users/${id}` ``). An operation name or message type equal to a declared name is the existing `names` link.
  - These are facts, styled as real.
- **Prose documentation** (an HTML/PDF/Markdown page). Nothing deterministic can be extracted, so Claude reads it and creates **abstraction nodes** for the endpoints and payloads (ADR-0040).
  - They are styled as Claude's interpretation (R2-02).
  - Each carries a quote from the doc as its receipt, and links to the code's real call sites.
- Either way, the API's own behavior (what the server does with a request) is outside the analyzed code. It is shown as the API's declared contract, never as traced code.

Status: open. To be decided when it's built; it needs the view work (abstraction nodes) for the prose case.

## Q-20: Data and ownership edges, not just control flow
The user's example asks how a function *gets its members*, which is a data question: the path from framework-managed state through a lock into a core struct, then a nested model, then a field. A call graph doesn't show that. Candidate second edge family: containment/ownership (struct A owns field B), access paths (`state.core.lock().model.items[id]`), and injection (the framework hands a command its state). Open: is this part of the same graph, or a second view? Which data edges can be proven statically (ADR-0002)? A catalogue of everything code "has" (data, types, concurrency, links, effects, meta), with a proposed v1 cut, is in `docs/research/2026-10-what-code-has.md`. Status: open; the user asked for guidance ("I don't know what I don't know").

Evidence from real code (Fletcher worked example, session 7): "separate the data structures for UI and what not" needs two data facts beyond the catalogue: **crosses-boundary** (a value is emitted, invoked, or serialized) and **persisted** (it reaches disk). With *lives-in* (which container owns the state), these classify state as domain / transport / UI-only deterministically.

**Ruled (session 8): state is connected at field/path level.** `hist.current.current`, `hist.current.nodes`, and a node's `.children` are separate pieces of state. The solver follows aliases (`const h = hist.current`), so a reader of `.current` connects only to writers of `.current`. This is still purely mechanical (ADR-0023).

**Superseding ruling (ADR-0023):** the solver only *connects*: every call and every read/write of every piece of state, plus same-source bindings and exact-match links. "Effect", "persisted", "crosses boundary", "UI-only", timing meaning, and the effect catalogue are all **model understanding** over those connections, not solver facts. The catalogue in `docs/research/2026-10-what-code-has.md` is now a list of things the *model* should recognize, not things the backend must classify.

**Earlier ruling (session 8), consistent with the above:** *persisted* is **one fact with its location**: "saved elsewhere on disk at X". There's no fact-level split between the app's own storage and another program's config. Whether a write is "Fletcher's own history file" or "Equalizer APO's config that changes system audio" is for the model to explain from the location (user: "it's up to the model to explain this"). The same principle applies throughout: facts stay minimal and precise, and interpretation is the LLM's job (ADR-0022). For example, "no tests reach `graftNode`" is a plain fact row, and whether to suggest writing one is the model's call.

## Q-21: How much power the LLM has — **Resolved → ADR-0013** (rules accepted; mechanisms unproven, Phase 1 spike)
*The discussion history below is kept for context. ADR-0013 is the normative version.*
User, session 3: the LLM must be able to *add* things, not just organize, but every addition has to be checked deterministically, and code is the source of truth because the real danger is doc-to-code drift, not LLM mistakes. Proposed wording, pending user confirmation:

1. **Code is the only source of truth.** Docs, annotations, labels, and groupings are all views of the code and must be re-checkable against it at any time.
2. **Taking away is free, as long as it's visible.** Hiding, collapsing, grouping, and abstracting can only make the picture *incomplete*, never wrong, and you can always expand back down the hierarchy to the code. Refinement (session 4, TB-056): an *invisible* omission can still create a false belief, so every collapse or filter shows what's hidden ("+11 more fields").
3. **Adding needs a receipt (an anchor).** The LLM can add an edge or claim, but only by pointing at the exact code that proves it (file + span). The compiler goes to that spot in the *current* code and checks deterministically that the thing is there. No receipt, or a receipt that no longer matches, means it isn't shown.
4. **Graph shape follows code shape.** Nodes are the code's own constructs (ADR-0008), never invented. If a step deserves to be its own node, it should be its own function *in the code* (the LLM may suggest that refactor). The graph then picks it up deterministically.
5. **Drift becomes a broken receipt.** When code changes, every anchor pointing into the changed spans is re-checked automatically. Annotations can't silently go stale.
6. **Never write down what the code already says.** (User concern: otherwise every line gets documented, "beyond extra".) Structure is computed, never written. Explanations are generated live per user (ADR-0004) and never committed. Committed annotations hold only (a) **intent/why** that only the author knows and (b) **gap-filling claims** that analysis can't derive. Most functions get no annotation at all. The compiler lints redundant claims ("already a proven fact, remove"). Less duplication also means less surface for drift.

Middle tier: where the compiler can prove the call site and the target both exist but not the connection between them (a callback, dynamic dispatch), an anchored LLM addition is shown as **inferred**. It is visually distinct from proven and observed edges, which stays honest under ADR-0002.

Scope note (session 4): rule 6 limits *committed annotations*. It does not limit *explanations*. The user wants lots of explanation ("what does this do", then "a different way" if you still don't get it). That's a separate, abundant layer (Q-22).

If confirmed, this becomes the next free ADR, superseding ADR-0006. (ADR-0010 and ADR-0011 were taken in the meantime.) Worked example: `docs/examples/save-note-worked-example.md`. It surfaced: receipts as code quotes rather than line numbers (TB-051), prose citing claims so it inherits staleness, grouping ≠ inventing, and direct vs. `via` claims (TB-052). Follow-up from the user ("why is c6 inferred exactly?"): the tier is a property of the *checker*, not the claim. Inferred usually means a global "this is the only value" fact that the plugin can't prove. Receipts can be chains of quotes across files (TB-053). Status: user agrees in principle ("yes I think that makes sense"); sign-off pending review of the worked example.
