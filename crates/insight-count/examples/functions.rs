//! `cargo run -p insight-count --example functions -- <file> [<from line> <to line>]` prints each
//! counted function's line range and kind, for checking a count mismatch by eye.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().ok_or_else(|| anyhow::anyhow!("usage: functions <file> [from to]"))?;
    let lang = insight_count::Lang::from_path(path).ok_or_else(|| anyhow::anyhow!("not a counted language"))?;
    let src = std::fs::read_to_string(path)?;
    let (from, to) = (args.get(1).and_then(|a| a.parse().ok()).unwrap_or(0usize), args.get(2).and_then(|a| a.parse().ok()).unwrap_or(usize::MAX));
    let line = |b: usize| src.as_bytes()[..b].iter().filter(|c| **c == b'\n').count() + 1;
    for e in insight_count::function_ends(lang, &src, &[])? {
        let l = line(e.saturating_sub(1));
        if (from..=to).contains(&l) {
            println!("ends {l}");
        }
    }
    println!("{}", insight_count::outline(lang, &src, from, to)?);
    Ok(())
}