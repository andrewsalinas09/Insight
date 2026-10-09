# Architecture (current state)

> **Derived document.** The ADRs and `docs/TESTBED.md` are normative. On any conflict with this summary, they win.

Early design. Settled so far:
- the prime directive (ADR-0002)
- hybrid tracing (ADR-0003)
- the free-form knowledge profile (ADR-0004)
- annotations committed to the repo (ADR-0005)
- the deterministic graph plus the LLM tool API; code is truth, and additions need receipts (ADR-0013, superseding ADR-0006; rules accepted, mechanisms unproven)
- language plugins (ADR-0007); the set and build order (ADR-0016)
- containers and control edges (ADR-0008)
- annotations inline first, sidecar optional (ADR-0009)
- show, then tell: answers drive the graph and the prose together (ADR-0010)
- step-throughs: AI-set-up, verified, debugger-like walkthroughs with real values (ADR-0011)
- two purposes on one engine: learning ~60%, system-level oversight ~40% (ADR-0012)
- multi-codebase workspaces joined by wire links (ADR-0014)
- tracing: static first → LLM in the loop where unsure (inferred) → LLM-placed log probes as the universal runtime tracer (ADR-0015)
- coding agents live outside; Insight is their MCP tool and the human's window (ADR-0017)
- time is explicit: timing kinds on every edge, execution contexts, a sequence view (ADR-0018)
- the MVP is one question: how an algorithm works, its data structures and where they live, and how we know it's correct (ADR-0019)
- license: Apache-2.0 (ADR-0020)
- home screen: the whole system plus a question box (ADR-0021)
- the fact layer is complete (every function, anonymous ones too); the LLM curates what's shown (ADR-0022)
- **a graph is an immutable snapshot of one git commit; comparing = two commits; explanations and views are pinned to their commit (ADR-0029). Identity = location; the model reconciles renames (ADR-0028)**
- **visual language: Fletcher's "Console"** (paper/ink, IBM Plex, hairline strips, labeled inspector cells, docked bottom bar for the agent's narration; ink = real code, orange = model's drawing, blue = changes now) (ADR-0027)
- **three layers (ADR-0026): fact layer (solver, never invented) → teaching views (model-authored: annotations, groups, hierarchy, abstractions, simplified code; visibly authored, mapped to real nodes) → UI**
- **how it's used (ADR-0025): you ask in the terminal (Claude Code / Codex); the agent answers in text and calls Insight's local MCP tools; the Insight desktop window shows and reacts. No in-app chat or model calls. One user, one machine.**
- **what we build (ADR-0024): the graph, the UI, and the tools the model drives. Understanding is the model's. Heavy verification machinery is optional, built only if real use demands it**
- **division of labor: the solver connects everything (calls, every read/write of every state, same-source bindings, exact-match links); anything requiring understanding (timing meaning, effects, validation vs. fallback, labels) is the model's job (ADR-0023)**

## Workspaces across devices (ADR-0014)

```
 firmware (C, uC)                    radio / protocol                 frontend (TS)
 read_adc ─► lowpass ─► pack_json ══ send ══(Wi-Fi/BLE: JSON "ax")══ recv ══► parse ─► smooth ─► plot
                                   wire link: proven (shared schema) / inferred (same keys) / observed (capture)
 lineage of value "ax": counts ─(lowpass)─► counts ─(scale ×0.000061)─► g ─(json)─► g ─(movavg)─► g ─► pixels
```

Wire links are protocol plugins (ADR-0007). Data lineage (Q-29) follows the value through every hop, and lenses (Q-19) collapse the plumbing.

## Oversight on the same engine (ADR-0012)

```
fact graph @ before ─┐
                     ├─► system diff (Q-25): new/removed paths, effects, error reachability, shared state
fact graph @ after  ─┘                         │
                                               ▼
expectations (Q-26) ── deterministic queries ──► pass / violated (offending path highlighted, ADR-0010)
```

Before and after can be any two versions: a commit, a PR, or an agent session (live mode, Q-18).

