//! Call-site matching: data that enters a shared helper at one call leaves through that
//! same call's result, not through every caller's.

use insight_engine::SnapshotWriter;
use insight_model::{FileFact, NodeFact, NodeKind, Span};
use insight_solver::path::{Follow, Options, path};

fn node(id: &str, parent: Option<&str>, kind: NodeKind) -> NodeFact {
    NodeFact { id: id.into(), parent: parent.map(String::from), kind, name: id.rsplit(" › ").next().unwrap().into(), span: Some(Span { byte_start: 0, byte_end: 1, line_start: 1, line_end: 1 }), anonymous: false, generated: false }
}

#[test]
fn data_returns_only_to_the_call_it_entered_by() {
    let dir = std::env::temp_dir().join(format!("insight-call-sites-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut w = SnapshotWriter::create(&dir.join("s.sqlite")).unwrap();
    w.file(&FileFact { path: "a.ts".into(), lang: "ts".into(), sha256: "0".into(), bytes: 1 }, b"x").unwrap();
    w.node(&node("a.ts", None, NodeKind::File), "ts", Some("a.ts")).unwrap();
    let id = |w: &mut SnapshotWriter, p: &str, parent: &str, k: NodeKind| w.node(&node(p, Some(parent), k), "ts", Some("a.ts")).unwrap();
    // helper(v) { return v } called by alice (with x, into ax) and by bob (with y, into by).
    let helper = id(&mut w, "a.ts › helper", "a.ts", NodeKind::Function);
    let v = id(&mut w, "a.ts › helper › v", "a.ts › helper", NodeKind::Var);
    let x = id(&mut w, "a.ts › x", "a.ts", NodeKind::Static);
    let ax = id(&mut w, "a.ts › ax", "a.ts", NodeKind::Static);
    let y = id(&mut w, "a.ts › y", "a.ts", NodeKind::Static);
    let by = id(&mut w, "a.ts › by", "a.ts", NodeKind::Static);
    w.edge_at("binds", Some("value"), x, Some(v), "exact", None, Some(1)).unwrap();
    w.edge_at("value-flow", Some("result"), helper, Some(ax), "exact", None, Some(1)).unwrap();
    w.edge_at("binds", Some("value"), y, Some(v), "exact", None, Some(2)).unwrap();
    w.edge_at("value-flow", Some("result"), helper, Some(by), "exact", None, Some(2)).unwrap();
    w.edge("value-flow", Some("return"), v, Some(helper), "exact", None).unwrap();
    let snap = w.finish().unwrap();
    let conn = insight_engine::open_snapshot(&snap).unwrap();
    let opts = Options { follow: vec![Follow::Data], max_occurrences: None, max_depth: None, schemas: vec![] };

    let to_own = path(&conn, &["a.ts › x".into()], &["a.ts › ax".into()], &opts).unwrap();
    assert!(to_own.has_hop("helper", "ax", "result"), "{:?}", to_own.hops);
    let to_other = path(&conn, &["a.ts › x".into()], &["a.ts › by".into()], &opts).unwrap();
    assert!(to_other.hops.is_empty(), "x must not reach bob's result: {:?}", to_other.hops);
    // Reverse queries match too: only x flows into ax.
    let into_ax = path(&conn, &[], &["a.ts › ax".into()], &opts).unwrap();
    assert!(into_ax.nodes.values().all(|p| !p.ends_with("› y")), "{:?}", into_ax.nodes);
    drop(conn);
    std::fs::remove_dir_all(dir).ok();
}
