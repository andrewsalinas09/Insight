//! The standard representation (ADR-0033, ADR-0048).
//!
//! Every language extractor translates its language into these programming
//! concepts and nothing else. There is no language- or framework-specific kind
//! in this crate; language knowledge lives only in the extractors (ADR-0034).
//!
//! Extractors stream these records as newline-delimited JSON (R2-28):
//! one `header`, then facts, then an `end` record whose counts must match.

use serde::{Deserialize, Serialize};

/// Version of the fact schema. Part of every snapshot cache key (R2-28).
pub const FACT_SCHEMA_VERSION: u32 = 1;

/// One line of an extractor's NDJSON stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    Header(Header),
    File(FileFact),
    Node(NodeFact),
    Diagnostic(Diagnostic),
    End(End),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    /// Must equal [`FACT_SCHEMA_VERSION`].
    pub schema: u32,
    /// Extractor name and build hash, e.g. `ts@<hash>`.
    pub extractor: String,
    /// The toolchain that produced the facts, e.g. `typescript 5.8.3`.
    pub toolchain: String,
}

/// A source file the extractor actually read, with the hash of the bytes it
/// read (R3-05: a snapshot is exactly the files read).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileFact {
    /// Repo-relative, forward-slash, case-exact path (BQ-11).
    pub path: String,
    pub lang: String,
    pub sha256: String,
    pub bytes: u64,
}

/// A node in the graph. M0 emits containers only (ADR-0008, BQ-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeFact {
    /// Location path: `file › container › …` (BQ-10, R3-G #1).
    pub id: String,
    /// Id of the lexically enclosing node; `None` for a file.
    pub parent: Option<String>,
    pub kind: NodeKind,
    /// Display name. Anonymous functions are named by position (R3-G #1).
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Module,
    /// struct, class, enum, interface, trait, union.
    Type,
    /// Every function, named or anonymous, with a body (ADR-0022).
    Function,
    /// A control-flow region: branch arm, loop body, try/catch/finally (BQ-18).
    Block,
}

/// A byte range plus 1-based lines, in the UTF-8 bytes of the snapshot file (BQ-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: Option<String>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Closing record. Its counts must equal what was streamed, which detects a
/// truncated or crashed extractor (R2-28, R2-05).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct End {
    pub files: u64,
    pub nodes: u64,
    pub diagnostics: u64,
}