## Step-throughs (ADR-0011)

```
AI sets up scenario ─► run / evaluate deterministically (Q-24) ─► reached the target? ─no─► adjust inputs
     (inputs, state,         stubs at effects = "assumed"                │yes
      interleaving)                                                      ▼
                                                     hand over: steps = location + values + prose
                                                     user clicks Next/Back; the graph follows (ADR-0010)
```

Value provenance: **executed**, **evaluated**, or **assumed** (stub). An LLM-predicted value is never displayed as a value.

## Graph model so far (ADR-0008)

- **Containers:** package › file › type › function › block, mapped by each language plugin (ADR-0007). This is the base hierarchy.
- **Control edges:** call, branch (one per outcome), return, throw.
- **Data and boundary edges:** being designed (Q-20; catalogue in `docs/research/2026-10-what-code-has.md`).

Everything else is a sketch with open questions attached.

## Two layers (ADR-0006)

| Layer | Built by | Contains | Can the LLM change it? |
|---|---|---|---|
| **Fact layer** | deterministic analysis + recorded runs | nodes, call/branch edges, (later) data/ownership edges (Q-20), provenance | **No.** The tool API has no add or remove operations |
| **Organization layer** | LLM via tools, and humans | hierarchy groupings, collapse/expand state, annotations (claims + prose), custom views (Q-19) | Yes. Claims still pass the compiler |

Annotations live in the repo as committed files, inline or in sidecars (ADR-0005). The knowledge profile lives with the user, not the repo (ADR-0004). It shapes prose and chat only, never claims or structure.

## The pipeline

```
codebase ─► ingest ─► static graph (the map) ─────────────┐
                           ▲                               ├─► view ─► converse
                 run ─► dynamic trace (overlay) ───────────┤
                                                           │
            LLM ─► annotations ─► compiler (verify) ───────┘
```

1. **Ingest.** Add a codebase, then index its functions, calls, and branches. Scope defaults to the project's own code. Live mode needs incremental re-indexing (Q-09, Q-18).
2. **Static graph: the map (ADR-0003).** Every reachable bubble, call edge, and branch edge from any trigger (Q-05). Edges static analysis can't resolve exactly are drawn as *possible*, never guessed (ADR-0002).
3. **Dynamic trace: the overlay (ADR-0003).** A recorded run (Q-17) highlights the path actually taken and upgrades *possible* edges to *observed*.
4. **Annotations and the compiler (Q-15, ADR-0005).** Annotations are committed files, inline or in sidecars, in a defined format: checkable claims plus prose. The LLM writes them through the tool API, and humans can too. The compiler checks every claim against the graph, whoever wrote it, and drops failures. Prose is shown as interpretation, linked to its claims and source spans. Existing annotations are read deterministically, so the LLM doesn't have to re-parse the codebase.
5. **View.** A zoomable hierarchy of bubbles, plus AI-composed custom views with code snippets (Q-19). It lives next to the editor while you develop (Q-18 leaning).
6. **Converse: show, then tell (ADR-0010).** Every answer is a view state plus prose. The answer focuses and highlights the graph (and can step through sequences, including labeled *hypothetical* scenarios), and each prose sentence links to the elements it relies on. Grounded on the selection, verified annotations, source, and the user's knowledge profile (ADR-0004). Answers are kept in the learning journal and can be shared (Q-22).

## Edge provenance

| Provenance | Meaning | Source |
|---|---|---|
| static (proven) | Analysis proves this edge exists in the code | analyzer |
| static (possible) | One of several edges analysis can't narrow down | analyzer |
| observed | Seen happening in a recorded run | runtime trace |
| scenario | A hypothetical sequence over real nodes, used to explain ("what if…") and always labeled | explanation (ADR-0010) |

Labels are never a provenance. They annotate edges but never create them.

## Stack (proposed, not decided)

Tauri 2 desktop app: a Rust core for indexing, tracing, and the compiler, and a web UI for the graph. See Q-08. The LLM tool API is likely exposed as MCP so external agents can drive it too (ADR-0006 consequences).
