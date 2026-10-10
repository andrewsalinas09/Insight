# ADR-0023: The solver connects everything; anything that requires understanding is the model's job

- **Status:** Accepted; amended by ADR-0033 (language semantics are granted to the language plugin, which parses each language to a standard; library and domain meaning stays with the model)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("If something can change something it should be connected… The 400 needs semantic reasoning. It's up to the model. Let me be _very_ clear. Anything that requires understanding the code is the model's job. That's why this is even possible. Too many edge cases. The solver's job is to simply connect everything together")

## Context
Several earlier decisions quietly put *interpretation* into the deterministic backend:
- plugins classifying timing kinds (ADR-0018);
- a per-library effect catalogue (Q-20);
- the assurance "inventory" sorting code into validation and fallbacks (Q-31);
- React-specific rules for which effect a state change triggers.

Each is a catalogue of special cases, and real code has endless ones (a 400 ms debounce, a custom event bus, a homemade logger). The user drew the line.

## Decision
**The solver (deterministic backend) only connects.** It records, mechanically and completely:
- every function (ADR-0022) and every call;
- every piece of state, and every read and write of it. **If something can change something, they're connected.** A write to a value is connected to every read of that value, including reads in dependency arrays, closures, and other threads;
- bindings that come from the same source (e.g. `const [histVersion, setHistVersion] = useState(0)` binds both names to one state, so the setter is connected to every reader);
- containers and nesting (ADR-0008), and exact-match links across languages and wires (the same command string, the same JSON key, the same UUID constant);
- an arrow in and an arrow out for every node, with receipts (quotes) for each connection.

**The model does everything that requires understanding.** That includes:
- timing meaning (this is a debounce; it runs 400 ms later; it restarts on change);
- what an effect is (this writes to another program's config);
- validation vs. fallback vs. assertion;
- algorithm vs. plumbing;
- labels, groupings, and explanations;
- fuzzy or semantic matches.

It works over the solver's connections and cites them.

## Why
"That's why this is even possible." Understanding code means handling unbounded edge cases, which LLMs do well and rule catalogues never finish. Connecting code is mechanical, finite, and can be made complete and exact. Splitting the work this way gives each side what it's good at. The solver is never wrong because it never interprets. The model is never ungrounded because every statement it makes rides on connections the solver proved.

## Consequences
- **Amends ADR-0018 (time is explicit).** Time must still be explicit on screen, but the *timing kind* (sync / await / spawn / event / timer…) is a **model judgment with receipts**, not plugin classification. The solver supplies the connection and the quoted code (`setTimeout(…, 400)`, `.await`, `thread::spawn`), and the model names what it means.
- **Amends Q-20 / Q-31.** There's no effect catalogue and no deterministic "validation/fallback inventory". The solver returns *everything connected to the value*; the model sorts it into validation, fallbacks, tests, and gaps, citing each.
- **Narrows ADR-0013's compiler.** Claims are checked *structurally*: does the quoted code exist, and does the claimed connection exist in the solver's graph? "This is a debounce" is interpretation: shown as the model's, grounded in the cited connections, and not mechanically provable.
- **Plugins get simpler.** A language plugin is a parser plus name/reference resolution, with no framework semantics. React's `useEffect` needs no special rule: the dependency array *reads* `histVersion`, and the setter *writes* it, so they're connected.
- **The graph gets denser.** All read/write connections, locals included, live in the fact layer. Curation is the model's job (ADR-0022).
