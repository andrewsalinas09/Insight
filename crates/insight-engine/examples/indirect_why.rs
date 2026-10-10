//! `cargo run -p insight-engine --example indirect_why -- <snapshot.sqlite> <called state path> <candidate path suffix>`
//! runs the indirect-call walk (the solver's own code) from one called state and prints the
//! chain that reaches a candidate: for checking why it was found.
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let conn = insight_engine::open_snapshot(std::path::Path::new(&a[0]))?;
    let id = |p: &str| -> anyhow::Result<i64> { Ok(conn.query_row("SELECT nid FROM nodes WHERE path = ?1", [p], |r| r.get(0))?) };
    let path = |n: i64| -> String { conn.query_row("SELECT path FROM nodes WHERE nid = ?1", [n], |r| r.get(0)).unwrap_or_default() };
    let start = id(&a[1])?;
    let (found, parent) = insight_engine::IndirectWalk::load(&conn)?.candidates(start);
    println!("{} candidates", found.len());
    let Some(mut cur) = found.iter().copied().find(|n| path(*n).ends_with(&a[2])) else {
        println!("not a candidate");
        return Ok(());
    };
    while let Some((p, how)) = parent.get(&cur) {
        println!("{}  <- {how} ->  {}", path(cur), path(*p));
        if *p == start {
            break;
        }
        cur = *p;
    }
    Ok(())
}