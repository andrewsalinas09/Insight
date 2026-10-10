# ADR-0043: If a compiler could do it, it's allowed. `@insight` keywords are parsed; free text is read by Claude

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R2-06 ("it can 'understand' parses. Aka we can have keywords that parse. That's not intelligence. If a compiler can do it then it's ok if it's there basically. But remember that the code itself is parsed and Claude reads comments and docs like any other text")
- **Closes:** Q-15 (the annotation format). **Amends:** ADR-0005/0009 (claims + prose)

## Decision
- **The test for the tool side of ADR-0035:** *could a compiler do it?* Parsing, keyword grammars, resolving names, structural checks: all allowed. Interpreting free text is not.
- **Annotation grammar:** `@insight <keyword>: <value>` in a doc comment directly above a symbol (or in `.insight/` sidecars), parsed mechanically. **`tag`** is the first keyword (ADR-0032). New keywords are added through the wish rule (ADR-0039), each with a purely syntactic meaning.
- **Structural checks are allowed** where they're compiler-like (e.g. an `@insight calls: X` claim checked against `call` edges), built only if real use asks for them (ADR-0024).
- **Free-text comments and docs aren't parsed into meaning.** They're source text, which Claude reads like any other code (via `source`).

## Why
It sharpens ADR-0035's line. A compiler "understands" syntax completely and deterministically, so anything at that level belongs in the tool. Prose needs a model.

## Consequences
- Q-15 is closed: the format is the `@insight` keyword grammar. The "claims + prose checked by the compiler" design is retired, except compiler-like structural checks, which remain possible.
- ADR-0035 gains this one-line practical test.
