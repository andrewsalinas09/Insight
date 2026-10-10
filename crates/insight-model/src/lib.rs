//! The standard representation (ADR-0033, ADR-0048).
//!
//! Every language extractor translates its language into these programming
//! concepts and nothing else. There is no language- or framework-specific kind
//! in this crate; language knowledge lives only in the extractors (ADR-0034).
//!
//! Extractors stream these records as newline-delimited JSON (R2-28):
//! one `header`, then facts, then an `end` record whose counts must match.
//! `docs/SCHEMA.md` §9 is the normative description.

use serde::{Deserialize, Serialize};

/// Version of the fact schema. Part of every snapshot cache key (R2-28).
pub const FACT_SCHEMA_VERSION: u32 = 3;

/// One line of an extractor's NDJSON stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    Header(Header),
    File(FileFact),
    Node(NodeFact),
    Edge(EdgeFact),
    Literal(LiteralFact),
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

/// A node in the graph: a container (BQ-09, ADR-0008) or an external leaf (BQ-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeFact {
    /// Containers: location path `file › container › …` (BQ-10, R3-G #1).
    /// Externals: `ext:<lang>:<module>:<qualified name>`.
    pub id: String,
    /// Id of the lexically enclosing node; `None` for files and externals.
    pub parent: Option<String>,
    pub kind: NodeKind,
    /// Display name. Anonymous functions are named by position (R3-G #1).
    pub name: String,
    /// Absent for externals (they have no source in the snapshot).
    #[serde(default)]
    pub span: Option<Span>,
    /// Named by position, not by a declared name (`map#1`, `=>#2`). `names` links skip these.
    #[serde(default)]
    pub anonymous: bool,
    /// Produced by macro expansion from the macro's own template, not written at its
    /// location (R4-B). Kept in the graph; not counted by the M0 check (SCHEMA.md §6).
    #[serde(default)]
    pub generated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Module,
    /// struct, class, enum, interface, trait, union, impl block.
    Type,
    /// Every function, named or anonymous, with a body (ADR-0022).
    Function,
    /// A control-flow region: branch arm, loop body, try/catch/finally (BQ-18).
    Block,
    /// A referenced symbol outside the analyzed code: one leaf per symbol (BQ-20).
    External,
    /// Source text the build configuration excluded (an inactive `#if` region, P9):
    /// recorded, never analyzed.
    Inactive,
}

/// A byte range plus 1-based lines, in the UTF-8 bytes of the snapshot file (BQ-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
}

/// An edge between two nodes, with its receipts (BQ-09, BQ-12, R2-30).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeFact {
    pub kind: EdgeKind,
    #[serde(default)]
    pub subkind: Option<String>,
    pub from: String,
    /// `None` = absent (a `possible` "may call nothing") or unknown (`unresolved`).
    pub to: Option<String>,
    pub resolution: Resolution,
    /// Ties one call's `possible` candidates together (extractor-local number).
    #[serde(default)]
    pub group: Option<u64>,
    pub receipts: Vec<Receipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Call,
    /// `subkind = "handoff"`: a function value passed to external code (BQ-16 as amended).
    /// `subkind = "returns"`: a function returns this function literal (BQ-15, R3-G #5).
    ValueFlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Exact,
    Possible,
    Unresolved,
}

/// Where a fact comes from in the source (BQ-12). Text is computed from the span (R3-G #8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    /// `call`, `value` (the write that supplied a candidate), `arg`, `literal`, `macro`.
    pub role: String,
    pub file: String,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line: u32,
    /// 1-based UTF-8 byte column.
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// Innermost container (blocks included).
    pub container: Option<String>,
}

/// One occurrence of a string or number literal in the source. The solver turns these
/// into literal nodes and the generic `link/literal` and `link/names` edges (BQ-22, ADR-0048).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiteralFact {
    /// The string's value, or for a number its canonical decimal form (`0x1F` → `31`).
    pub value: String,
    /// True for a number literal (integers; `lit:31`), false for a string (`lit:"31"`).
    #[serde(default)]
    pub number: bool,
    /// The innermost function (or file) containing it.
    pub from: String,
    pub receipt: Receipt,
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
    #[serde(default)]
    pub edges: u64,
    #[serde(default)]
    pub literals: u64,
    pub diagnostics: u64,
}
