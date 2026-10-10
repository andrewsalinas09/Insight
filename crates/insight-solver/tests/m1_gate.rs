//! The M1 gate (R3-12): required function→function hops for the drag-and-drop
//! walkthrough (`docs/examples/fletcher-history-graft-worked-example.md`), on a
//! snapshot of Fletcher at `cd5d6ff`. Every hop is found by generic facts only:
//! calls, one-hop function-value flow, handoffs, literals and `names` (ADR-0048).
//!
//! Run after `insight build Fletcher --commit cd5d6ff`:
//! `INSIGHT_GATE_SNAPSHOT=<snapshot.sqlite> cargo test -p insight-solver --test m1_gate -- --ignored`

use insight_solver::path::{Follow, Options, PathResult, path};

fn snapshot() -> insight_engine::rusqlite::Connection {
    let p = std::env::var("INSIGHT_GATE_SNAPSHOT").expect("set INSIGHT_GATE_SNAPSHOT to a Fletcher cd5d6ff snapshot");
    insight_engine::open_snapshot(std::path::Path::new(&p)).expect("open snapshot")
}

fn require(r: &PathResult, hops: &[(&str, &str, &str)]) {
    let missing: Vec<String> = hops.iter().filter(|(f, t, l)| !r.has_hop(f, t, l)).map(|(f, t, l)| format!("{f} --{l}--> {t}")).collect();
    assert!(missing.is_empty(), "missing required hops:\n{}", missing.join("\n"));
}

#[test]
#[ignore = "needs a Fletcher cd5d6ff snapshot (INSIGHT_GATE_SNAPSHOT)"]
fn drag_reaches_graft_node_in_both_windows() {
    let conn = snapshot();
    let opts = Options { follow: vec![Follow::Call, Follow::Handoff, Follow::Literal], max_occurrences: Some(20), max_depth: Some(6) };
    let r = path(&conn, &["HistoryTree › nodeUp".into()], &["MainApp › graftNode".into()], &opts).unwrap();
    require(
        &r,
        &[
            // Main window: `onGraft={graftNode}` (one-hop value flow through the prop).
            ("HistoryTree › nodeUp › =>#1", "MainApp › graftNode", "call (possible)"),
            // Pop-out window: the `onGraft` arrow, then the event bus.
            ("HistoryTree › nodeUp › =>#1", "PopoutHistory › onGraft#1", "call (possible)"),
            ("PopoutHistory › onGraft#1", "lit:\"hist-cmd\"", "contains literal"),
            ("lit:\"hist-cmd\"", "MainApp › useEffect#15", "literal"),
            ("MainApp › useEffect#15", "useEffect#15 › listen#2", "handoff"),
            ("useEffect#15 › listen#2", "MainApp › cmd#2", "call (possible)"),
            ("MainApp › cmd#2", "MainApp › graftNode", "call"),
        ],
    );
}

#[test]
#[ignore = "needs a Fletcher cd5d6ff snapshot (INSIGHT_GATE_SNAPSHOT)"]
fn graft_node_reaches_the_file_write_across_languages() {
    let conn = snapshot();
    let opts = Options { follow: vec![Follow::Call, Follow::Names], max_occurrences: None, max_depth: None };
    let r = path(&conn, &["MainApp › graftNode".into()], &["fsx.rs › write_atomic".into()], &opts).unwrap();
    require(
        &r,
        &[
            ("MainApp › graftNode", "MainApp › applySnap", "call"),
            ("MainApp › applySnap", "MainApp › pushChain", "call"),
            ("MainApp › pushChain", "pushChain › send", "call"),
            // The language boundary: a string literal equal to a declaration's name.
            ("pushChain › send", "lit:\"set_fletcher_chain\"", "contains literal"),
            ("lit:\"set_fletcher_chain\"", "app/src-tauri/src/lib.rs › set_fletcher_chain", "names"),
            ("app/src-tauri/src/lib.rs › set_fletcher_chain", "crates/fletcher-core/src/fsx.rs › write_atomic", "call"),
        ],
    );
}
