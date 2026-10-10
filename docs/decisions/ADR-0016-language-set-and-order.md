# ADR-0016: The supported language set and build order

- **Status:** Accepted; tooling per language decided by ADR-0034 and R2-15
- **Date:** 2026-10-09
- **Source:** user, discovery session 7. The core list is the user's; additions were proposed by Claude and accepted ("yes I agree with everything you said, add them")

## Context
ADR-0007 made languages plugins, and ADR-0014 brought in multi-device workspaces. We needed the actual set, and an order to build it in.

## Decision
**Supported set** (each a plugin, ADR-0007):

| Group | Languages | Role |
|---|---|---|
| Core | C/C++, Rust, Python, TypeScript/JavaScript, Go, SQL | User's committed list. SQL includes queries embedded in other languages' strings (TB-095) |
| Mobile / app | Kotlin/Java, Swift, C# | Phone and desktop sides of devices; JVM backends; Unity/.NET |
| Definition | Protocol Buffers, OpenAPI, GraphQL, JSON Schema | Not executed. They define wires, which makes wire links **proven** (ADR-0014) |
| Glue | Shell, Dockerfile/compose, CI workflows (YAML) | What wires processes and services together |
| UI markup | JSX/TSX, HTML, Vue/Svelte templates | Map "click this element" to its handler (Q-05) |
| Embedded flavors | Arduino (`.ino`), ESP-IDF, Zephyr, FreeRTOS | Conventions inside the C/C++ plugin, not separate plugins |

**Build order:**
1. TypeScript + Rust (+ JSX/TSX, + Tauri bridge): the Fletcher tracer bullet, and Insight itself.
2. Python, C/C++ (+ embedded flavors), Go, SQL.
3. Kotlin/Java, Swift, C#.
4. Definition and glue languages, added as soon as a wire link or workspace needs them. They may jump the queue when one does.

## Why
The core list covers what the user builds and what vibe-coded projects are mostly written in. Mobile languages complete the device → phone → cloud picture. Definition languages are the cheapest route from *inferred* to *proven* wire links. Glue and markup carry real edges (process wiring, UI triggers) that would otherwise be invisible. The order follows the tracer bullet (Fletcher is TS + Rust) and then the user's own stacks.

## Consequences
- Each plugin reports its static resolution rate (ADR-0015, TB-097).
- Several plugins are languages *inside* languages (SQL in strings, JSX in TS, templates in HTML), so the plugin interface must support embedded regions.
- Order is a plan, not a commitment of dates. Reshuffling it doesn't need a new ADR unless the *set* changes.
