# SCHEMA: the graph model

**Normative.** This file defines what a snapshot contains. On a conflict with `ARCHITECTURE.md` or `README.md`, this file wins. A change to the graph model changes this file first, plus an ADR if a decision was involved (CLAUDE.md rule 7).

Sources: BQ-09 (kinds), BQ-10/R3-G #1 (IDs), BQ-11 (spans), BQ-12 (receipts), BQ-18 (blocks), R2-28 (extractor protocol), R2-30/R3-G (tables), R3-11 (counting rules), R4-B (concept and schema additions), ADR-0038 (SQL), ADR-0048 (programming concepts only), ADR-0049 (lean by default, pedantic on request).

Each item has a **Status**:
- **Built**: emitted and stored today.
- **Planned**: specified here, not built yet. Until it is built, the database simply has no rows of that kind; nothing is guessed in their place.

## 1. Principles the schema obeys

- **Programming concepts only** (ADR-0048). Every kind below is a concept that exists in programming languages generally. No kind names a framework or library. Language knowledge lives only in each language's extractor, which translates its language into these concepts.
- **Facts, not interpretation** (ADR-0002, ADR-0035). Everything stored was produced by a compiler-grade tool from the snapshot's exact bytes (ADR-0043). Meaning is Claude's.
- **Lean by default, pedantic on request** (ADR-0049). Detail such as standard-library-origin facts is stored, but marked (`origin`), and is excluded by default by the tools, not by the database.

## 2. Nodes

Every node is one row in `nodes`. Containment (what is lexically inside what) is `parent_nid`; it is not stored as edges.

