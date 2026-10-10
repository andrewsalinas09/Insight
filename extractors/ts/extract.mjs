// Insight TypeScript/JavaScript extractor (ADR-0034, R2-28).
//
// Translates TS/TSX/JS into the standard representation (ADR-0033, ADR-0048) using
// the repo's OWN TypeScript compiler (R2-15). It knows the language and nothing about
// any framework. Output: NDJSON on stdout (header, facts, end). Progress and problems
// go to stderr. M0 emits containers only: files, types, functions, blocks.
//
// Usage: node extract.mjs --root <dir> --project <tsconfig> [--project …] --schema <N>

import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";

const FACT_SCHEMA_VERSION = 1;
const EXTRACTOR = "ts@0.1.0";

function parseArgs(argv) {
  const out = { root: null, projects: [], schema: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--root") out.root = argv[++i];
    else if (a === "--project") out.projects.push(argv[++i]);
    else if (a === "--schema") out.schema = Number(argv[++i]);
    else throw new Error(`unknown argument ${a}`);
  }
  if (!out.root || out.projects.length === 0) throw new Error("need --root and at least one --project");
  if (out.schema !== FACT_SCHEMA_VERSION) throw new Error(`schema mismatch: asked ${out.schema}, extractor speaks ${FACT_SCHEMA_VERSION}`);
  return out;
}

const args = parseArgs(process.argv.slice(2));
const root = path.resolve(args.root);

// Load the repo's own TypeScript, resolved from the first project's folder (R2-15).
function loadTypeScript() {
  for (const p of args.projects) {
    const req = createRequire(path.join(root, path.dirname(p), "package.json"));
    try {
      return req("typescript");
    } catch {}
  }
  throw new Error("the repo's own `typescript` package was not found next to its tsconfig (install dependencies first)");
}
const ts = loadTypeScript();

let counts = { files: 0, nodes: 0, diagnostics: 0 };
const emit = (rec) => process.stdout.write(JSON.stringify(rec) + "\n");
const rel = (abs) => path.relative(root, abs).split(path.sep).join("/");
function diag(file, severity, message) {
  counts.diagnostics++;
  emit({ type: "diagnostic", file, severity, message });
}

emit({ type: "header", schema: FACT_SCHEMA_VERSION, extractor: EXTRACTOR, toolchain: `typescript ${ts.version}` });

// ---- Which files: every root project and the projects it references (R2-26). ----
const owner = new Map(); // abs file -> project
const seenProjects = new Set();
function addProject(configPath) {
  const abs = path.resolve(root, configPath);
  if (seenProjects.has(abs)) return;
  seenProjects.add(abs);
  const read = ts.readConfigFile(abs, ts.sys.readFile);
  if (read.error) {
    diag(rel(abs), "error", ts.flattenDiagnosticMessageText(read.error.messageText, "\n"));
    return;
  }
  const parsed = ts.parseJsonConfigFileContent(read.config, ts.sys, path.dirname(abs), undefined, abs);
  const files = [...parsed.fileNames].sort();
  for (const f of files) {
    const fa = path.resolve(f);
    if (!owner.has(fa)) owner.set(fa, rel(abs));
  }
  for (const ref of parsed.projectReferences ?? []) {
    const refPath = ts.resolveProjectReferencePath(ref);
    addProject(path.relative(root, refPath));
  }
}
for (const p of [...args.projects].sort()) addProject(p);

const isAnalyzable = (f) =>
  /\.(m|c)?(t|j)sx?$/.test(f) && !f.endsWith(".d.ts") && !f.split(path.sep).includes("node_modules") && f.startsWith(root + path.sep);

// ---- Per file: UTF-16 position -> UTF-8 byte offset (BQ-11). ----
function utf8Offsets(text) {
  const off = new Uint32Array(text.length + 1);
  let b = 0;
  for (let i = 0; i < text.length; i++) {
    off[i] = b;
    const c = text.charCodeAt(i);
    if (c < 0x80) b += 1;
    else if (c < 0x800) b += 2;
    else if (c >= 0xd800 && c <= 0xdbff) {
      // High surrogate: the pair is 4 bytes; the low half adds 0.
      b += 4;
      off[++i] = b;
      continue;
    } else b += 3;
  }
  off[text.length] = b;
  return off;
}

