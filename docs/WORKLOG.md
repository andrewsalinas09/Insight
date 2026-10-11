# Worklog

Append-only session journal, newest entry first. Every agent session ends by adding an entry.

Template:
```
## YYYY-MM-DD — short title
**Done:** what actually changed (files, decisions, code)
**Learned:** anything non-obvious discovered
**Next:** the single most useful next step
```

---

## 2026-10-11 (cont.) — Condensed views: subsets, groups, collapse (ADR-0040)
**Done:**
- The user asked for a flow on their desktop-app project, overview first. On the full map the overview step opened huge files and showed "nothing". The user: "I think this is a good time for you to make condenced views … You should eb able to remove stuff temporarily right? I gave you that ability?" They had: ADR-0040's subset, groups and collapse, unbuilt since the window began (reported as drift then).
- `show` takes `nodes` (only these are drawn, in their real boxes), `groups` (Claude's named boxes over real code, nestable) and `collapse` (a group's name or a code reference, drawn folded). Every box that lost children says "+N hidden" (closing R2-11 for condensed views); double-click brings them back, and "Whole system" leaves the subset. Lines run only between chosen things.
- Line citations outside any function (a constant) now go to the innermost thing holding the lines; data opens its data trace from the map too. The user had clicked one and nothing happened.
- The legend starts hidden for a condensed view.
- Docs: FRONTEND-QUESTIONS (FQ-13, R2-11), MCP-INSTRUCTIONS point 9, ARCHITECTURE, ROADMAP.

**Learned:**
- ELK gives a line's coordinates relative to the lowest box holding both its ends. A condensed view nests by groups rather than the real tree, so lines are placed by the view's own nesting.
- A flow with no call between its halves (a click that only schedules, and workers that poll) is invisible to `path`. The overview has to say so, and a condensed view shows the gap.

**Next:** the user's verdict on the condensed overview; abstraction nodes, code excerpts and example data (the rest of ADR-0040).

**Later:** the user pressed Whole system and had no way back. Back now restores the condensed view and step; the button reads "Back to Claude's view" while out. Tested by clicking in the window. The click script needed DPI awareness: screenshot pixels are physical pixels.

## 2026-10-11 — Aside views; Claude's first review for a race
**Done:**
- The user asked mid-walkthrough (on step 4 of "how data gets sent over BLE") why a task takes a mutex, and whether there's a race. They also asked for a way to pause the walkthrough, see the answer, and return "nothing changed" (ADR-0055).
- `show` takes `aside: true`. The server attaches the view underneath and the user's step (`under`); the window shows "◂ Back to … · step N", restores that view and step, and tells the server (`restore_view`). Every view change is also one ↶ Back away.
- Notes render **bold** and *italics*. A long trace trail clips its outer frames instead of wrapping over the code.
- Claude answered with a cited six-step aside. A completion notification doesn't say which transmission finished, so a late completion after a timeout, or an error from a failed connection, can leave a stale notification. The next send then frees its buffer while the BLE stack still points at it: a likely use-after-free. This came from reading the code. Confirming it on hardware and fixing it are the user's.
- Docs: ADR-0055, FRONTEND-QUESTIONS (FQ-12), MCP-INSTRUCTIONS point 9, ARCHITECTURE.

**Learned:** state that must survive the window restarting or missing an event belongs on the server. The first aside lost its link because the window started after the walkthrough was sent.

**Next:** the user's verdict on the aside, then the review workflow it suggests: Claude proposing checks, with the window as evidence.

## 2026-10-10 (cont.) — Details panel, data traces, Back history; libraries start closed
**Done:**
- **The user tried the trace view on their own project** ("I absolutly love the step in and see on the left and right and arrow key control"). Three gaps: data was a dead end, navigation had no way back, and each thing showed only its name. They asked for a pop-out details panel. All three built (ADR-0053).
- **Home view:** the libraries start as one closed box; while the project's code sits in a single box, home opens it one level further. On a larger project the open libraries column had filled the screen.
- **Bottom bar** floats over the canvas, so hover text can't resize it (the user saw the whole view jump on hover).
- **Claude's job, tested:** the user asked why one allocator function had so many callers. Claude used `sql` and `source` to find the 29 call sites and read the code, then sent a six-step `show` walkthrough mixing the map and traces, with a note per step. The user's verdict is pending. Two counts in the first draft were wrong and corrected before sending (check every number against the query).
- Docs: ADR-0053, FRONTEND-QUESTIONS (FQ-10), MCP-INSTRUCTIONS point 9, ARCHITECTURE, ROADMAP.

**Learned:**
- An early map layout finishing after a trace opened moved the camera away. The map's layout now leaves the camera alone while a trace is open.
- A CSS class the header already used (`.grp`) silently restyled the details lists. The new panel's classes are prefixed.
- Inline `node -e` scripts with backticks get eaten by the shell, twice this session. Payload-as-data, every time.

**Drift from the spec:** a field's type isn't linked to its type's box: that needs `has-type` edges (SCHEMA, Planned).

**Next:** the user's verdict on the walkthrough, the details panel and data traces; then `has-type` in the extractors.

**Later the same session:** citations (ADR-0054, FQ-11). The user asked to hover the pool names in a note and see them light up. Notes take `[label](ref)`, resolved and checked by the server; the window lights a hovered citation up and goes there on click. The walkthrough was rewritten with 43 citations. Validation caught one wrong reference: the pool arrays are defined in the .cpp, not as members of the class. The details panel now stacks above the bar, so long notes push it up instead of hiding under it.

## 2026-10-10 (cont.) — The trace view, hoverable lines, code tied to the graph
**Done:**
- **The user's first session on the app.** They liked the folder grouping and seeing `main` hand off to `run`. But inside a big function they couldn't see where lines went or in what order things happen, and asked for a way to connect the code to the graph. Claude proposed four things; the user: "I say yes build everything" (FQ-08 → ADR-0052).
- **Window:**
  - Calls are teal, so lines and boxes read apart.
  - Hovering a line lights it and both ends and names them in the bar. Clicking it frames both ends.
  - Calls and data uses are marked in a function's code (a tick at the right edge, the call underlined). Hovering one lights what it calls; clicking it opens the trace there.
  - **The trace view** (`ui/trace.js`): one function's code in the middle, callers on the left, its calls, reads and writes on the right, level with their lines, in the order written. Next/prev call, step in and out with a trail you can jump back along, Tab between dynamic targets, Esc back to the map at that function.
- **Backend:**
  - The window graph carries each line's sites (from receipts).
  - `show` takes `trace: {stack, line}`, also per step. `selection` reports the trace position.
- **Picker:** one entry per repo and commit, with its build time (FQ-09). The user had seen the same commit listed over and over. My first fix hid every older-format snapshot, which emptied a project of theirs ("now there's no commit under it"). Now a commit with only older builds still shows its newest, marked "older format, rebuild".
- Docs: ADR-0052, FRONTEND-QUESTIONS (FQ-06 note, FQ-08, FQ-09), MCP-INSTRUCTIONS point 9, ARCHITECTURE, ROADMAP.

**Learned:**
- Receipts already hold each call's line and column (1-based), so ordering calls as written needed no new extraction.
- When a layout snaps the camera into place, a camera flight still running from before overrode it. The snap now cancels the flight.
- Testing the window with simulated key presses and mouse moves fights the user when they are using it at the same time. Stop once they're on it.

**Drift from the spec:** none new. The order shown is the order written, not run (stated in ADR-0052 and the MCP instructions).

**Next:** the user's verdict on the trace view and line hover (FQ-08), then FQ-06 and FQ-01.

## 2026-10-10 (cont.) — The window app: frontend prototypes, then `insight-app`, `show`, layout steering
**Done:**
- **Frontend session:** started `docs/FRONTEND-QUESTIONS.md`.
  - The user tried two clickable prototypes over Insight's own snapshot. The first used circle packing; the second used ELK layered on a canvas, left to right.
  - Their feedback set the zoom-to-open thresholds (open at 15%, close 20% past, with hysteresis; in Settings) and asked for an at-a-glance visual encoding (FQ-06, being judged).
  - Then they asked to build it into the app (ADR-0051, closing Q-08) and for Claude to steer the layout (ADR-0050).