| `kind` | `subkind` | Meaning | Status |
|---|---|---|---|
| `container` | `file` | A source file. | Built |
| `container` | `module` | An inline module with a body (Rust `mod x { … }`). File modules are the `file`. | Built |
| `container` | `type` | struct, class, enum, union, interface, trait, and Rust `impl` blocks (named `impl T` / `impl Tr for T`). | Built |
| `container` | `function` | Every function with a body, named or anonymous: functions, methods, accessors, constructors, arrows, closures, and functions written inside macro-call arguments (§6). Signatures without bodies are not nodes. | Built |
| `container` | `block` | A control-flow region: `if`/`else` arm, `switch` case/default, `match` arm, loop body, `try`/`catch`/`finally`, ternary arm (BQ-18). | Built |
| `container` | `init` | Module initialization code, with init order where the language defines it (P11). | Planned |
| `state` | `var` / `field` / `static` (`element` / `cell`: Planned) | Storage (R3-G #3): locals and parameters are `var` (child of their function), fields are `field` (child of their type; one node per declaration, shared by every instance until `base_nid`), globals and static locals are `static`. Ids use their own ordinals and are assigned after a file's containers, so adding a variable never renames a function. **TS:** variables, destructured bindings, parameters, class fields and interface/type-literal members (so props are fields). **Rust:** `let`/parameter/match bindings and `self`, struct fields, statics and consts. | Built |
| `literal` | | One node per distinct value, so same-literal links are not N² (R2-30): strings `lit:<JSON string>` (`lit:"hist-cmd"`), integers in canonical decimal `lit:31` (`0x1F`, `31u8` and `31` are one node). Created by the solver's link pass from every extractor's literal occurrences, across languages. | Built (strings, integers) |
| `external` | | A referenced symbol outside the analyzed code, one leaf per symbol (BQ-20): `ext:ts:<package or lib>:<Owner.name>`, `ext:rust:<crate>:<module::path::Type::name>`, `ext:cpp:<declaring header>:<ns::Type::name>` (builtins have an empty header). Package version: Planned. | Built |
| `unresolved` | | A reference the compiler could not resolve. Today this is an edge with `resolution='unresolved'` and `to_nid NULL` instead of a node. | Planned |
| `macro` | | A macro definition; expansions link to it with `expand` (P9). | Planned |
| `inactive` | | Source text the build configuration excluded (an inactive `#if` region, P9), id `<file> › #if-inactive#k`. Recorded, never analyzed. A region compiled by another inclusion of the same header is not inactive. | Built (C/C++) |

Node column `anonymous` (Built): 1 when the name is positional (`map#1`, `=>#2`, blocks), so `names` links only match declared names. Node column `generated` (Built for C/C++, R4-B): 1 for a function or type a macro produced from its own template (its body is spelled in the macro, not at its location). Planned node columns: `origin` (`project` / `dependency` / `stdlib`, ADR-0049), `body_hash` (R3-17: "same name, different body" is visible).

## 3. Node IDs (`nodes.path`)

Implements BQ-10 / R3-G #1.

- **Segments** are joined by ` › ` (space, U+203A, space). The first segment is the repo-relative file path.
- **Path segments** are files, modules, types and functions. **Blocks are containers but not scopes**: a block's ID is `<scope> › <label>#<k>`, but a function inside a block is named under the enclosing scope, so adding an `if` never renames a function.
- **Named** functions and types use their name: `app/src/App.tsx › MainApp › graftNode`.
- **Anonymous** functions are named by where they appear, always with a 1-based ordinal per (scope, name):
  - passed to a call: the callee (`useEffect#14`, `map#1`, `listen#2`);
  - a JSX attribute: the attribute name (`onClick#1`);
  - TS `export default` without a name: `default#1`;
  - otherwise `=>#k` (TS) or `|…|#k` (Rust);
  - inside macro-call arguments (Rust): the macro (`println!#1`, `json!#3`).
- **Duplicate names** in one scope get ordinals from the second on (`f`, `f#2`).
- **Block labels:** `if`, `else`, `case`, `default`, `for`, `while`, `do`, `loop`, `try`, `catch`, `finally`, `?`, `:` (TS ternary arms), `arm` (Rust `match`). Ordinals count per scope.
- IDs are unique per snapshot (enforced on insert). The agent may pass a unique suffix; an ambiguous one returns the candidates (BQ-10, M2).

## 4. Spans and files

- Paths: repo-relative, forward slash, case-exact (BQ-11).
- Spans: UTF-8 byte range `[byte_start, byte_end)` into the snapshot's bytes, plus 1-based `line_start`/`line_end`. UTF-16 conversion happens only at the window boundary.
- The `files` table holds the exact bytes of every file an extractor read, with their SHA-256. **A snapshot is exactly those files** (R3-05). Text for receipts is computed from spans, never stored twice (R3-G #8).

## 5. Edges and receipts

| `kind` | `subkind` | Meaning | Status |
|---|---|---|---|
| `call` | | A function calls a function (or an external leaf). **TS:** resolved by the project's own checker; a call through a variable, parameter, property or prop gets `possible` edges to every function value written to that declaration (one-hop value flow, BQ-15), tied by `group_id`, plus an absent candidate (`to_nid NULL`) when the call is optional (`?.`) or the declaration is optional. Writes counted: variable initializers, assignments, object-literal properties and JSX attributes (matched through contextual typing), and call arguments (matched to the resolved signature's parameters). **Rust:** resolved by rust-analyzer through macro expansions (attribute macros included). **C/C++:** resolved by clang (libclang) on the project's own compile commands, across translation units by symbol (USR); a template instance resolves to its template. A virtual call through a pointer or reference gets `possible` edges, tied by `group_id`, to the named method (if it has a body) and every override in the analyzed code. Calls to compiler-generated constructors and operators have no code and are not edges. | Built |
| `call` | `implicit` | Operators, accessors, conversions, Deref, Index, iteration, destructors (P3). | Built (C/C++ operator and conversion calls to written code); Planned elsewhere |
| `value-flow` | `handoff` | A function value passed to external code (an argument of an external call; for JSX, the component, or the function-valued attributes of an intrinsic element; for C/C++, a function, `&function` or lambda argument). Who calls it later is the model's question. | Built |
| `value-flow` | `assign` | State, function value or lambda → the state receiving it (`b = a`, `T b = a;`, `b = f;`, destructuring reads the property it names). By declaration, not by program point: where it happens is the receipt. | Built |
| `value-flow` | `result` | A callee → the state, parameter or function receiving its result (`b = f(x)`, `g(f(x))`, `return f(x)`). | Built |
| `value-flow` | `return` | State → the function returning its value (`return b;`, a tail expression, an arrow's expression body). | Built |
| `value-flow` | `returns` | A function returns this function literal, one level (BQ-15; curried path starts, R3-G #5). | Built (TS) |
| `link` | `literal` | From a function (or file) to a `literal` node, one edge per occurrence (BQ-22). Order-unknown hop (R3-14 as amended). | Built |
| `link` | `names` | From a `literal` node to every named function, type or module whose declared name equals the string (ADR-0048). | Built |
| `read` / `write` / `access` | `write` subkind `pointee`: through a pointer it holds | Function → state, one edge per occurrence. `read`: the value is used. `write`: assigned, initialized, `++`/`--` (compound assignments also read). `access`: may write: passed by non-const reference, or `&x` to a pointer parameter (R3-G #13). `via_eid`: Planned. | Built |
| `binds` | mode: `value` / `ref` / `pointer` (move / out / inout, closure captures: Planned) | Argument → parameter (P6): each state, function value or lambda the argument's value comes from → the callee's parameter at that index, or → the external leaf for external callees (data handed to code outside the analysis). For a virtual call, `possible` edges to the same parameter of every override, tied by `group_id`. `ref`/`pointer` mode means the callee can write back through it (Rust: `&mut` parameters and `&mut x` arguments). **TS:** a JSX attribute binds to the prop's field; an object literal assigns into its contextual type's fields. | Built |
| `implements` | overrides, structural conformance, default methods (P7) | Type ↔ interface/trait; dispatch is expanded at query time (BQ-19). Rust calls through `dyn Trait` are `unresolved` until then. | Planned |
| `exit` | kinds and targets: return, `?`, throw, break/continue, diverges; catch types (P5) | | Planned |
| `suspend` / `resume` | `await` / `yield` (P2) | | Planned |
| `imports`, `has-type`, `instantiates`, `expand`, `annotates` | (P8–P11) | | Planned |

Each edge row is one occurrence (R2-30):
- `resolution` is `exact`, `possible` or `unresolved`.
- `group_id` ties one call's `possible` candidates together.
- `to_nid NULL` is an absent candidate (`possible`) or an unknown target (`unresolved`).

**Calls through variables, parameters and fields (C/C++, Rust; TS keeps its one-hop candidates above):** the call edge points at the state called (`resolution=exact`: "calls whatever this holds"). The solver then adds `possible` call edges, one group per call, to every function value that reaches that state backward through `assign` and `binds` edges, through `ref`/`pointer` bindings (what the callee assigns to the parameter reaches the argument), and through `result` (a function's `returns` targets; the states it `return`s). Language-independent; it applies to any extractor that emits these edges.

**Known limit (C/C++):** libclang exposes template code as written, not instantiated, so a name that depends on a template parameter (`&F<T>::run` with `F` a template parameter) resolves to nothing. The flow stops at the write in the template, and its receipt shows the code. **Known limit (Rust):** code inside macro-call arguments is not read for data flow (calls there are).

**External leaves are never passed through** by `path`: one leaf stands for every call site of a symbol, so a route may start or end at one (data handed to, or coming from, outside code) but never continues through it.

Planned edge columns:
- `timing` (P1);
- `call_eid` + `arg_index`;
- `base_nid`;
- `via_eid`;
- `generated`.

**Receipts** (BQ-12, R3-G #4, #8). Each receipt records the role, file, byte range, line/col range and the innermost `container_nid` (blocks included). Roles:
- `call`: the call expression;
- `value`: the write that supplied a `possible` candidate;
- `arg`: the handed-off argument;
- `jsx`: the component tag;
- `returns`: the returning function;
- `literal`: the literal occurrence;
- `decl`: the declaration a `names` edge points at.

Macro-generated facts with no source text of their own (role `macro`) are Planned. Calls written inside macro arguments are receipted at their own source text.
## 6. Counting rules for the M0 check

The M0 pass (R3-11) compares the extractor's function count per file against `insight-count`, a separate tree-sitter counter. Mismatches fail the build (exit 7) and are listed per file.

A **function** is any function-like syntax **with a body** written in the source, named or anonymous:
- **TS/JS:** function declarations and expressions, generators, arrows, and method definitions with bodies (including constructors and accessors). Abstract and overload signatures don't count. `.d.ts` files are not counted.
- **Rust:** `fn` items with bodies and closures. Trait method signatures without a default body don't count.
- **Rust macro-call arguments are counted**, because that code is written in the source.
  - **Extractor side:** found through rust-analyzer's real expansion and mapped back to the source text.
  - **Counter side:** re-parsed independently:
    1. the whole argument list, parsed as call arguments or as block statements;
    2. else each comma-separated piece, as an expression, a pattern, or the value after a leading `key:` / `key =>` / `key =`;
    3. else the bracketed groups inside the piece, recursively.
- **C/C++:** definitions with a written body (`{…}` or a function-try-block) and lambdas. `= default`, `= delete` and pure virtual declarations have no body. Functions in inactive `#if` regions are not compiled, so the counter skips the extractor's recorded `inactive` regions (which regions are inactive is a preprocessor fact; whether a function is written there is still counted independently). The counter needs a function declarator, so a misparse like `struct EXPORT_API Name { … }` is not a function.
- **Not counted:** `macro_rules!` and `#define` bodies (templates), and functions a macro generates from its own template (`generated=1`).
- **Unchecked (C/C++ only):** without a preprocessor, tree-sitter cannot parse some code (unexpanded macros produce `ERROR` nodes). A function is identified on both sides by its last line; any last line where either side's function touches a failed parse is left out of both counts. Each file with unchecked functions gets an info diagnostic and a log line with the number, so the gap is stated, never silent. Rust and TypeScript parse without a preprocessor, so their counts stay strict.
- **On a mismatch,** the refusal lists, per file, the last lines of the functions only one side found.

## 7. Tables (layout version 5)

Built today, as created by `insight-engine`:

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (path TEXT PRIMARY KEY, lang TEXT NOT NULL, sha256 TEXT NOT NULL,
                    bytes INTEGER NOT NULL, content BLOB NOT NULL);
CREATE TABLE nodes (nid INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                    parent_nid INTEGER REFERENCES nodes(nid), kind TEXT NOT NULL, subkind TEXT,
                    name TEXT NOT NULL, anonymous INTEGER NOT NULL DEFAULT 0, generated INTEGER NOT NULL DEFAULT 0, lang TEXT, file TEXT,
                    byte_start INTEGER, byte_end INTEGER, line_start INTEGER, line_end INTEGER);
CREATE TABLE edges (eid INTEGER PRIMARY KEY, kind TEXT NOT NULL, subkind TEXT,
                    from_nid INTEGER NOT NULL REFERENCES nodes(nid), to_nid INTEGER REFERENCES nodes(nid),
                    resolution TEXT NOT NULL, group_id INTEGER);
CREATE TABLE receipts (eid INTEGER NOT NULL REFERENCES edges(eid), role TEXT NOT NULL, file TEXT NOT NULL,
                       byte_start INTEGER NOT NULL, byte_end INTEGER NOT NULL, line INTEGER NOT NULL,
                       col INTEGER NOT NULL, end_line INTEGER NOT NULL, end_col INTEGER NOT NULL,
                       container_nid INTEGER REFERENCES nodes(nid));
CREATE TABLE tags (node_nid INTEGER NOT NULL REFERENCES nodes(nid), tag TEXT NOT NULL);
CREATE TABLE diagnostics (file TEXT, severity TEXT NOT NULL, message TEXT NOT NULL);
```

Indexes: `nodes(parent_nid)`, `nodes(file)`, `nodes(kind)`, `nodes(name)`, `edges(from_nid)`, `edges(to_nid)`, `receipts(eid)`.

`meta` keys: `project`, `repo`, `commit`, `fact_schema`, `extractor.<lang>` (name and toolchain), `layout_version`, `complete`.

`diagnostics` holds extractor diagnostics and `not analyzed: …` rows for source files that no Cargo package or tsconfig covers. Insight never silently skips a file.

Planned: `entries` and `order_breaks` tables, a `race_candidates` view, and the `reach()` / `nid()` SQL functions (R4-B).

## 8. Snapshot lifecycle

- **Written atomically:** a temp file, bulk insert, indexes, then `meta.complete = 1`, then a rename. Readers open the database read-only and refuse any snapshot without `complete` (R3-G #9).
- **Cache key:** a hash of everything that affects the stored facts (R2-28, R3-08):
  - the commit;
  - the fact-schema version and the table-layout version;
  - the TS extractor script and the Rust extractor binary;
  - that repo's install and check commands.

  Identical keys reuse the existing file.
- **File name:** `%LOCALAPPDATA%\Insight[-dev]\snapshots\<project>\<repo>-<key>.sqlite`.

## 9. Extractor protocol (R2-28), fact schema 3

`<extractor> --root <dir> (--project <cfg> | --package <dir> | --compdb <compile_commands.json>)… --schema <N>` writes NDJSON to stdout. Progress goes to stderr, then to the build log.

1. `{"type":"header","schema":N,"extractor":"<name>@<version>","toolchain":"…"}`
2. Facts:
   - `{"type":"file","path","lang","sha256","bytes"}`, before that file's nodes;
   - `{"type":"node","id","parent","kind","name","span":{…}|null,"anonymous","generated"}`, parents before children. `kind` is `file` / `module` / `type` / `function` / `block` / `external` / `inactive`; externals have no parent and no span.
   - `{"type":"edge","kind":"call"|"value_flow","subkind","from","to"|null,"resolution","group"|null,"receipts":[…]}`. Edges may come after all nodes; they may name any node of the same stream.
   - `{"type":"literal","value","number","from","receipt"}`: one per string or integer literal occurrence; `number` is true for integers, whose `value` is canonical decimal.
   - `{"type":"diagnostic","file","severity","message"}`.
3. `{"type":"end","files","nodes","edges","literals","diagnostics"}`. The counts must match what was streamed, which detects truncation.

Insight rejects any of the following:
- a schema mismatch;
- facts before the header;
- records after `end`;
- a missing `end`;
- `end` counts that don't match;
- an edge naming an unknown node;
- a file whose bytes no longer match its `sha256` (R3-05).

The engine stores a container `kind` as `kind='container'` with that `subkind`, and `value_flow` as `value-flow`. The same external leaf named by two extractors is stored once.

## 10. `path` (R2-12, ADR-0047, R4-B)

`insight path <project> [<from>…] [--to <to>]… --follow <tokens> [--max-occurrences N] [--max-depth N] [--snapshot <file>] [--no-receipts]`

Over MCP: `path(from?, to?, follow, max_occurrences?, max_depth?, receipts=expr|none)`.

**Follow tokens** are always explicit; there is no default. Each one is an exact predicate over `edges`:
- `call`: `kind='call'` with a target. `possible` is labeled `call (possible)`.
- `handoff`: `kind='value-flow' AND subkind='handoff'`.
- `literal`: function →`link/literal`→ literal node → every function containing the same literal. The literal node is shown on the route.
- `names`: function →`link/literal`→ literal node →`link/names`→ the declaration.
- `data`: `value-flow` `assign` / `result` / `return` and `binds` (labeled `possible` for virtual-dispatch bindings). Data reaching a function arrives at its parameter, so a data route usually ends at a parameter or an external leaf.
- `write` / `access`: function → state. `read`: state → the function reading it (the read edge walked toward the reader). With both `read` and `write`, a function joins every state it reads to every state it writes; that is coarser than `data`, and the choice is the caller's.

**Ends:**
- **Resolution:** an exact id, or a unique suffix starting at a ` › `, `/` or `:` boundary. An ambiguous suffix returns the candidates.
- **Sets and open ends (R4-B):** each end is a set. Give both for routes between them, only `from` for forward reachability, or only `to` for reverse reachability (everything that reaches a target).
- **The start** also includes the functions it returns (`value-flow/returns`, R3-G #5).
- **Targets** are terminal: a route ends there.

**Constraints:**
- `max_occurrences`: skips literals that occur more often than N.
- `max_depth`: keeps only routes of at most N hops (needs `--to`).

**Result:** every hop on some route from a start to a target. The rules:
- **Progress:** each hop moves further from the start or closer to a target.
- **Literal round trips:** `F → lit → F` is never a hop.
- **Pruning:** with starts, a node other than a start needs a way in; with targets, a node other than a target needs a way out.
- **Loops:** strongly connected groups and self-loops are reported as cycles or recursion, never followed forever.
- **Receipts:** each hop lists its receipts with the source text.

The answer goes to stdout; the build-log rule (R2-20) is for builds.

## 11. Agent SQL (ADR-0038, R3-G #7, R4-B)

`sql(query, max_rows, timeout_ms)` runs over the loaded snapshot (`main`) and any attached ones (`<alias>.<table>`). The rules:
- **Open:** immutable and read-only.
- **Authorizer allowlist:** SELECT, reads, recursive CTEs, functions except `load_extension`, and read-only introspection pragmas. Everything else is denied, including ATTACH and DETACH.
- **Limits:**
  - one statement;
  - `timeout_ms` (a progress-handler deadline);
  - a 1 GiB heap limit and length limits;
  - `max_rows` ≤ 10,000 and `timeout_ms` ≤ 30,000.
- **Result:** the total row count is always reported, and it's a lower bound if the deadline hit while counting. Blobs show as their size.
- **Shipped views (TEMP):** `callers` and `receipt_text`. `writers` and `race_candidates` are Planned, with read/write edges. `reach()` and `nid()` are Planned.
- **Deny rules:** when the agent's deny rules cover a file in the snapshot, `files.content` reads as NULL.

**M1 gate (R3-12):** `crates/insight-solver/tests/m1_gate.rs` checks the drag-and-drop walkthrough's required hops on Fletcher `cd5d6ff`. It covers both windows' routes to `graftNode` and the cross-language route to `fsx::write_atomic` through a `names` hop.
