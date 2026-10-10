//! Which projects inside a repo get extracted (R2-26: all of them).
//!
//! Language-level markers only (ADR-0048): a Cargo manifest with a `[package]`
//! is a Rust package; a `tsconfig*.json` is a TypeScript project. Sorted, so
//! discovery is deterministic.

use std::collections::BTreeMap;

pub struct Projects {
    /// Folders (repo-relative) holding a Rust package.
    pub rust_packages: Vec<String>,
    /// tsconfig files (repo-relative).
    pub ts_projects: Vec<String>,
}

/// `files` = every file path in the snapshot, with any byte payload accessor.
pub fn discover(files: &BTreeMap<String, u64>, read: impl Fn(&str) -> Option<String>) -> Projects {
    let mut rust_packages = Vec::new();
    let mut ts_projects = Vec::new();
    for path in files.keys() {
        if path.split('/').any(|seg| seg == "node_modules") {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or(path);
        if name == "Cargo.toml" {
            let is_package = read(path).is_some_and(|t| t.lines().any(|l| l.trim() == "[package]"));
            if is_package {
                let dir = path.strip_suffix("Cargo.toml").unwrap_or("").trim_end_matches('/');
                rust_packages.push(if dir.is_empty() { ".".to_string() } else { dir.to_string() });
            }
        } else if name.starts_with("tsconfig") && name.ends_with(".json") {
            ts_projects.push(path.clone());
        }
    }
    Projects { rust_packages, ts_projects }
}