- **`crates/insight-app`** (Tauri 2, `insight-app.exe`):
  - The app owns the backend `Server` and serves `insight mcp` over a named pipe.
  - The window is plain HTML/JS: vendored elkjs 0.9.3 and IBM Plex. There's no frontend build step.
  - It shows the whole system as folder › file › type › function › block boxes, plus fields, statics and libraries, with typed lines. Line kinds can be filtered from the legend.
  - Interaction: zoom, double-click or zoom-to-open, and focus reshaping. The code view is the deepest level.
  - It has a project/snapshot picker.
- **`insight-mcp`:**
  - The server is a public `Server` with a window sink.
  - New tools:
    - `show(view)` validates the view and resolves every code reference, including `{"path": …}` routes. It refuses unknown fields and the ELK options the window fixes. With no window, it keeps the view (BQ-42).
    - `show(step=N)` jumps to a step.
    - `selection` reports what the user picked and which step they're on.
  - The window graph comes from `window.rs`. Folders come from file paths, and a chain of single-child folders is one box, so the grouping is generic for any language and layout.
- **Layout steering (ADR-0050):** `layout.chain` (invisible high-priority ELK edges), `layout.order` (model order per box) and `layout.options` (ELK). The user has ▲/▼ in the inspector and spacing in Settings.
  - These were tested first in elkjs. Chains work at any depth, even against the direction of calls. ELK partitioning works only at the top level, so it isn't used.
- **The bridge (`insight mcp`):**
  - It answers `initialize` and `tools/list` itself.
  - On the first tool call it connects to the app, or starts it, then relays.
  - The pipe accepts a client only after it sends a token from a user-only run file (`<cache>\run\app.json`). It rejects remote clients, and the app creates it as the first instance of a random name.
  - With `INSIGHT_NO_APP=1`, or with no app to start, it serves in-process.
  - `insight install` also copies `insight-app`.
- **Verified end to end:** an MCP client loaded Insight's own snapshot through the app, showed a three-step view with notes and a route chain, and jumped to step 3. `selection` reported the window's step. The window was checked from screenshots, and the headless fallback and error messages were checked too.
- **Docs:** MCP-INSTRUCTIONS point 9 (show, steps, layout, selection); ARCHITECTURE, ROADMAP, OPEN-QUESTIONS, README and CLAUDE/AGENTS status.

**Learned:**
- **On Windows every child process inherits the parent's inheritable handles:** a direct spawn, `cmd /C start` and null stdio all pass them on. An app launched from the MCP bridge then holds the agent's stdio pipes open, and the client never sees EOF. Launching through `explorer.exe` (like a double-click) avoids it without unsafe code.
- **A screen copy of a WebView2 window comes out black:** `PrintWindow` with full-content rendering captures it.
- A CSS `display` rule overrides the `hidden` attribute, so add `[hidden] { display: none !important }`.
- Framing the previous and the current step together zooms out to empty space when they're far apart. Frame the current step only.

**Drift from the spec (reported to the user):**
- **R3-G #6:** the pipe is limited to the current user by a token in a user-only file, not by an ACL, and the processes are not verified. This avoids unsafe code.
- **BQ-44:** the TS types are not generated; the window reads plain JSON.
- **ADR-0040:** groups, collapse, code excerpts, abstraction nodes and example data are not built, and views cover only the `main` repo.
- **R2-13:** CLI commands other than `mcp` still run standalone, and there's no Start-menu shortcut.

**Next:** the user tries the app from Claude Code and judges FQ-06 (the encoding) and FQ-01 (what opens a box). Then the remaining view elements.

## 2026-10-10 (cont.) — Validation on private projects; call-site matching; `insight install`
**Done:**
- **Two-device firmware pair** (private validation set, C++):
  - The gateway builds against the installed ESP-IDF 5.5.3 with a small compatibility header (C++ only, one component compiled). The user chose this because the code is for testing.
  - Setup used only project settings: an install step that generates a header the build needs, toolchain packages, and a literal-path check command.
  - A "where is sensor data created, sent, and stored" question was answered from the graph alone, end to end across both repos.
- **C/C++ extractor:**
  - A call through an object's `operator()` is a call to that object (callable objects).
  - `operator=` is an assignment.
  - A one-argument function returning a reference passes its argument through.
- **Indirect calls:** the candidate walk is one shared `IndirectWalk` used by the solver and the `indirect_why` example, which explains a candidate hop by hop. Aliasing follows only the callee's own assignments.
- **Snapshot key** includes the `insight` binary itself. A stale snapshot had kept an old extractor's over-merged candidates.
- **Cross-repo `path`:** the name and `same` links use hash maps instead of SQL suffix joins (388 s → about 1 s).
- **Private TypeScript + Rust desktop app** (validation): "how does clicking Start lead to a saved CSV, highlighting the binary format and its decoding" was answered with routes. It found three gaps, now fixed:
  - TS: a `catch` clause was walked twice (count mismatch).
  - Rust: calls through `dyn Trait` now list every implementing method as a `possible` candidate.
  - Settings: per-repo `env K=V` for build scripts that need an environment variable. It is approved like any command; a placeholder value is used for validation.
- **Call-site matching** (fact schema 5, DB layout 6):
  - `call`, `binds` and `value-flow/result` edges carry their call's `site`.
  - In `path`, data that enters a function at one call leaves only through that call's result.
  - The search stops at `max_depth`.
  - On the private app, the Start → CSV query went from 1,897 nodes in 114 s to 285 nodes in 3 s.
  - Test: `call_sites.rs`.
- **`insight install`:**
  - copies the binaries to Insight's own `bin` folder;
  - checks Node.js, cargo and libclang by running them;
  - registers the MCP server with Claude Code at user scope (as `insight-dev` for debug builds), or prints the command.

  The README has a quick start.
- **MCP:** every loaded repo has its own deny rules and frozen-source folder. `source` takes a `<schema>:` prefix.
- **Docs:** the view proposal (cross-device flow, focusing on a step in between) is recorded as unconfirmed in `docs/research/view-proposal-cross-device-flow.md`.

**Learned:**
- A shared helper (here an IPC wrapper) joins every caller's data to every other caller's until results are matched to their call.
- Snapshot reuse must be keyed on every program that produced it, the CLI included.
- Real projects need small per-repo environment and install steps, not extractor changes. Claude does this setup quickly from `status`.

**Next:** with the backend usable from Claude Code, either API-description nodes (Q-38) or start the window (frontend questions).

