//! Cross-repo `path` on two made-up snapshots: a producer repo and a consumer repo that
//! both compile the same shared header, and share one string literal.

use insight_engine::SnapshotWriter;
use insight_model::{FileFact, NodeFact, NodeKind, Receipt, Span};
use insight_solver::path::{Follow, Options, path};

fn span(s: u64, e: u64) -> Span {
    Span { byte_start: s, byte_end: e, line_start: 1, line_end: 1 }
}

fn node(id: &str, parent: Option<&str>, kind: NodeKind, s: u64, e: u64) -> NodeFact {
    NodeFact { id: id.into(), parent: parent.map(String::from), kind, name: id.rsplit(" › ").next().unwrap().into(), span: Some(span(s, e)), anonymous: false, generated: false }
}

fn receipt(file: &str, s: u64, e: u64) -> Receipt {
    Receipt { role: "x".into(), file: file.into(), byte_start: s, byte_end: e, line: 1, col: s as u32 + 1, end_line: 1, end_col: e as u32 + 1, container: None }
}

/// One repo: its own source file plus the shared header (identical bytes in both repos).
fn snapshot(dir: &std::path::Path, repo: &str, own: &str, own_fn: &str, write: bool) -> std::path::PathBuf {
    let shared = b"struct Packet { int value; };";
    let src = format!("void {own_fn}() {{ p.value; \"topic\"; }}");
    let mut w = SnapshotWriter::create(&dir.join(format!("{repo}.sqlite"))).unwrap();
    w.meta("repo", repo).unwrap();
    for (path, bytes) in [("include/packet.h", shared.as_slice()), (own, src.as_bytes())] {
        let sha: String = {
            use sha2::Digest;
            sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
        };
        w.file(&FileFact { path: path.into(), lang: "cpp".into(), sha256: sha, bytes: bytes.len() as u64 }, bytes).unwrap();
        w.node(&node(path, None, NodeKind::File, 0, bytes.len() as u64), "cpp", Some(path)).unwrap();
    }
    w.node(&node("include/packet.h › Packet", Some("include/packet.h"), NodeKind::Type, 0, 28), "cpp", Some("include/packet.h")).unwrap();
    let field = w.node(&node("include/packet.h › Packet › value", Some("include/packet.h › Packet"), NodeKind::Field, 16, 26), "cpp", Some("include/packet.h")).unwrap();
    let f = w.node(&node(&format!("{own} › {own_fn}"), Some(own), NodeKind::Function, 0, src.len() as u64), "cpp", Some(own)).unwrap();
    let eid = if write { w.edge("write", None, f, Some(field), "exact", None).unwrap() } else { w.edge("read", None, f, Some(field), "exact", None).unwrap() };
    w.receipt(eid, &receipt(own, 10, 17)).unwrap();
    let lit = w.raw_node("lit:\"topic\"", None, "literal", None, "topic", false, None, None, None).unwrap();
    let eid = w.edge("link", Some("literal"), f, Some(lit), "exact", None).unwrap();
    w.receipt(eid, &receipt(own, 19, 26)).unwrap();
    w.finish().unwrap()
}

#[test]
fn routes_cross_repos_through_shared_code_and_literals() {
    let dir = std::env::temp_dir().join(format!("insight-multi-repo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = snapshot(&dir, "producer", "src/sender.cpp", "fill", true);
    let b = snapshot(&dir, "consumer", "src/receiver.cpp", "handle", false);
    let conn = insight_engine::open_snapshot(&a).unwrap();
    conn.execute("ATTACH DATABASE ?1 AS consumer", [b.to_string_lossy()]).unwrap();
    let schemas = vec!["main".to_string(), "consumer".to_string()];

    // Data: the producer writes the shared field; the same field in the consumer is read by `handle`.
    let opts = Options { follow: vec![Follow::Write, Follow::Same, Follow::Read], max_occurrences: None, max_depth: None, schemas: schemas.clone() };
    let r = path(&conn, &["sender.cpp › fill".into()], &["consumer:receiver.cpp › handle".into()], &opts).unwrap();
    assert!(r.has_hop("fill", "packet.h › Packet › value", "write"), "{:?}", r.hops);
    assert!(r.has_hop("main:include/packet.h › Packet › value", "consumer:include/packet.h › Packet › value", "same code"));
    assert!(r.has_hop("consumer:include/packet.h › Packet › value", "handle", "read by"));

    // Literal: one node for "topic" across both repos.
    let opts = Options { follow: vec![Follow::Literal], max_occurrences: None, max_depth: None, schemas };
    let r = path(&conn, &["sender.cpp › fill".into()], &["consumer:receiver.cpp › handle".into()], &opts).unwrap();
    assert!(r.has_hop("fill", "lit:\"topic\"", "contains literal"), "{:?}", r.hops);
    assert!(r.has_hop("lit:\"topic\"", "handle", "literal"));
    drop(conn);
    std::fs::remove_dir_all(dir).ok();
}
