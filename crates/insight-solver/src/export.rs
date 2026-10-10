//! Commit export (R3-06): a commit's exact bytes, read from git's object store
//! with `gix` (BQ-07 ruling), written into an Insight-owned tree (R3-02).
//!
//! Submodules are exported too, each at the commit the parent records, read from
//! the local git data (`.git/modules/<name>` or the submodule's own checkout).
//! Nothing is fetched: a submodule whose commit is not on this machine is listed
//! as skipped, and the project's build check then reports what it lacks. The
//! build needs submodule contents, and what the build compiles is the program.
//!
//! The tree is published atomically (temp folder, then rename), so a crash
//! never leaves a half-exported tree that looks complete.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct Exported {
    /// Full commit id.
    pub commit: String,
    pub dir: PathBuf,
    /// Repo-relative path -> git blob bytes length, for every exported file (submodules included).
    pub files: BTreeMap<String, u64>,
    /// Entries that were not exported (symlinks, unavailable submodules), as notes for the log.
    pub skipped: Vec<String>,
    /// Submodules exported: (path, commit).
    pub submodules: Vec<(String, String)>,
}

/// Resolves `rev` in the repo at `repo_path` to a full commit id.
pub fn resolve(repo_path: &Path, rev: &str) -> Result<String> {
    let repo = gix::open(repo_path).with_context(|| format!("opening git repo {}", repo_path.display()))?;
    let commit = repo.rev_parse_single(rev).with_context(|| format!("resolving `{rev}`"))?.object()?.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Exports `commit` into `dir` unless an identical export is already there.
pub fn export(repo_path: &Path, commit: &str, dir: &Path) -> Result<Exported> {
    let marker = dir.join(".insight-export");
    let reuse = std::fs::read_to_string(&marker).map(|s| s.trim() == commit).unwrap_or(false);
    let tmp = dir.with_extension("tmp");
    if !reuse {
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
    }
    let mut ex = Exported { commit: commit.to_string(), dir: dir.to_path_buf(), files: BTreeMap::new(), skipped: Vec::new(), submodules: Vec::new() };
    let repo = gix::open(repo_path)?;
    write_commit(&repo, repo_path, commit, "", if reuse { None } else { Some(&tmp) }, &mut ex)?;
    if !reuse {
        std::fs::write(tmp.join(".insight-export"), commit)?;
        std::fs::rename(&tmp, dir)?;
    }
    Ok(ex)
}

/// Writes one commit's tree under `prefix`, recursing into submodules.
fn write_commit(repo: &gix::Repository, worktree: &Path, commit: &str, prefix: &str, out: Option<&Path>, ex: &mut Exported) -> Result<()> {
    let id = gix::ObjectId::from_hex(commit.as_bytes())?;
    let tree = repo.find_object(id)?.peel_to_commit()?.tree()?;
    let mut rec = gix::traverse::tree::Recorder::default();
    tree.traverse().breadthfirst(&mut rec)?;
    for e in rec.records {
        let path = format!("{prefix}{}", e.filepath);
        if e.mode.is_tree() {
            continue;
        }
        if e.mode.is_commit() {
            let local = e.filepath.to_string();
            match submodule_repo(repo, worktree, &local) {
                Some((sub, sub_worktree)) if sub.find_object(e.oid).is_ok() => {
                    ex.submodules.push((path.clone(), e.oid.to_string()));
                    write_commit(&sub, &sub_worktree, &e.oid.to_string(), &format!("{path}/"), out, ex)?;
                }
                _ => ex.skipped.push(format!("{path} (submodule at {}: not on this machine; run `git submodule update --init --recursive` in the repo)", e.oid)),
            }
            continue;
        }
        if !e.mode.is_blob() {
            ex.skipped.push(format!("{path} ({:?})", e.mode.kind()));
            continue;
        }
        let obj = repo.find_object(e.oid)?;
        ex.files.insert(path.clone(), obj.data.len() as u64);
        if let Some(out) = out {
            let dest = out.join(&path);
            std::fs::create_dir_all(dest.parent().expect("has parent"))?;
            std::fs::write(&dest, &obj.data)?;
        }
    }
    Ok(())
}

/// The git data of a submodule at `path` (as git stores it): `.git/modules/<path>`, or a checkout's own `.git`.
fn submodule_repo(parent: &gix::Repository, worktree: &Path, path: &str) -> Option<(gix::Repository, PathBuf)> {
    let sub_worktree = worktree.join(path);
    let candidates = [parent.git_dir().join("modules").join(path), sub_worktree.join(".git")];
    candidates.iter().find_map(|c| gix::open(c).ok()).map(|r| (r, sub_worktree))
}
