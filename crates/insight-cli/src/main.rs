//! `insight.exe` (ADR-0031). Every command writes a log file; the terminal
//! shows only the log's path and the exit code (R2-20). Refusals have their
//! own exit codes (R3-10): 2 needs init/settings, 3 needs approval, 5 does not
//! build, 6 tripwire, 7 count mismatch; 1 is any other error.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use insight_solver::{BuildOptions, Log, Refusal};

#[derive(Parser)]
#[command(name = "insight", version, about = "Insight: see and check code")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a project from one or more folders; each folder's git repo is found by following `.git` upward.
    Init { project: String, folders: Vec<PathBuf> },
    /// Approve the project's current install/check commands (R3-01).
    Approve { project: String },
    /// Build a snapshot of every repo in the project.
    Build {
        project: String,
        /// The commit (or any git revision) to snapshot.
        #[arg(long)]
        commit: Option<String>,
    },
    /// Reachability with receipts (R2-12). You choose the hop kinds; there is no default (ADR-0047).
    Path {
        project: String,
        /// Start node: a full id or a unique suffix (e.g. "HistoryTree › nodeUp").
        from: String,
        /// End node(s); omit for everything reachable.
        #[arg(long)]
        to: Vec<String>,
        /// Comma-separated hop kinds: call, handoff, literal, names.
        #[arg(long)]
        follow: String,
        /// Skip literals that occur more often than this.
        #[arg(long)]
        max_occurrences: Option<usize>,
        /// Keep only routes of at most this many hops (needs --to).
        #[arg(long)]
        max_depth: Option<usize>,
        /// A specific snapshot file (default: the project's newest).
        #[arg(long)]
        snapshot: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // `path` is a query: its answer goes to stdout (the log rule, R2-20, is for builds).
    if let Cmd::Path { project, from, to, follow, max_occurrences, max_depth, snapshot } = &cli.cmd {
        return match run_path(project, from, to, follow, *max_occurrences, *max_depth, snapshot.as_deref()) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e:#}");
                ExitCode::from(1)
            }
        };
    }
    let (log, result) = match cli.cmd {
        Cmd::Init { project, folders } => {
            let mut log = match insight_solver::paths::data_root().and_then(|r| Log::create_in(&r.join("logs"), "init")) {
                Ok(l) => l,
                Err(e) => return fatal(e),
            };
            let r = insight_solver::init(&project, &folders, &mut log).map(|_| ());
            (log, r)
        }
        Cmd::Approve { project } => with_log(&project, "approve", |log| insight_solver::approve(&project, log)),
        Cmd::Path { .. } => unreachable!("handled above"),
        Cmd::Build { project, commit } => with_log(&project, "build", |log| {
            let built = insight_solver::build(&project, &BuildOptions { commit }, log)?;
            for b in built {
                log.line(format!("OK {}: {} functions -> {}", b.repo, b.functions, b.snapshot.display()));
            }
            Ok(())
        }),
    };
    let mut log = log;
    let code = match result {
        Ok(()) => {
            log.line("result: ok");
            0
        }
        Err(e) => {
            let code = e.downcast_ref::<Refusal>().map(Refusal::exit_code).unwrap_or(1);
            log.line(format!("result: refused ({code})\n{e:#}"));
            code
        }
    };
    println!("{}", log.path.display());
    println!("exit {code}");
    ExitCode::from(code as u8)
}

fn with_log(project: &str, verb: &str, f: impl FnOnce(&mut Log) -> anyhow::Result<()>) -> (Log, anyhow::Result<()>) {
    // A project that doesn't exist yet still gets a log, under the data root.
    let log = Log::create(project, verb)
        .or_else(|_| insight_solver::paths::data_root().and_then(|r| Log::create_in(&r.join("logs"), verb)));
    let mut log = match log {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot create a log file: {e:#}");
            std::process::exit(1);
        }
    };
    let r = f(&mut log);
    (log, r)
}

fn run_path(
    project: &str,
    from: &str,
    to: &[String],
    follow: &str,
    max_occurrences: Option<usize>,
    max_depth: Option<usize>,
    snapshot: Option<&std::path::Path>,
) -> anyhow::Result<String> {
    let follow = insight_solver::path::Follow::parse_list(follow)?;
    let snap = match snapshot {
        Some(s) => s.to_path_buf(),
        None => insight_solver::latest_snapshot(project)?,
    };
    let conn = insight_engine::open_snapshot(&snap)?;
    let opts = insight_solver::path::Options { follow, max_occurrences, max_depth };
    let r = insight_solver::path::path(&conn, from, to, &opts)?;
    Ok(format!("snapshot: {}\n{}", snap.display(), insight_solver::path::render(&conn, &r)?))
}

fn fatal(e: anyhow::Error) -> ExitCode {
    eprintln!("cannot create a log file: {e:#}");
    ExitCode::from(1)
}