## 2026-10-10 (cont.) — Multi-repo projects: submodule export, cross-repo `path`
**Done:**
- **Export:** each submodule is exported at the commit its parent records, from local git data (`.git/modules/<path>` or a checkout's `.git`). Nothing is fetched. A missing submodule is listed as "not on this machine; run `git submodule update --init --recursive`". This is a proposed amendment to R3-G #11 and awaits the user.
- **Build:** every repo of a project is built. A failing repo keeps its last good snapshot and the refusal comes at the end (R3-20).
- **`path` across repos** (`Options::schemas`):
  - nodes show as `<schema>:<path>`, and a `schema:` prefix narrows an end;
  - equal literals are one node across repos;
  - `names` matches a string in one repo to a declared name in another;
  - new hop kind `same` links identical source files (by SHA-256) node for node.

  All three links are computed at query time from the snapshots' facts, with no link database.
- **MCP `load(project)`** loads every repo's newest snapshot: the first as `main`, the others attached by repo name. The CLI `path` does the same.
- **Test** `multi_repo.rs`: two made-up repos sharing a header and a literal. Routes go write → same code → read, and through the shared literal.
- **Private validation project** (sensor + gateway, newer sensor checkout):
  - The sensor builds (firmware + PC): 3,647 functions, 0 mismatches.
  - The gateway configures with ESP-IDF but stops on its five missing submodules, which is reported as such.

**Then (same session):**
- The export's reuse check now includes the submodules exported, so a tree exported before they were available is redone.
- `path` lists equality links (`literal`, `names`, `same`) under "to review": they are shortcuts, not compiler facts (user).
- `status` shows each project's settings file and every repo's last build result, so Claude can do setup (user: setup is Claude's work).
- The private gateway now gets through configure and compile, but its committed code needs the ESP-IDF version it pins (6.2.0); 5.5.3 is installed.

**Next:** ESP-IDF 6.2 (the user decides), then build the pair and trace a field across repos with `same`.

## 2026-10-10 (cont.) — Data flow for TypeScript and Rust; external leaves are never passed through
**Done:**
- **TS extractor:**
  - **State:** variables, destructured bindings, parameters, class fields and interface/type-literal members, so props are fields. They're named after the file's containers, so functions keep their names.
  - **Reads and writes:** `++`/`--` and compound assignments count as both.
  - **`assign`:** includes destructuring from a property, and object literals into their contextual type's fields.
  - **`binds`:** arguments to parameters, JSX attributes to prop fields, and data to external callees.
  - **`return`/`result`**, including arrow expression bodies.
- **Rust extractor (`flow.rs`):**
  - **State:** `let`/parameter/match bindings and `self`, struct fields, statics and consts. A binding only counts if rust-analyzer resolves it to a local, so `None` in a pattern is not state.
  - **Reads and writes:** including `*p` (pointee) and `a[i]`.
  - **`assign`:** from `let` and assignments, and from struct literal fields.
  - **`binds`:** to parameters (`&mut` = ref) or to external callees.
  - **`return`:** tail expressions and `return`.
  - **Calls through locals and fields** point at the state; the solver's indirect-call pass resolves them (42 in Fletcher, 59 in Insight).
- **`path`:** an external leaf can start or end a route but is never passed through. It stands for every call site of a symbol, so passing through it joined unrelated code.
- **C/C++ firmware build** (private sensor, ARM gcc):
  - `-std` now reaches the gcc query;
  - gcc's extended-float macros are dropped and `__clang__` kept;
  - nested type definitions are walked once;
  - diagnostics carry `file:line`.

  Result: 178 files, 0 clang errors, 0 count mismatches.

**Learned:**
- **Function and variable names can collide** when a function is named after its variable. State has to be named after all of a file's containers, never interleaved with them.
- **External leaves join unrelated call sites.** A `data` route through `Promise.catch` connected two unrelated functions, so leaves must be terminal.

**Next:** multi-repo projects (link layer across repos), validated on the private sensor and gateway. The gateway's build needs its submodules.

## 2026-10-10 (cont.) — Data flow (C/C++), indirect calls, `path` data tokens
**Done:**
- **Fact schema 4.**
  - State node kinds `var` / `field` / `static`. Their ordinals are separate, so adding a variable never renames a function.
  - Edge kinds `read` / `write` / `access` / `binds`, and value-flow subkinds `assign` / `result` / `return`.
- **C/C++ extractor (`flow.rs`):**
  - locals, parameters, fields and globals as state, with their reads and writes;
  - an `access` edge for each pass by non-const reference or `&x`;
  - `assign` for initializers and assignments, `binds` from argument to parameter (or to the external callee) with mode;
  - `return` / `result`;
  - calls through a variable or field point at that state;
  - arguments of virtual calls bind to every override's parameter (possible).
- **Solver, language-independent:** calls through state get `possible` callees by walking function values backward through the data edges, including reference bindings and results.
- **`path`:** new hop kinds `data`, `read`, `write`, `access`. Virtual-dispatch bindings are labeled `possible`.
- Sample test extended. Fletcher, Insight and the sensor all build with 0 mismatches, and the M1 gate passes.

**Validation (private sensor, PC build):**
- A `--follow data` path from the buffer pool's allocate function to the radio-send function's parameter gives a fully receipted route:
  - the acquired packet goes into the simulator's local;
  - into the pipeline's input parameter;
  - through the virtual `process` (possible) into the BLE node;
  - into the send function's parameter.
- 30 calls through function-pointer fields resolve to 67 candidates.

**Learned:**
- **libclang shows templates as written, not instantiated.** `&F<T>::run` with `F` a template parameter resolves to nothing. The flow stops at the write in the template, and the receipt shows it. This is documented in SCHEMA §5.
- **Data reaching a function arrives at its parameter.** So `data` routes end at parameters, states or external leaves. Adding `read` makes functions pass-through nodes, which is coarser and is Claude's choice.

**Next:**
- The same state and data edges for TS and Rust.
- Multi-repo projects, so a producer in one repo and a consumer in another connect. They would link through shared declarations (the same source compiled in both) and through literal links.
- The sensor firmware build, once its SDK and toolchain are available.

## 2026-10-10 (cont.) — C/C++ extractor; number literals; count-gate diagnostics
**Done:**
- **`extractors/cpp` (`insight-extract-cpp`):** libclang (LLVM 23, loaded at runtime) on every `compile_commands.json` the project's check produced.
  - **Compile commands:** cl/clang-cl use clang's cl mode, and clang commands pass through. gcc-style cross compilers are asked for their include paths and macros (`-E -dM -v`), then parsed with `-undef -nostdinc`.
  - **One owner per file:** each file is analyzed by the first translation unit (sorted) that reaches it. Calls resolve across translation units by USR.
  - **Virtual calls** get `possible` candidates: the named method plus every override.
  - **Handoffs:** functions and lambdas passed to external code.
  - **Literals:** strings and integers. **Inactive `#if` regions** are recorded, and **macro-made functions** are marked `generated`.
  - Launcher crate `insight-lang-cpp`. A made-up sample test covers dispatch, handoff, lambdas, `#if 0`, macros and `= default`.
- **Fact schema 3 / DB layout 5:**
  - `NodeFact.generated` and the `nodes.generated` column;
  - node kind `inactive`;
  - `LiteralFact.number`;
  - integer literals in TS and Rust too, with literal nodes `lit:31` shared across languages.
- **Count gate:**
  - C/C++ rules;
  - inactive regions skipped;
  - "unchecked" where the counter's own parse fails (C/C++ only), stated in diagnostics;
  - mismatches list the differing functions' last lines.

  New debugging aid: `cargo run -p insight-count --example functions -- <file> [from to]`.
- **Validation (private sensor firmware, PC build):** 136 files, 3,284 functions, 0 mismatches, 208 unchecked (all in vendored, macro-heavy code). The build takes about 16 s including the check. Fletcher and Insight still pass, and so does the M1 gate.

**Learned:**
- **libclang's per-file skipped ranges only cover a header's first inclusion.** The TU-wide list is needed, because a header included twice (declarations, then implementation) is inactive in one inclusion and compiled in the other.
- **clang synthesizes bodies for `= default` members.** "Has a body" has to exclude defaulted functions.
- **An independent tree-sitter count can't be strict for C/C++.** Unexpanded macros break its parse, and `struct EXPORT_API Name {}` reads as a function. Hence the function-declarator rule plus explicit, reported "unchecked" functions.

**Next:** data-flow edges (state nodes, read/write, argument→parameter binds, function values through variables and fields) across all three extractors, for "where is this created → where is it sent".

## 2026-10-10 (cont.) — M2: `insight mcp`, after a security review
**Security review** (MCP is where outside input reaches Insight). Findings and fixes:
1. **Project and repo names became paths unchecked.** `..\..\x` from an agent could reach folders outside Insight's. Fixed: `paths::check_name` on every project, repo and snapshot path. Snapshot names over MCP must be one of the project's listed files.
2. **A command against a missing project created its folder** (the log was created before the check). Fixed: `Log::create` requires an existing project.
3. **SQL** (R3-G #7):
   - immutable read-only URI open;
   - authorizer allowlist (ATTACH, DETACH, PRAGMA writes, `load_extension` and all writes denied);
   - one statement;
   - a progress-handler deadline;
   - `hard_heap_limit` plus length limits;
   - `max_rows` and `timeout_ms` as parameters under ceilings.

   Snapshots are attached only by Insight, with the authorizer lifted for that one statement. Tested: multi-statement, ATTACH, DELETE, `PRAGMA writable_schema`, `load_extension` and an infinite recursive CTE are all refused or interrupted.
4. **`source`** reads only the snapshot's `files` table, never the disk. `../../Windows/win.ini` and `C:/Windows/win.ini` resolve to nothing.
5. **Secrets (R3-03):** Claude Code's `Read(...)` deny rules are mirrored, from user and repo settings. A covered file is refused by `source`, left out of the frozen-source folder, and makes `files.content` read as NULL.
6. **The frozen-source folder** only accepts relative, normal paths from the snapshot; anything else is skipped. Files are written read-only.
7. **Over MCP, nothing can create or approve a project or change its commands.** `build` refuses unapproved projects (R3-01).
8. **Not applicable yet:** named-pipe hardening (R3-G #6), because there's no app process to talk to. Repo text returned to the agent is untrusted data, the same as any file the agent reads.

**Done:**
- `crates/insight-mcp`: newline-delimited JSON-RPC MCP server, protocol 2025-06-18 (also 2025-03-26 and 2024-11-05). Tools: `status`, `load` (pinned `main`; `alias` attaches), `build`, `sql`, `path`, `source`.
- `insight mcp` subcommand.
- `insight-engine::query`: the hardened SQL, plus the `callers` and `receipt_text` views.
- `path` takes set-valued ends, and either end may be open (forward or reverse reachability). The CLI accepts `--no-receipts`. The M1 gate still passes.
- `docs/MCP-INSTRUCTIONS.md` finalized to describe only what's built, and compiled into the binary.
- SCHEMA §10–11, ARCHITECTURE, ROADMAP updated.
- New Q-38 / TB-152: load an external API's documentation next to the code (user idea).

**Learned:**
- The `immutable=1` URI plus the authorizer gives read-only access with no locks and no writes, and needs no unsafe code. `hard_heap_limit` is a pragma, so it doesn't need FFI either.
- `reach()`/`nid()` as table functions need rusqlite's `vtab`, whose trait is `unsafe`, against our `unsafe_code = "forbid"`. They're deferred; `path` covers reachability.

**Next:** the user registers the server and tries it in Claude Code: `claude mcp add -s user insight -- <repo>\target\debug\insight.exe mcp`. Then the C/C++ extractor (clangd on the project's `compile_commands.json`) and data-flow edges, validated on the user's private C++ firmware.

## 2026-10-10 — M1: edges, link passes, `insight path`; the gate passes
**Done:**
- **Protocol (fact schema 2):** edge, literal and external records, plus the `anonymous` flag. **Engine** layout 4: an `anonymous` column and edge indexes.
- **TS extractor:**
  - runs on a real `Program` + checker per tsconfig;
  - calls resolve exactly; calls through variables, parameters, properties and props get one-hop value flow (BQ-15) with `possible` candidates and an absent candidate;
  - handoffs to external code and JSX;
  - `returns` edges for curried starts;
  - string literals.
- **Rust extractor:** a second pass on rust-analyzer. Callee tokens are descended into macro expansions, so calls inside attribute macros and macro arguments resolve. External leaves, handoffs of closures and fn items, string literals. It runs on a 256 MB-stack thread with the database attached.
- **Solver:** the generic `link/literal` and `link/names` passes, across languages.
- **`insight path`:** explicit follow tokens; `max_occurrences` and `max_depth` constraints; terminal targets; progress rule; no literal round trips; pruning; cycles reported. Specified in SCHEMA.md §10.
- **M1 gate** (`m1_gate.rs`) passes on Fletcher `cd5d6ff`. Dogfood: Insight on itself passes, and `path` works on its own code.
- Dev profile optimizes dependencies, which took the Rust extractor from 297 s to 34 s.

**Learned:**
- With `literal` on, any common string ("main", "") links most of an app. That's true but unreadable, and the R4-B constraints (`max_occurrences`, `max_depth`) plus "targets are terminal" and the progress rule make results compact without any built-in cutoff.
- The `"graft"` literal links the pop-out's `{type: "graft"}` directly to the bus's `p.type === "graft"`, the connection BQ-22 predicted.
- The real code has `pushChain` → `send()` → `invoke("set_fletcher_chain")`. The walkthrough summary skipped `send`, so the gate follows the code.

**Next:** M2: the MCP bridge (security review first, per the build plan), and finalize `docs/MCP-INSTRUCTIONS.md`. Then the planned state, read/write and `binds` edges.

## 2026-10-09 (cont.) — M0 built and passing; SCHEMA.md
**Done:**
- **Cargo workspace:**
  - `insight-model`: the NDJSON protocol.
  - `insight-engine`: SQLite snapshots, atomic, read-only open.
  - `insight-solver`: settings, approval fingerprint, gix commit export, install/check in Insight's own tree with the tripwire, discovery, ingest with protocol and byte checks, cache key, count gate, "not analyzed" diagnostics.
  - `insight-lang-ts` and `insight-lang-rust`: launchers.
  - `insight-count`: tree-sitter.
  - `insight-cli`: `init` / `approve` / `build`, log path plus exit code.
- **Extractors:**
  - `extractors/ts/extract.mjs` runs on the repo's own TypeScript.
  - `extractors/rust` (`insight-extract-rust.exe`) runs on rust-analyzer semantics (ra_ap 0.0.345), with build scripts and proc macros on.
- **M0 result:** Fletcher `cd5d6ff` = 1,936 functions in 24 files, 0 mismatches. Insight on itself passes too.
- **`docs/SCHEMA.md` written** (normative), with Built/Planned status per concept.
- **Drifts reported and fixed with the user:**
  1. Macro-argument functions are now counted. R3-11 says re-parse them; a first cut skipped them.
  2. The Rust extractor uses full rust-analyzer semantics now, not the parser only. User: don't build scaffolding that M1 tears down.
  3. Node kinds now follow BQ-09: `kind='container'` with a `subkind`.
- `extractors/ts` got its own `package.json` and tsconfig, so dogfooding covers the TS extractor.

**Learned:**
- rust-analyzer crates newer than 0.0.345 need rustc > 1.95 or a newer salsa. `unicode-ident` must stay at 1.0.22 to match rust-analyzer's lexer.
- `json!({...})` and `matches!(x, Pat)` arguments aren't Rust expressions, so the independent counter needs a per-piece fallback.
- The cache key must include the DB layout version. Without it, a layout change silently reused an old snapshot.

**Next:** M1: call / read / write / binds / names edges with receipts in both extractors, then `insight path` and the required-hop gate (R3-12).

## 2026-10-09 (cont.) — Round 4: concept coverage, runtime-Claude simulation, leftovers
**Done:** Three reviews.
1. A concept-coverage stress test across TS, Rust, Python, Go, C/C++, Kotlin/Java, Swift, C# and SQL found 14 gaps (P1–P14).
2. A runtime-Claude simulation answered five Fletcher questions using only the specified tools, and found 17 friction points.
3. A leftovers audit found framework remnants and built-in defaults.

User rulings:
- **ADR-0049, lean by default, pedantic on request.** R4-01 is strict: built-ins are external leaves with `origin=stdlib`, and stdlib handoffs are off by default.
- **R4-02:** no RTOS/SDK conventions; Vue, Svelte and `.ino` count as languages.
- **R4-03:** a global INSIGHT.md for prose only.
- **R4-04:** `insight init` writes `tagging = "on-request"`.
- **R4-B** delegated; **R4-C** approved.

Applied (fork):
- **ARCHITECTURE and GLOSSARY:** the P1–P11 concepts, which include timing, suspend/resume, implicit calls, exit kinds, `binds` with modes, inheritance and conformance, generics, macros and inactive code, decorators, and imports/init. Also the tool and schema items: follow tokens as SQL predicates, `call_eid`/`arg_index`, `base_nid`, `via_eid`, `has-type`, open-ended `path` with a DAG result, `reach()`/`nid()`, `race_candidates`, the frozen-source folder, external `origin`, `sql(max_rows, timeout_ms)`, and `tag_coverage(scope, kinds)`.
- **New `docs/MCP-INSTRUCTIONS.md`** (draft).
- **BQ-03:** no framework link crates. Removed BQ-16 `render` and the 200-row cap; labeled the superseded R3-13/R3-14 recs.
- **Living docs:** OPEN-QUESTIONS table refreshed plus Q-38/Q-39; VISION gets the third principle; TESTBED R4 table plus TB-152…156.
- **Drag-graft example:** re-anchored to `cd5d6ff` (it was about 18 lines off), with a note that its effects and timing are the model's answer.
- **ADR status lines:** 0007, 0018, 0019, 0021, 0022, 0034, 0038, 0039 and 0048. ADR-0048 is scoped to the *analysis*, so Insight's own Tauri shell is fine.

**Learned:** The simulation was the most valuable review yet. Cheap extra columns (`base_nid`, `via_eid`, `call_eid`/`arg_index`) turn whole classes of questions into single joins, and open-ended reachability is the tool that discovers effects without any framework knowledge.
**Next:** Build M0.
## 2026-10-09 (cont.) — Round 3: dry run, red team, TESTBED sweep; ADR-0041…0048
**Done:**
- **Three reviews against the cleaned-up docs:**
  - a fresh-session M0 dry run (13 stop points);
  - a red team (17 findings: running repo code without a trust boundary, `npm ci` wiping the live `node_modules`, pipe/SQL hardening, receipt-text DB blow-up, hook storms, cache growth…);
  - a TESTBED sweep (broken cases, gaps, ~45 missing edge cases).
- **Merged into R3 in BUILD-QUESTIONS:** 24 user-visible questions plus 15 internal items; all answered or accepted. Key rulings:
  - R3-01: only init'ed projects build; build commands come only from the project folder, with a fingerprint and an approval diff.
  - R3-02: Insight never mutates the live working copy (frozen copy, install/check kinds, tripwire).
  - R3-03: secrets (input-set-only `source`, mirrored deny rules, purge).
  - R3-04: the hook is only a trigger; Insight runs the check; triggers are coalesced.
  - R3-05: frozen source store; a snapshot is exactly the files read.
  - R3-06: milestones on `cd5d6ff`.
  - R3-07/08: all settings in INSIGHT.md TOML blocks; bad settings refuse; settings in the cache key.
  - R3-09/10: init-then-build; asking via exit code plus `--yes`.
  - R3-11: an independent tree-sitter function counter.
  - R3-12: the M1 gate as required hops.
  - R3-13 → ADR-0047.
  - R3-14…24: accepted as recommended.
- **New ADRs:**
  - ADR-0047: Claude chooses at query time; parameters, not defaults; every fact kept; the BQ-22 cutoff removed.
  - ADR-0048: the solver speaks programming concepts; language knowledge only in extractors; no framework knowledge; the generic `names` link replaces the Tauri link (BQ-21/R2-31 superseded).
- **Applied the sweep:**
  - TESTBED gained an authoritative R3 status table and sections U–Z with TB-107…TB-151.
  - ARCHITECTURE and GLOSSARY were rewritten to current state.
  - ADR status lines were updated.
  - ROADMAP focus = build M0.
- Also recorded: the "maximally correct when cheap" corollary (ADR-0035).

**Learned:** Two recurring Claude errors, now recorded as rules:
- asking the user to choose *defaults* for relevance the model judges at query time (ADR-0047);
- letting framework knowledge creep into the solver (ADR-0048).

The red team's strongest point was the trust boundary: building means running repo code, so an agent must never be able to approve its own builds.

**Next:** Build M0: scaffold the workspace (BQ-03), write SCHEMA.md (BQ-09 + R2-30 + R3-G + ADR-0048), export `cd5d6ff`, run the extractors, write SQLite, and pass the independent function count.

## 2026-10-09 (cont.) — Docs consistency cleanup after the R2 round
**Done:** Applied the consistency audit's stale-text list (approved by the user), reconciled with every R2 ruling (ADR-0041…0046 and R2-01…R2-33).
- **Rewrote the living docs to describe now:** CLAUDE.md/AGENTS.md (build phase, stack, doc map, SCHEMA.md created at M0), README, VISION (goals updated; "What this is not" filled in), ARCHITECTURE (usage, pipeline, division of labor, graph model, snapshots, post-MVP, stack), GLOSSARY (re-sorted; superseded terms replaced; new terms such as snapshot, project, tag, receipt, extractor, causal order, view element), ROADMAP (M0→M2, Phase 0 done).
- **Added authoritative status tables** at the top of OPEN-QUESTIONS (every Q) and TESTBED (every changed TB), so the original entries keep their history.
- **Fixed stale headers in BUILD-QUESTIONS:** BQ-09…22, 33, 35, 37, 42–44 and 46 now show "decided"; BQ-24 notes SQLite.
- **Appended amendment notes to ADR status lines** (0003, 0005, 0007, 0008, 0009, 0012, 0013, 0014, 0015, 0016, 0018, 0019, 0020, 0025). No ADR bodies were changed.
- The proprietary-name check is clean.

**Learned:** With ~46 ADRs amended many times over, status tables at the top of OPEN-QUESTIONS and TESTBED are the only practical way to keep current state findable without rewriting history.
**Next:** The next review round requested by the user, then M0.

## 2026-10-09 (cont.) — Backend question round complete
**Done:** Answered all 47 build questions with the user one at a time, plus the delegated ones. New ADRs: 0031 (standalone app, Claude drives it via MCP, stdio bridge), 0032 (tags are the compiler, opt-in modes), 0033 (language semantics live in plugins), 0034 (each language's best native toolchain via its own API, NDJSON facts), 0035 (the intelligence principle), 0036 (a snapshot must build; working copy allowed; multi-codebase), 0037 (defer to the intelligence; verification is a tool; Claude tests interactively), 0038 (Claude queries the graph in SQL over per-snapshot SQLite), 0039 (fixed tool, free composition, wishes go to the user), 0040 (nine-element view vocabulary). Delegated decisions are recorded in BUILD-QUESTIONS (Rounds 3 and 6, BQ-03/04/07/24 details). BQ-45: Insight never depends on Fletcher; CI uses generic fixtures; real-repo tests are local and untracked, so proprietary code never reaches the public repo.
**Learned:** The user's corrections kept pulling intelligence out of the tool (no cwd default, no --to-effect, SQL instead of fixed queries, no mods, no value policing) and pushing toward reuse of existing tools. ADR-0035 is the rule that predicts them.
**Next:** Build M0: scaffold the Cargo workspace (BQ-03), the TS extractor (TS compiler API) and the Rust extractor (rust-analyzer), the build check (ADR-0036), and SQLite snapshot output.

## 2026-10-09 (cont.) — The backend question round begins
**Done:** ADR-0030: the backend and frontend are separate, and backend decisions are asked one at a time. Three background agents (solver, MCP tool API, architecture/stack/testing) drafted 57 Fletcher-anchored questions with options and recommendations. They were merged and deduplicated into `docs/BUILD-QUESTIONS.md` (BQ-01…BQ-47, seven rounds, most-blocking first), which is added to the doc map. Two places where the agents disagreed are flagged for the user: BQ-05 (TS checker in a Node sidecar vs. Rust-native oxc without types) and BQ-46 (whole-graph goldens vs. targeted assertions). One real conflict between accepted ADRs is surfaced: BQ-38 (example values in steps: ADR-0011 vs. 0019 vs. 0026).
**Learned:** The solver agent found that the field-level state ruling (Q-20) effectively requires a real TS type checker, a consequence nobody had noticed.
**Next:** Ask BQ-01.

## 2026-10-09 (cont.) — Clickable mockup; Fletcher's Console as the visual language
**Done:** The user didn't understand the four views from their text descriptions, so I built a clickable mockup of the history-tree answer (private artifact: https://claude.ai/artifact/XgSXHu6urDiPDzwafaMtRr; 11 steps, real Fletcher code excerpts, example data labeled). v1 was a generic split terminal/window. The user: "the UI should be very very different… similar design language as Fletcher." ADR-0027: adopt Fletcher's Console. v2 is the Insight window full-frame: header (INSIGHT · GRAPH/SEQUENCE/JOURNAL tabs · repo chip · "claude · terminal" chip), a code graph strip (functions → state they write → what reacts), an example-instance strip (Map table + tree, the model's drawing), a labeled-cell inspector with real code, and a docked bottom bar carrying Claude's narration and step transport.
**Learned:** Describing a visual tool in prose fails. Mock it in the real visual language from the start. The bottom-bar slot from Fletcher's A/B bar is a natural home for the agent's voice.
**Outcome:** The user: "Honestly this isn't even close to what I want, but UI doesn't really matter right now." ADR-0027 narrowed to palette + type; the anatomy is marked unvalidated. UI is deferred. Don't iterate mockups until the user brings UI back up.
**Next:** Back to the solver: node identity (Q-35), then the SCHEMA draft.
**Then (2):** ADR-0029. A graph is an immutable snapshot of one git commit; two graphs are two commits; explanations are snapshots pinned to their commit, not carried forward. Closed Q-09, made ADR-0013 rule 6 (drift) unnecessary, and reframed Q-18 (live = a working-tree snapshot).
**Then:** ADR-0028. Identity is the location, recomputed each build; the solver emits a structural diff (git as input); the model reconciles renames and moves. The user asked "why does this even matter?" Honest answer: only for saved references and a readable system diff, neither of which is in the MVP. So keep it trivial.

## 2026-10-09 (cont.) — Dogfood: Claude answers as the model
**Done:** The user asked Claude to *be* the model: "How does the history tree get held in memory, how do you modify it, how would you show me?" Traced Fletcher's `hist` ref, `HistNode`, `rail`, `histVersion`, and all eight mutators (`commitGesture`, `graftNode`, `moveTo`/`undo`/`redo`/`jumpTo`, `deleteNode`, `editNode`, `initHistory`/`installHistory`). Answered in text and designed four views. Recorded in `docs/examples/fletcher-history-tree-model-answer.md`; TB-106 added.
**Learned:** The views the model actually wanted weren't code graphs: (1) an *example instance* diagram of runtime data (a Map that is really a tree), (2) a functions × fields-written mutation matrix (pure solver facts), (3) before/after state frames, (4) invariants as "all writers of X also write Y". So the tool API needs instance diagrams and before/after frames, and the solver's write connections are the backbone.
**Next:** User reaction to the four views; Q-37 (where views live); tool API spec.

## 2026-10-09 (cont.) — Discovery 8f: model-authored teaching views
**Done:** The user: "annotations is huge… let it change hierarchy / abstraction / grouping / smaller simpler code as needed for demonstration. Use the QA notes." Re-read the learning log for the moves it actually used: role regrouping, data-focused re-hierarchy, pseudocode abstraction, simplified excerpts, `?`→`match` and chain→loop expansions, illustrative tables, interleavings. ADR-0026 creates three layers: the fact layer (never invented), **teaching views** (fully model-authored, visibly marked, every element mapped back to real nodes, one click to the real code), and the UI. Scoped ADR-0013 rule 5 to the fact layer, and expanded the tool list. Q-19 largely resolved; new Q-37 (where views live); TB-050 solved; TB-103–105 added.
**Learned:** "Never invent" was right for facts and wrong for teaching. Explanation *is* selective invention (merge, rename, rewrite), and it's honest as long as it's labeled and maps back.
**Next:** User picks the storage for teaching views (Q-37); then the tool API spec and the solver SCHEMA.

## 2026-10-09 (cont.) — Discovery 8e: personal tool, terminal-driven
**Done:** ADR-0025. You ask in Claude Code or Codex in the terminal; the agent answers in text and drives the Insight desktop window through local MCP tools. There's no in-app chat or model calls in the MVP. Built for one person on one machine (open source, not a product for others). Closed Q-06 (no API integration; the agent uses its own subscription, and the user noted API credit allowances aren't much for Opus-class use, which is their statement, unverified) and Q-14 (no business model). Q-11 resolved (the author is the user). The ADR-0021 question box is the terminal.
**Learned:** Each scope cut removes a pile of work: accounts, sharing, an in-app model, provider auth. What remains is sharply defined: the solver, the window, and the tool API.
**Next:** Define the MVP tool set the agent calls; then the solver SCHEMA.

## 2026-10-09 (cont.) — Discovery 8d: Insight is the instrument, not a second brain
**Done:** ADR-0024. The model already understands (it wrote the code; the learning log shows it explains well in text). Insight's job is the graph, the UI, and the tools the model uses to highlight, focus, and step, so the human can see, trace, and follow. Annotation is the model's call. Re-prioritized ADR-0013: receipts stay so highlights land on real code, and the compiler mechanism is optional. The ROADMAP's Phase 1 leads with a UI + tool API spike. Q-20: state is connected at field/path level, with alias following.
**Learned:** The design had been drifting toward verifying the model. The user's point is that the gap is *visual*, not epistemic: you can't hold a system in your head from text alone. Make the instrument excellent; the model is the expert.
**Next:** Specific question on who drives the MVP UI (an external agent via MCP vs. Insight's own question box); then the solver SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 8c: the solver connects, the model understands
**Done:** ADR-0023, the user's sharpest ruling yet: "Anything that requires understanding the code is the model's job… The solver's job is to simply connect everything together." The solver records every function, call, and read/write of every state ("if something can change something, it should be connected"), plus same-source bindings and exact-match links, all with receipts. Timing meaning, effects, validation/fallback, and labels are the model's job. Amended ADR-0018 (the model assigns timing kinds), Q-20 (no effect catalogue; the "persisted/crosses-boundary" facts become model understanding), Q-31 (the solver returns everything connected; the model sorts it), and narrowed the ADR-0013 compiler to structural checks. TB-099 updated; TB-102 (state change → useEffect → debounce) solved by plain write→read connection.
**Learned:** I kept sliding semantics into the backend (timing catalogues, effect lists, React rules) because that felt "safer". The user's line is better. Deterministic *connection* is complete and exact, while deterministic *interpretation* is an endless edge-case list. Never-wrong holds because the solver never interprets and the model always cites.
**Next:** SCHEMA draft for the solver (node and edge kinds are now small: containers, functions, state, call, read, write, bind, link); node identity (Q-35).

## 2026-10-09 (cont.) — Discovery 8b: every function is a bubble; MVP acceptance test corrected
**Done:** ADR-0022: the fact layer is complete (every function, anonymous included, with deterministic names), and the LLM curates what's shown ("LLM first, really solid back end"). Fallbacks in the assurance trace are a neutral row (TB-098). **Corrected ADR-0019:** I had made "how do we know the frequency is correct?" Fletcher's acceptance test, but the user meant *sensor data*. The Fletcher acceptance test is now the drag-and-drop question (TB-100, whose key mostly exists already). The sensor question became TB-101, the post-MVP cross-device target and the motivation for Q-31. The correction is visible in ADR-0019, not silently rewritten.
**Learned:** Don't bind the user's examples to whichever codebase is open. They have several projects in mind at once.
**Then:** The user ruled that *persisted* is a single fact with its location ("saved elsewhere on disk"), and the model explains what that location means. I applied the same minimal-facts / model-interprets principle to test reach ("no tests reach X" as a plain row). Recorded in Q-20.
**Next:** The rest of the Q-20 MVP cut (which effect kinds, how timing kinds are detected in TS/Rust); SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 8: the MVP, explicit time, agents outside, Apache-2.0
**Done:** Reviewed "what am I missing" with the user, which produced four ADRs:
- **ADR-0017:** coding agents (Claude Code, Codex) live outside; Insight is their MCP tool and the human's window. An embedded agent is optional later. Resolves Q-27.
- **ADR-0018:** time is explicit. Timing kinds on every edge (sync / await / spawn / event / poll / wire, plus *unknown*, never defaulted), visible execution contexts, and a first-class sequence (swimlane) view.
- **ADR-0019:** the MVP is one question: "load the software, ask about an algorithm: how it works, its data structures and where they're stored, and how we know it's correct". The acceptance test is Fletcher's filter-frequency question (TB-100). Real key excerpts: `clamp(10.0, 24000.0)` in `chain_of`, silent `fc_hz.unwrap_or(1000.0)` defaults (TB-098), and DSP tests in `dsp.rs`.
- **ADR-0020:** Apache-2.0, with the standard `LICENSE` added (identical to Fletcher's).

Also filed Q-30 (first five minutes), Q-31 (assurance trace), Q-32 (prior art, scoped to "what worked"), Q-33 (eval harness), Q-34 (query language), Q-35 (node identity), Q-36 (editor sync). Q-07 now includes security; Q-12 includes latency. VISION goals 21–23 added; ROADMAP Phase 2 = MVP.
**Learned:** The user's MVP sentence hides the most valuable feature yet: "how do we know it's correct" is an *assurance trace*. The first real probe found a silent 1 kHz default on exactly the value asked about, the kind of thing a vibe-coded app hides and Insight should surface first.
**Also:** ADR-0021: the home screen is the whole system (top level, with sizes) plus a question box. The user: "you ask what you want". Resolves Q-30. Working-style note from the user: ask very specific questions, because general ones don't help.
**Next:** The Q-20 data-edge cut scoped to the MVP; SCHEMA draft with node identity; hand-trace the TB-100 key.

## 2026-10-09 (cont.) — Discovery 7b: static first, LLM in the loop, log probes; the language set
**Done:** ADR-0015: tracing is static-first (the user estimates ~99%), with the LLM resolving what static analysis can't (inferred tier). LLM-placed, tagged log probes in a scratch copy are the universal runtime tracer, which works on microcontrollers over serial (resolves most of TB-090). Q-03: committed C/C++, Rust, Python, TS/JS, Go, SQL. Suggested Kotlin/Java, Swift, C#, definition languages (protobuf/OpenAPI/GraphQL/JSON Schema → proven wire links), glue (shell/Docker/CI), UI markup, and embedded flavors, with a proposed order. Q-17/Q-24 now default to probes. TB-094–097 added (leftover probes, SQL in strings, probe-induced heisenbugs, measuring the 99%).
**Learned:** Log probes turn "a runtime tracer per language per platform" into "the LLM writes print statements". That's cheap and universal, and the parsing stays deterministic through tags. The 99% claim is plausible for direct-call languages but needs measuring on React-style code, so it's recorded as TB-097 rather than assumed.
**Next:** ~~User rules on the suggested languages~~ → accepted as ADR-0016 (all additions and the proposed order). Then the Q-20 v1 cut; first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 7: across the wire
**Done:** ADR-0014: a workspace can load multiple codebases (e.g. microcontroller firmware + protocol + frontend), connected by wire links (send effect ↔ receive), which are protocol plugins with proven / inferred / observed tiers. New Q-28 (wire-link matching: keys per protocol, binary layouts, versions, message correlation) and Q-29 (field-level data lineage, including unit and representation changes). Q-19 gained *lenses* ("focus on the algorithms"). Q-03 now notes embedded C/C++. TB-087–093 added (field-name mismatch → dangling ends, binary structs, no device tracer, double filtering, unit changes). VISION goal 20, GLOSSARY, ARCHITECTURE diagram, ROADMAP updated.
**Learned:** The user's question follows a *value*, not a call chain. That promotes value flow (Q-20) from "partly provable, later" to required. Wire mismatches (TB-088, TB-093) are high-value oversight findings that no single-repo tool can see.
**Next:** The Q-20 v1 cut, which now has to name lineage and wire links as v1 or later; first SCHEMA draft.

## 2026-10-09 (cont.) — First real-code example: Fletcher's history drag-graft
**Done:** The user offered Fletcher (their open-source Apache-2.0 Tauri app) as a usable real codebase. Traced "how does the history graph work, especially drag and drop, separating UI vs. other data structures" by hand through `App.tsx` (350 KB) and `lib.rs`. Wrote `docs/examples/fletcher-history-graft-worked-example.md`: a layered data-structure view (domain / sound / transport / UI-only, with "+N hidden"), a 12-step step-through with real code-quote receipts, and six deterministic findings (a gesture → system-wide audio effect; one component with two wirings by window; a duplicated type copied field by field; a silent `history_save` catch; ref + version-counter; an overloaded `base` field). TB-081–086 added. Q-20 gained the *crosses-boundary* and *persisted* data facts. ROADMAP: Fletcher is the lead tracer-bullet candidate, with this example as the known-correct answer. Fletcher was not modified.
**Learned:** Real code validated the design: every finding mapped onto an existing feature (effects, sibling comparison, expectations, error reachability, the explanation ladder). The most striking one: Fletcher's own CLAUDE.md has a process rule ("every feature ships to the satellites… props actually passed in PopoutHistory…") that exists only because the two wirings are invisible. That's exactly the gap Insight fills. Also, in vibe-coded monoliths the closures inside components are units too. **Correction (user):** I first wrote that files are "useless" containers in a monolith. Wrong. A file is a real logical boundary and the user's navigation memory, and a visibly huge file is a split-me signal. Clarified in ADR-0008 (every container level always exists; container size is visible), the example, and TB-086.
**Next:** The Q-20 v1 cut, now informed by real code (effects, error reachability, lives-in / crosses-boundary / persisted); first SCHEMA draft.

## 2026-10-09 (cont.) — ADR-0013 accepted (with an honesty caveat)
**Done:** Q-21 became ADR-0013 (seven rules: code is truth; taking away is free but visible; adding needs a quote receipt; proven/inferred/rejected set by the checker; graph shape = code shape; drift = broken receipt; never write down what the code says). It supersedes ADR-0006. The user accepted it but called much of it "hand-wavy for now", so the ADR states that the rules are accepted and the mechanisms unproven. A compiler spike was added to Phase 1. The TB cases that pointed at Q-21 now point at ADR-0013 as Partial (mechanism unproven).
**Learned:** In discovery it's fine to settle *rules* before *mechanisms*, as long as the doc says which is which. Otherwise a future session will treat a paper design as solved.
**Next:** The Q-20 v1 cut (effects + error reachability first); first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 6b: the oversight loop
**Done:** The user described the oversight workflow in their own words (prompt → see what it did → "you forgot the logger" → "this could happen a lot" → refactor by design). Wrote `docs/examples/oversight-loop-worked-example.md` around it. Q-25: the unit of review is a prompt, plus deterministic sibling comparison for absences. Q-26: ad hoc queries graduate into expectations. New Q-27: directing the agent from the graph (selection as context, MCP, hosted vs. external agent). TB-078–080 added. VISION goal 19 added.
**Learned:** The prompt is the natural scope of a system diff *and* its intent, which closes the open "where does the why come from" thread for AI-written code. Absences are the hardest thing for a human to spot, and the easiest for a deterministic graph to count.
**Next:** Sign off Q-21; the Q-20 v1 cut (effects + error reachability first, since both purposes need them); first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 6: the second purpose, oversight of AI-written code
**Done:** The user reframed the product: ~40% is development oversight, because "we no longer inspect line by line, we inspect system by system", and there's no IDE or debugger for AI-first code. Recorded as ADR-0012 (two purposes, one engine, learning wins ties). New Q-25 (system diff: behavioral before/after, with refactors collapsing to "no change") and Q-26 (plain-language expectations → user-confirmed deterministic graph queries, re-run on every change). Q-10 now leans toward IDE, Q-11 is partly answered. TB-073–077 added. The thesis was added to VISION (goals 17–18), README, CLAUDE.md/AGENTS.md, and ARCHITECTURE.
**Learned:** Oversight needs nothing new at the core. The fact graph, effects, receipts, and show-then-tell all carry over. What's new is *comparison* (two graph versions) and *standing intent* (expectations). Effects (Q-20) are now load-bearing for three features: learning ("what does this do?"), sandboxing step-throughs, and the system diff.
**Next:** Sign off Q-21; the Q-20 v1 cut, now with oversight in mind (effects and error reachability first); first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 5b: the step-through
**Done:** ADR-0011: debugger-like step-throughs with concrete values. The AI sets up the scenario, tests it (it must actually reach its target), then hands it over. Values are tagged executed / evaluated / assumed (stub), and LLM-predicted values are never displayed. Q-24 covers the mechanisms (generated harness, existing tests, interpreter, stubs at effects, deterministic schedulers) and sandboxing. TB-067–072 added. Scenarios (TB-065) gained an upgrade path: run the alternative in a scratch copy. VISION goal 16 and ARCHITECTURE updated. Q-21's target became "the next free ADR".
**Learned:** "The AI tests it before giving it to you" is the user's own instinct for ADR-0002 applied to values. It also gives Q-17 an AI-driven route to recorded runs, and gives effects (Q-20) a second job: they're the sandbox boundary list.
**Next:** Sign off Q-21; the Q-20 v1 cut; first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 5: show, then tell
**Done:** ADR-0010: every answer is view state plus prose. The graph focuses, highlights, and steps through, and each sentence links to its elements. Scenario overlays were added as a labeled hypothetical overlay kind (TB-065), alongside static and observed. Q-22: explanations can be shared (mechanism still open). Q-23 gained visual angles. Q-21's target ADR was renumbered to ADR-0011. TB-065/066, VISION goal 15, and the ARCHITECTURE provenance table were updated.
**Learned:** The graph is the explanation medium, not a backdrop. Linking sentences to elements also gives a cheap grounding check: a code claim with nothing to highlight is suspect (TB-066).
**Next:** Sign off Q-21 → ADR-0011; then the Q-20 v1 data/boundary cut, then Q-04 leftovers → first SCHEMA draft.

## 2026-10-09 (cont.) — Discovery 4: lots of explanation; patterns from a real learning log
**Done:** The user wants abundant explanation ("what does this do", then "a different way" if it doesn't land) and pointed to their own Q&A learning notes from learning a proprietary app. Generalized them into `docs/research/2026-10-learning-session-patterns.md`: 16 question shapes, 10 teaching moves, the honesty rules, and the user's entry format. Nothing app-specific was copied. New Q-22 (an abundant, anchored explanation layer plus a personal learning journal, distinct from minimal committed annotations) and Q-23 (the explain-differently ladder). Scoped Q-21 rule 6 to committed annotations only. TB-056–064 added. VISION goals 13–14 added.
**Learned:** (1) The winning teaching move was "follow one action, AI supplies the next question", not overviews. (2) Omission can mislead: a partial diagram made a container look pointless, so subtraction needs visible "+N hidden" markers (TB-056), which refines "taking away is free". (3) Symbol origin (ours vs. std vs. framework) is a cheap deterministic feature that answered a whole class of confusion. (4) The user's learning log already used receipts ("Where it lives") and preserved corrections, so the journal format is half-designed by the user.
**Next:** Sign off Q-21 → ADR-0010 (with the "+N hidden" refinement to rule 2); decide Q-22's sharing question.

## 2026-10-09 (cont.) — Worked example: "Save note" end to end
**Done:** The user agreed with the Q-21 restatement in principle but wanted a concrete example before signing off. Wrote `docs/examples/save-note-worked-example.md`: an invented Tauri notes app traced through the fact graph, an inline annotation with seven claims (5 proven, 1 inferred callback, 1 hallucinated log call rejected), collapsing wrappers, grouping vs. refactor for "validation", a dynamic run upgrading inferred → observed, a drift scenario (two broken receipts re-anchored), and two knowledge profiles. TB-051/052 added.
**Learned:** Line-number receipts would cause false drift on every unrelated edit, so receipts should be code quotes matched within the annotated container. Prose that cites claims inherits their staleness, which closes the "unverifiable prose drifts" gap.
**Next:** User reviews the example → if signed off, write ADR-0010 (supersedes ADR-0006); then the Q-20 v1 cut.
**Follow-up:** The user asked why c6 was inferred, and the answer corrected the example. c6 needs a global exhaustiveness fact ("the field's only value is this closure"), and in that Rust code a strong plugin *can* prove it. So the tier depends on the checker, not the claim. Chain receipts were added, plus TB-053.
**Follow-up 2:** The user flagged that this design would document every line. Correct: c1–c5 restated proven facts. Added Q-21 rule 6, "never write down what the code already says": structure is computed, explanations are generated live per user, and only intent/why plus gap-filling claims are committed, with redundant claims linted. The example's annotation shrank from 10 lines to 3. TB-054/055 added.

## 2026-10-09 (cont.) — Discovery 3: plugins, containers, and "adding needs a receipt"
**Done:** ADR-0007: languages are plugins on a language-agnostic core (cross-language bridges too). ADR-0008: containers are package › file › type › function › block, and control edges are call/branch/return/throw. The user's refinement of ADR-0006 is restated as Q-21 and awaits confirmation: code is the source of truth, taking away is free, adding needs an anchor the compiler re-checks, graph shape follows code shape, and drift becomes a broken receipt, plus an *inferred* tier for partly provable additions. Wrote `docs/research/2026-10-what-code-has.md` (data, types, concurrency, links vs. effects, meta, with a proposed v1 cut) because the user asked for guidance on Q-20. TB-045–050 added. ADR-0009: annotations are inline first (Doxygen-style), with sidecar files optional.
**Learned:** The user's worry isn't LLM accuracy, it's drift. That reframes the compiler from a "hallucination filter" into an "anchor re-checker", a more concrete and buildable thing. The boundary family splits naturally into *links* (other code we have) and *effects* (the outside world), and effects are likely the highest-value data for a learner.
**Next:** User confirms or corrects Q-21 → ADR-0010; reacts to the Q-20 catalogue and v1 cut.

## 2026-10-09 (cont.) — Discovery 2: notes not levels; annotations in the repo; the LLM gets tools, not the pen
**Done:** Three rulings became ADRs. ADR-0004: the knowledge profile is free-form notes the LLM interprets ("I know threading, but only Rust's"), personal and not committed. ADR-0005: annotations are committed repo files, possibly inline like Doxygen, which also lets the deterministic pipeline read them without an LLM pass. ADR-0006: the graph is built deterministically, and the LLM operates on it through a tool API (group, collapse, expand, annotate, compose a view) with no way to create structure, so ADR-0002 holds by construction. New questions: Q-19 (AI-composed custom views with code snippets; the user says this is tentative) and Q-20 (data/ownership edges, "how a function gets its members"). Q-16 resolved; Q-15 partially resolved (location settled; format and claim vocabulary open); Q-18 leans toward "beside the editor". TESTBED: TB-037–040 and TB-044 Solved (design); new TB-038–044. A real proprietary codebase was read as an example only, and its cases are recorded generically (TB-041–043).
**Learned:** The user's example question was about *data*, not calls: where a function's members come from through managed state and locks. A pure call graph would have failed the user's own first demo. Q-20 probably reshapes the graph model (Q-04).
**Next:** Q-04 + Q-20 together: what a node is, and which edge families exist.

## 2026-10-09 (cont.) — Discovery 1: never wrong; the map and the overlay; the annotation compiler
**Done:** Two rulings became ADRs. ADR-0002: the graph may be incomplete, never wrong (the HTT prime directive, adopted). ADR-0003: hybrid tracing, where the static graph is the map and dynamic runs are overlays that upgrade *possible* edges to *observed*. New ideas filed: Q-15 (a defined annotation format plus a "compiler" that removes hallucinations, "like doxygen but way better"), Q-16 ("levels": the user's own knowledge vs. abstraction zoom, needs clarification), Q-17 (how to capture "this point" at runtime), and Q-18 (live while developing AND after the fact, with both wanted). TESTBED: TB-002/033/034 Solved (design); TB-006/024/030 Partial; new cases TB-030–037. VISION, ARCHITECTURE, GLOSSARY, and ROADMAP synced.
**Learned:** The user's compiler idea is the enforcement mechanism for ADR-0002. Without it, "never wrong" has no teeth against LLM output. The proposed split is that claims are machine-checked and prose is shown as interpretation and must cite its sources.
**Next:** Clarify Q-16, then design the annotation format (Q-15): what kinds of claims exist and where annotations live.

## 2026-10-09 — Project created; doc system copied from Humanity Tech Tree
**Done:** Created the Insight repo with the Humanity Tech Tree documentation standard (ADR-0001): CLAUDE.md/AGENTS.md with the doc map and agent rules, README, VISION, ARCHITECTURE (pipeline sketch only), GLOSSARY, OPEN-QUESTIONS (Q-01–Q-14), TESTBED (TB-001–TB-029), ROADMAP, WORKLOG, ADR template, and the original pitch archived verbatim.
**Learned:** The pitch contains a latent tension: "start by clicking a UI element" suggests dynamic tracing, while "finds all branches" suggests static analysis (Q-02). HTT's prime directive ("incomplete, never wrong") looks like a natural fit for the static/LLM split (Q-01).
**Next:** The discovery conversation, starting with Q-02, Q-01, and Q-04.