// ---- Naming (R3-G #1). Path segments are file, types and functions; blocks are
// containers but not path segments, so adding a block never renames a function. ----
function calleeName(call) {
  const e = call.expression;
  if (ts.isIdentifier(e)) return e.text;
  if (ts.isPropertyAccessExpression(e)) {
    const left = ts.isIdentifier(e.expression) ? e.expression.text : ts.isPropertyAccessExpression(e.expression) ? e.expression.name.text : "…";
    return `${left}.${e.name.text}`;
  }
  return "call";
}
function propName(n) {
  if (!n) return null;
  if (ts.isIdentifier(n) || ts.isPrivateIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n)) return n.text;
  if (ts.isComputedPropertyName(n)) return "[computed]";
  return null;
}
/** Name for a function-like node; `anon` = true when named by position. */
function functionName(n) {
  if ((ts.isFunctionDeclaration(n) || ts.isFunctionExpression(n)) && n.name) return { name: n.name.text, anon: false };
  if (ts.isMethodDeclaration(n) || ts.isGetAccessorDeclaration(n) || ts.isSetAccessorDeclaration(n)) {
    const base = propName(n.name) ?? "method";
    const prefix = ts.isGetAccessorDeclaration(n) ? "get " : ts.isSetAccessorDeclaration(n) ? "set " : "";
    return { name: prefix + base, anon: false };
  }
  if (ts.isConstructorDeclaration(n)) return { name: "constructor", anon: false };
  if (ts.isFunctionDeclaration(n)) return { name: "default", anon: true }; // export default function () {}
  const p = n.parent;
  if (ts.isVariableDeclaration(p) && p.initializer === n && ts.isIdentifier(p.name)) return { name: p.name.text, anon: false };
  if (ts.isPropertyAssignment(p) && p.initializer === n) return { name: propName(p.name) ?? "prop", anon: false };
  if (ts.isPropertyDeclaration(p) && p.initializer === n) return { name: propName(p.name) ?? "prop", anon: false };
  if (ts.isJsxExpression(p) && p.parent && ts.isJsxAttribute(p.parent)) return { name: p.parent.name.getText(), anon: true };
  if (ts.isCallExpression(p) && p.arguments.includes(n)) return { name: calleeName(p), anon: true };
  if (ts.isNewExpression(p) && (p.arguments ?? []).includes(n)) return { name: `new ${ts.isIdentifier(p.expression) ? p.expression.text : "…"}`, anon: true };
  if (ts.isExportAssignment(p)) return { name: "default", anon: true };
  return { name: "=>", anon: true };
}
const isFunctionLike = (n) =>
  (ts.isFunctionDeclaration(n) && n.body) ||
  ts.isFunctionExpression(n) ||
  ts.isArrowFunction(n) ||
  ((ts.isMethodDeclaration(n) || ts.isGetAccessorDeclaration(n) || ts.isSetAccessorDeclaration(n) || ts.isConstructorDeclaration(n)) && n.body);
const isType = (n) => ts.isClassDeclaration(n) || ts.isClassExpression(n) || ts.isInterfaceDeclaration(n) || ts.isEnumDeclaration(n);
function typeName(n) {
  if (n.name) return { name: n.name.text, anon: false };
  return { name: "class", anon: true };
}

// Blocks (BQ-18): branch arms, loops, try/catch/finally, ternary arms.
function blockChildren(n) {
  const out = [];
  if (ts.isIfStatement(n)) {
    out.push(["if", n.thenStatement]);
    if (n.elseStatement && !ts.isIfStatement(n.elseStatement)) out.push(["else", n.elseStatement]);
  } else if (ts.isForStatement(n) || ts.isForInStatement(n) || ts.isForOfStatement(n)) out.push(["for", n.statement]);
  else if (ts.isWhileStatement(n)) out.push(["while", n.statement]);
  else if (ts.isDoStatement(n)) out.push(["do", n.statement]);
  else if (ts.isTryStatement(n)) {
    out.push(["try", n.tryBlock]);
    if (n.catchClause) out.push(["catch", n.catchClause.block]);
    if (n.finallyBlock) out.push(["finally", n.finallyBlock]);
  } else if (ts.isConditionalExpression(n)) {
    out.push(["?", n.whenTrue]);
    out.push([":", n.whenFalse]);
  }
  return out;
}

