//! Commit export (R3-06): a commit's exact bytes, read from git's object store
//! with `gix` (BQ-07 ruling), written into an Insight-owned tree (R3-02).
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
    /// Repo-relative path -> git blob bytes length, for every exported file.
    pub files: BTreeMap<String, u64>,
    /// Entries that were not exported (submodules, symlinks), as notes for the log.
    pub skipped: Vec<String>,
}

/// Resolves `rev` in the repo at `repo_path` to a full commit id.
pub fn resolve(repo_path: &Path, rev: &str) -> Result<String> {
    let repo = gix::open(repo_path).with_context(|| format!("opening git repo {}", repo_path.display()))?;
    let commit = repo.rev_parse_single(rev).with_context(|| format!("resolving `{rev}`"))?.object()?.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Exports `commit` into `dir` unless an identical export is already there.
pub fn export(repo_path: &Path, commit: &str, dir: &Path) -> Result<Exported> {
    let repo = gix::open(repo_path)?;
    let id = gix::ObjectId::from_hex(commit.as_bytes())?;
    let tree = repo.find_object(id)?.peel_to_commit()?.tree()?;
    let mut rec = gix::traverse::tree::Recorder::default();
    tree.traverse().breadthfirst(&mut rec)?;

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

    let mut files = BTreeMap::new();
    let mut skipped = Vec::new();
    for e in rec.records {
        let path = e.filepath.to_string();
        if e.mode.is_tree() {
            continue;
        }
        if !e.mode.is_blob() {
            skipped.push(format!("{path} ({:?})", e.mode.kind()));
            continue;
        }
        let obj = repo.find_object(e.oid)?;
        files.insert(path.clone(), obj.data.len() as u64);
        if !reuse {
            let out = tmp.join(&path);
            std::fs::create_dir_all(out.parent().expect("has parent"))?;
            std::fs::write(&out, &obj.data)?;
        }
    }
    if !reuse {
        std::fs::write(tmp.join(".insight-export"), commit)?;
        std::fs::rename(&tmp, dir)?;
    }
    Ok(Exported { commit: commit.to_string(), dir: dir.to_path_buf(), files, skipped })
}
