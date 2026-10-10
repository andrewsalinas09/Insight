# SCHEMA: the graph model

**Normative.** This file defines what a snapshot contains. On a conflict with `ARCHITECTURE.md` or `README.md`, this file wins. A change to the graph model changes this file first, plus an ADR if a decision was involved (CLAUDE.md rule 7).

Sources: BQ-09 (kinds), BQ-10/R3-G #1 (IDs), BQ-11 (spans), BQ-12 (receipts), BQ-18 (blocks), R2-28 (extractor protocol), R2-30/R3-G (tables), R3-11 (counting rules), R4-B (concept and schema additions), ADR-0038 (SQL), ADR-0048 (programming concepts only), ADR-0049 (lean by default, pedantic on request).

Each item has a **Status**:
- **Built**: emitted and stored today.
- **Planned (Mn)**: specified here, arriving with milestone *n*. Until then the database simply has no rows of that kind; nothing is guessed in their place.

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
| `container` | `init` | Module initialization code, with init order where the language defines it (P11). | Planned (M1) |
| `state` | `var` / `field` / `element` / `static` / `cell` | Storage. Locals and parameters are `var` state (R3-G #3). | Planned (M1) |
| `literal` | | One node per distinct literal value (`lit:"hist-cmd"`), so same-literal links are not N² (R2-30). | Planned (M1) |
| `external` | | A referenced symbol outside the analyzed code, one leaf per symbol, carrying package/module/version (BQ-20, R4-B). | Planned (M1) |
| `unresolved` | | A reference the compiler could not resolve. | Planned (M1) |
| `macro` | | A macro definition; expansions link to it with `expand` (P9). | Planned (M1) |

Planned node columns: `origin` (`project` / `dependency` / `stdlib`, ADR-0049), `body_hash` (R3-17: "same name, different body" is visible), `generated` (1 for macro-generated, R4-B).

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

| `kind` | Subkinds | Meaning | Status |
|---|---|---|---|
| `call` | `implicit` (operators, accessors, conversions, Deref, Index, iteration, destructors; P3) | A function calls a function. | Planned (M1) |
| `read` / `write` | | A function reads / writes state. | Planned (M1) |
| `access` | | May write (e.g. through `&self` with interior mutability, R3-G #13); `via_eid` names the method. | Planned (M1) |
| `binds` | modes: value / move / ref / mut-ref / out / inout / pointer; closure captures | Argument → parameter, prop → field (JSX via contextual typing, R4-B). | Planned (M1) |
| `value-flow` | `handoff` | A value (often a function) flows somewhere; `handoff` = passed to external code that may call it later. | Planned (M1) |
| `implements` | overrides, structural conformance, default methods (P7) | Type ↔ interface/trait; dispatch is expanded at query time (BQ-19). | Planned (M1) |
| `link` | `literal` (same literal, BQ-22), `names` (a literal equals a declaration's name, ADR-0048) | Order-unknown hops (R3-14 as amended). | Planned (M1) |
| `exit` | kinds and targets: return, `?`, throw, break/continue, diverges; catch types (P5) | | Planned (M1) |
| `suspend` / `resume` | `await` / `yield` (P2) | | Planned (M1) |
| `imports`, `has-type`, `instantiates`, `expand`, `annotates` | (P8–P11) | | Planned (M1) |

Each edge row is one occurrence (R2-30): `resolution` is `exact` / `possible` / `unresolved`; `group_id` ties one call's `possible` candidates together; an absent target is `to_nid NULL`. Planned edge columns: `timing` (inline / eager-start / deferred-create / concurrent / scope-exit / load / build / unsequenced; P1), `call_eid` + `arg_index` (edges inside argument lists), `base_nid` (which object a state access is on), `via_eid`, `generated`.

**Receipts** (BQ-12, R3-G #4, #8): one per edge (two for `link`, by `role`): file, byte range, line/col range, and the innermost `container_nid`. Macro-generated facts carry the macro call site with `role='macro'`.

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
- **Not counted:** `macro_rules!` bodies (templates), and functions a macro generates from its own template. Those stay in the graph with `generated=1` once that column lands.

## 7. Tables (layout version 2)

Built today, as created by `insight-engine`:

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (path TEXT PRIMARY KEY, lang TEXT NOT NULL, sha256 TEXT NOT NULL,
                    bytes INTEGER NOT NULL, content BLOB NOT NULL);
CREATE TABLE nodes (nid INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE,
                    parent_nid INTEGER REFERENCES nodes(nid), kind TEXT NOT NULL, subkind TEXT,
                    name TEXT NOT NULL, lang TEXT, file TEXT,
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

`meta` keys: `project`, `repo`, `commit`, `fact_schema`, `extractor.<lang>` (name and toolchain), `layout_version`, `complete`.

`diagnostics` holds extractor diagnostics and `not analyzed: …` rows for source files that no Cargo package or tsconfig covers. Insight never silently skips a file.

Planned (M1–M2): `entries` and `order_breaks` tables, a `race_candidates` view, and the `reach()` / `nid()` SQL functions (R4-B).

## 8. Snapshot lifecycle

- **Written atomically:** a temp file, bulk insert, indexes, then `meta.complete = 1`, then a rename. Readers open the database read-only and refuse any snapshot without `complete` (R3-G #9).
- **Cache key:** a hash of everything that affects the stored facts (R2-28, R3-08):
  - the commit;
  - the fact-schema version and the table-layout version;
  - the TS extractor script and the Rust extractor binary;
  - that repo's install and check commands.

  Identical keys reuse the existing file.
- **File name:** `%LOCALAPPDATA%\Insight[-dev]\snapshots\<project>\<repo>-<key>.sqlite`.

## 9. Extractor protocol (R2-28)

`<extractor> --root <dir> (--project <cfg> | --package <dir>)… --schema <N>` writes NDJSON to stdout. Progress goes to stderr, then to the build log.

1. `{"type":"header","schema":N,"extractor":"<name>@<version>","toolchain":"…"}`
2. Facts, in this order per file: first `{"type":"file","path","lang","sha256","bytes"}`, then that file's `{"type":"node","id","parent","kind","name","span":{…}}` records, parents before children, plus any `{"type":"diagnostic","file","severity","message"}`.
3. `{"type":"end","files","nodes","diagnostics"}`. The counts must match what was streamed, which detects truncation.

Insight rejects any of the following:
- a schema mismatch;
- facts before the header;
- records after `end`;
- a missing `end`;
- `end` counts that don't match;
- a file whose bytes no longer match its `sha256` (R3-05).

In the protocol, `kind` is the container subkind (`file` / `module` / `type` / `function` / `block`); the engine stores it as `kind='container'` with that `subkind`.