for (const abs of [...owner.keys()].sort()) {
  if (!isAnalyzable(abs)) continue;
  let bytes;
  try {
    bytes = readFileSync(abs);
  } catch (e) {
    diag(rel(abs), "error", `cannot read: ${e.message}`);
    continue;
  }
  const text = bytes.toString("utf8");
  const file = rel(abs);
  emit({ type: "file", path: file, lang: "ts", sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.length });
  counts.files++;

  const kind = /x$/.test(abs) ? ts.ScriptKind.TSX : /\.(m|c)?js$/.test(abs) ? ts.ScriptKind.JS : ts.ScriptKind.TS;
  const sf = ts.createSourceFile(abs, text, ts.ScriptTarget.Latest, true, kind);
  const u8 = utf8Offsets(text);
  const span = (node, start = node.getStart(sf)) => {
    const s = sf.getLineAndCharacterOfPosition(start);
    const e = sf.getLineAndCharacterOfPosition(node.end);
    return { byte_start: u8[start], byte_end: u8[node.end], line_start: s.line + 1, line_end: e.line + 1 };
  };
  const node = (id, parent, kind, name, sp) => {
    emit({ type: "node", id, parent, kind, name, span: sp });
    counts.nodes++;
  };
  node(file, null, "file", path.basename(file), { byte_start: 0, byte_end: bytes.length, line_start: 1, line_end: sf.getLineAndCharacterOfPosition(text.length).line + 1 });

  // scope = the nearest path segment (file/type/function); parent = nearest container (may be a block).
  const ordinals = new Map(); // scopeId -> Map(name -> count)
  function nextName(scopeId, name, anon) {
    let m = ordinals.get(scopeId);
    if (!m) ordinals.set(scopeId, (m = new Map()));
    const k = (m.get(name) ?? 0) + 1;
    m.set(name, k);
    return anon || k > 1 ? `${name}#${k}` : name;
  }

  function visit(n, scopeId, parentId) {
    let nextScope = scopeId;
    let nextParent = parentId;
    if (isFunctionLike(n) || isType(n)) {
      const { name, anon } = isType(n) ? typeName(n) : functionName(n);
      const seg = nextName(scopeId, name, anon);
      const id = `${scopeId} › ${seg}`;
      node(id, parentId, isType(n) ? "type" : "function", seg, span(n));
      nextScope = id;
      nextParent = id;
    }
    const blockId = (label) => `${nextScope} › ${nextName(nextScope + "\u0000blocks", label, true)}`;
    if (ts.isCaseClause(n) || ts.isDefaultClause(n)) {
      // The clause itself is the block: its label expression belongs to the
      // enclosing container, its statements to the block.
      const label = ts.isCaseClause(n) ? "case" : "default";
      const id = blockId(label);
      node(id, nextParent, "block", label, span(n));
      if (ts.isCaseClause(n)) visit(n.expression, nextScope, nextParent);
      for (const s of n.statements) visit(s, nextScope, id);
      return;
    }
    const blocks = blockChildren(n);
    if (blocks.length) {
      const bodies = new Set(blocks.map(([, b]) => b));
      ts.forEachChild(n, (c) => {
        if (!bodies.has(c)) visit(c, nextScope, nextParent);
      });
      for (const [label, body] of blocks) {
        const id = blockId(label);
        node(id, nextParent, "block", label, span(body));
        // Visit the body node itself, so a function that IS the arm (`c ? () => a : b`) is found.
        visit(body, nextScope, id);
      }
      return;
    }
    ts.forEachChild(n, (c) => visit(c, nextScope, nextParent));
  }
  ts.forEachChild(sf, (c) => visit(c, file, file));
}

emit({ type: "end", files: counts.files, nodes: counts.nodes, diagnostics: counts.diagnostics });
