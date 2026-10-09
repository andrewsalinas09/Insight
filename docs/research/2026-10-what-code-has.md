# What code "has": a catalogue for Q-20 (data and boundary edges)

Written 2026-10-09 to answer the user's "I don't know what I don't know." This is a menu of everything a codebase contains beyond containers and control flow (ADR-0008), with what a learner gets from each and whether it can be proven deterministically (ADR-0002). It is not a decision.

Legend for **Provable**: ✅ statically provable in typed languages · ◐ partly or per-language · ▶ needs a runtime trace (ADR-0003)

## 1. Data: where values come from and go

| Thing | Example | What the learner gets | Provable |
|---|---|---|---|
| Inputs / outputs | params, return type, `Result<T, E>` | The bubble's interface: what goes in, what comes out | ✅ |
| Field read / write | `self.count += 1` | Which functions touch which state | ✅ |
| Read vs. mutate | `&self` vs `&mut self`, `const` | "This only looks; that one changes things" | ✅ (typed) ◐ (dynamic) |
| Construction | `Parser::new()`, `new Foo()` | Where objects are born | ✅ |
| Ownership / move / borrow / clone | Rust moves, `Rc`, `Arc` | Who owns what, and who keeps it alive | ✅ Rust, n/a elsewhere |
| Value flow (def-use) | `let x = load(); save(x)` | "Where does this value come from?", tracing a value backwards | ◐ (local is easy, across functions is harder) |
| Shared / global state | statics, singletons, module globals | Hidden coupling, a classic vibe-coding hazard | ✅ |
| Framework-managed state | Tauri `State<T>`, DI containers, React context, Redux/Zustand stores | Data that appears with no visible caller (TB-041) | ◐ (needs framework plugins) |
| Access paths through locks | `state.core.lock().model.items` | How a function reaches its members (TB-042) | ✅ |

## 2. Types: how things relate

| Thing | Example | Learner value | Provable |
|---|---|---|---|
| Implements / extends | `impl Storage for Disk`, `class A extends B` | Why `storage.save()` has three possible targets (TB-002) | ✅ |
| Type-of / uses type | a field of type `Settings` | The data-model map: what contains what | ✅ |
| Generics instantiation | `Vec<Item>` | Usually noise; hide by default | ✅ |

## 3. Concurrency (the user knows this domain, at least in Rust)

| Thing | Example | Learner value | Provable |
|---|---|---|---|
| Spawn | `tokio::spawn`, `thread::spawn`, `setTimeout` | Execution forks and continues elsewhere (TB-010) | ✅ site, ▶ timing |
| Await / join | `.await`, `join()` | Where execution waits | ✅ |
| Lock acquire / release | `mutex.lock()` | What's guarded; deadlock risk; lock held across await | ✅ |
| Channels / messages | `tx.send` → `rx.recv` | Implicit edges between tasks | ◐ (pairing send/recv) |
| Atomics | `AtomicBool` | Lock-free shared state | ✅ |

## 4. Boundaries: where execution leaves the code you have

Two very different kinds:

**4a. Links: to *other code we also have*.** Insight can connect both ends into one graph.

| Thing | Example | Provable |
|---|---|---|
| IPC | Tauri `invoke("x")` ↔ `#[command] fn x`, Electron IPC | ◐ string match, verifiable |
| HTTP / RPC within the repo | `fetch("/api/x")` ↔ route handler | ◐ |
| Events / pub-sub | `emit("saved")` ↔ `listen("saved")` | ◐ |
| FFI / WASM | Rust ↔ C, JS ↔ WASM | ✅ declared, ◐ wiring |
| Subprocess of own tool | `spawn("our-cli", …)` | ◐ |

**4b. Effects: to *the outside world*.** These are the ends of the line, and probably the most important thing to a learner: what does this click actually do?

| Effect | Example |
|---|---|
| Filesystem | read/write/delete files |
| Network | HTTP to external APIs, sockets |
| Database | SQL queries, ORM calls |
| Process / OS | spawn programs, env vars, signals |
| UI | render, update DOM, show dialog |
| Time / randomness | clocks, timers, RNG, which cause nondeterminism (TB-036) |
| Devices / hardware | serial, USB, audio, sensors |
| External AI | LLM API calls (common in vibe-coded apps) |

Effects are mostly provable by recognizing known library calls (`std::fs`, `fetch`, `sqlx`). A per-library "effect catalogue" fits the plugin model (ADR-0007).

## 5. Around the code (meta)

| Thing | Learner value |
|---|---|
| Tests → what they exercise | "Is this path tested?"; tests are also free dynamic traces (Q-17) |
| Config / feature flags / `cfg` | Branches decided outside the code (TB-011) |
| Build / entry points | `main`, routes, CLI definitions: the trigger list (Q-05) |
| Dependencies | Leaves by default; scope boundary (TB-018) |
| Git history | When and why something changed; which AI session wrote it |
| Comments / existing docs | Input to annotations; also a source of drift |

## A possible v1 cut (for discussion)

Containers + control (ADR-0008), plus **inputs/outputs, field read/write, construction, implements**, plus **effects (4b)**, plus **links (4a) for one bridge (Tauri)**. Everything else waits. Rationale: inputs/outputs make bubbles meaningful, field access answers "where does it get its members", effects answer "what does it *do*", and the Tauri link lets Insight read itself.
