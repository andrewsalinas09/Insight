//! `cargo run -p insight-count --example count -- <file>…` prints per-file function counts.
fn main() -> anyhow::Result<()> {
    let mut total = 0;
    for path in std::env::args().skip(1) {
        let Some(lang) = insight_count::Lang::from_path(&path) else { continue };
        let n = insight_count::count_functions(lang, &std::fs::read_to_string(&path)?)?;
        println!("{n:6} {path}");
        total += n;
    }
    println!("{total:6} total");
    Ok(())
}
