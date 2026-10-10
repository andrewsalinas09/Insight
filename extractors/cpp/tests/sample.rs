//! End-to-end: a small made-up C++ program through the extractor (needs libclang).

use std::process::Command;

use insight_model::{NodeKind, Record, Resolution};

const SAMPLE: &str = r#"
#include <cstdlib>
#define MAKE_GETTER(name) int get_##name() { return 7; }
#define PACKET_KIND 0x05

struct Stage {
    Stage() = default;
    virtual ~Stage() = default;
    virtual int process(int x) = 0;
};
struct Double : Stage { int process(int x) override { return x * 2; } };
struct Square : Stage { int process(int x) override { return x * x; } };

MAKE_GETTER(seven)

static int compare(const void* a, const void* b) { return *(const int*)a - *(const int*)b; }

int run(Stage* s, int* xs, int n) {
    std::qsort(xs, n, sizeof(int), compare);
    auto twice = [](int v) { return v + v; };
#if 0
    int never() { return 0; }
#endif
    if (n > 0) {
        return s->process(xs[0]) + twice(PACKET_KIND) + get_seven();
    } else {
        return 0;
    }
}
"#;

#[test]
fn extracts_a_small_cpp_program() {
    let dir = std::env::temp_dir().join(format!("insight-cpp-sample-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("sample.cpp");
    std::fs::write(&src, SAMPLE).unwrap();
    let db = serde_json::json!([{ "directory": dir, "file": src, "arguments": ["clang++", "-std=c++17", "-c", src] }]);
    std::fs::write(dir.join("compile_commands.json"), db.to_string()).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_insight-extract-cpp"))
        .arg("--root")
        .arg(&dir)
        .arg("--compdb")
        .arg(dir.join("compile_commands.json"))
        .arg("--schema")
        .arg(insight_model::FACT_SCHEMA_VERSION.to_string())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let recs: Vec<Record> = String::from_utf8(out.stdout).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    let nodes: Vec<_> = recs.iter().filter_map(|r| if let Record::Node(n) = r { Some(n) } else { None }).collect();
    let edges: Vec<_> = recs.iter().filter_map(|r| if let Record::Edge(e) = r { Some(e) } else { None }).collect();
    let fns: Vec<&str> = nodes.iter().filter(|n| n.kind == NodeKind::Function).map(|n| n.id.as_str()).collect();

    // Bodies only: `= default` and pure virtual are not functions; the `#if 0` one is not compiled.
    for want in ["sample.cpp › Double › process", "sample.cpp › Square › process", "sample.cpp › compare", "sample.cpp › run", "sample.cpp › run › twice#1", "sample.cpp › get_seven"] {
        assert!(fns.contains(&want), "missing {want} in {fns:?}");
    }
    assert!(!fns.iter().any(|f| f.contains("Stage") || f.contains("never")), "{fns:?}");
    // The macro-made function is in the graph but marked generated.
    assert!(nodes.iter().any(|n| n.id.ends_with("get_seven") && n.generated));
    assert!(nodes.iter().any(|n| n.kind == NodeKind::Inactive));

    // Virtual dispatch: both overrides, as possible candidates of one call.
    let dispatch: Vec<_> = edges.iter().filter(|e| e.from == "sample.cpp › run" && e.resolution == Resolution::Possible).collect();
    assert_eq!(dispatch.len(), 2, "{dispatch:?}");
    assert!(dispatch.iter().all(|e| e.group == dispatch[0].group));
    // A function handed to external code.
    assert!(edges.iter().any(|e| e.subkind.as_deref() == Some("handoff") && e.to.as_deref() == Some("sample.cpp › compare")));
    assert!(edges.iter().any(|e| e.to.as_deref().is_some_and(|t| t.starts_with("ext:cpp:") && t.ends_with("qsort"))));
    // Data flow: the argument of a virtual call binds to each override's parameter (possible);
    // parameters and locals are state; `twice` is written by its initializer.
    let binds: Vec<_> = edges.iter().filter(|e| e.kind == insight_model::EdgeKind::Binds && e.to.as_deref().is_some_and(|t| t.ends_with("process › x"))).collect();
    assert_eq!(binds.len(), 2, "{binds:?}");
    assert!(binds.iter().all(|e| e.resolution == Resolution::Possible));
    assert!(nodes.iter().any(|n| n.id == "sample.cpp › run › xs" && n.kind == NodeKind::Var));
    assert!(edges.iter().any(|e| e.kind == insight_model::EdgeKind::Write && e.to.as_deref() == Some("sample.cpp › run › twice")));
    // A number literal through a macro, canonical decimal.
    assert!(recs.iter().any(|r| matches!(r, Record::Literal(l) if l.number && l.value == "5")));
    std::fs::remove_dir_all(dir).ok();
}
