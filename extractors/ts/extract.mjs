// Insight TypeScript/JavaScript extractor (ADR-0034, R2-28).
//
// Translates TS/TSX/JS into the standard representation (ADR-0033, ADR-0048) using
// the repo's OWN TypeScript compiler (R2-15). It knows the language and nothing about
// any framework. Output: NDJSON on stdout (header, facts, end). Progress and problems
// go to stderr. Pass 1: containers (files, types, functions, blocks). Pass 2: calls,
// one-hop function-value flow, handoffs to external code, and string literals.
//
// Usage: node extract.mjs --root <dir> --project <tsconfig> [--project …] --schema <N>

import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";

const FACT_SCHEMA_VERSION = 4;
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

let counts = { files: 0, nodes: 0, edges: 0, literals: 0, diagnostics: 0 };
const emit = (rec) => process.stdout.write(JSON.stringify(rec) + "\n");
const rel = (abs) => path.relative(root, abs).split(path.sep).join("/");
function diag(file, severity, message) {
  counts.diagnostics++;
  emit({ type: "diagnostic", file, severity, message });
}

emit({ type: "header", schema: FACT_SCHEMA_VERSION, extractor: EXTRACTOR, toolchain: `typescript ${ts.version}` });

// ---- Which files: every root project and the projects it references (R2-26).
// Each file belongs to the first project that includes it, so IDs stay unique. ----
const owner = new Map(); // abs file -> project config abs path
const configs = new Map(); // project config abs path -> parsed config
function addProject(configPath) {
  const abs = path.resolve(root, configPath);
  if (configs.has(abs)) return;
  const read = ts.readConfigFile(abs, ts.sys.readFile);
  if (read.error) {
    configs.set(abs, null);
    diag(rel(abs), "error", ts.flattenDiagnosticMessageText(read.error.messageText, "\n"));
    return;
  }
  const parsed = ts.parseJsonConfigFileContent(read.config, ts.sys, path.dirname(abs), undefined, abs);
  configs.set(abs, parsed);
  for (const f of [...parsed.fileNames].sort()) {
    const fa = path.resolve(f);
    if (!owner.has(fa)) owner.set(fa, abs);
  }
  for (const ref of parsed.projectReferences ?? []) {
    addProject(path.relative(root, ts.resolveProjectReferencePath(ref)));
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
/** The state kind a declaration introduces (`var` / `static` / `field`), or null. */
function stateKind(n) {
  const named = n.name && (ts.isIdentifier(n.name) || ts.isPrivateIdentifier(n.name));
  if (!named) return null;
  const inFunction = () => !!ts.findAncestor(n.parent, isFunctionLike);
  if (ts.isVariableDeclaration(n) || ts.isBindingElement(n)) return inFunction() ? "var" : "static";
  if (ts.isParameter(n)) return isFunctionLike(n.parent) ? "var" : null;
  if (ts.isPropertyDeclaration(n)) return n.modifiers?.some((m) => m.kind === ts.SyntaxKind.StaticKeyword) ? "static" : "field";
  if (ts.isPropertySignature(n)) return "field";
  return null;
}
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

// ---- Programs: one per project, so the checker sees each project's own options. ----
const programs = new Map(); // config abs -> { program, checker }
function programFor(cfg) {
  if (!programs.has(cfg)) {
    const parsed = configs.get(cfg);
    const program = ts.createProgram({
      rootNames: parsed.fileNames,
      options: { ...parsed.options, noEmit: true },
      projectReferences: parsed.projectReferences,
    });
    programs.set(cfg, { program, checker: program.getTypeChecker() });
  }
  return programs.get(cfg);
}

// Per analyzed file: its context (for spans and receipts), keyed by repo-relative path.
const ctxByFile = new Map(); // rel -> { sf, file, u8, shift }
// Containers by source position, valid across programs: `${file}@${start}@${end}@${kind}`.
const fnIds = new Map();
const typeIds = new Map();
const blockIds = new Map();
const stateIds = new Map(); // posKey(declaration) -> state node id
const usedIds = new Set();
const posKey = (n) => {
  const sf = n.getSourceFile();
  return `${rel(sf.fileName)}@${n.getStart(sf)}@${n.end}@${n.kind}`;
};

// ================================ pass 1: files and containers ================================
const fileOrder = [...owner.keys()].filter(isAnalyzable).sort();
for (const abs of fileOrder) {
  const cfg = owner.get(abs);
  if (!configs.get(cfg)) continue;
  let bytes;
  try {
    bytes = readFileSync(abs);
  } catch (e) {
    diag(rel(abs), "error", `cannot read: ${e.message}`);
    continue;
  }
  const text = bytes.toString("utf8");
  const file = rel(abs);
  const { program } = programFor(cfg);
  const sf = program.getSourceFile(abs);
  if (!sf) {
    diag(file, "error", "the project's program did not load this file");
    continue;
  }
  // TypeScript strips a UTF-8 BOM; spans must still index the file's real bytes.
  let shift = 0;
  if (sf.text !== text) {
    if (text === "﻿" + sf.text) shift = 3;
    else {
      diag(file, "error", "the compiler's text differs from the file on disk");
      continue;
    }
  }
  emit({ type: "file", path: file, lang: "ts", sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.length });
  counts.files++;
  const u8 = utf8Offsets(sf.text);
  ctxByFile.set(file, { sf, file, u8, shift });
  const span = (node, start = node.getStart(sf)) => {
    const s = sf.getLineAndCharacterOfPosition(start);
    const e = sf.getLineAndCharacterOfPosition(node.end);
    return { byte_start: u8[start] + shift, byte_end: u8[node.end] + shift, line_start: s.line + 1, line_end: e.line + 1 };
  };
  const node = (id, parent, kind, name, sp, anonymous = false) => {
    emit({ type: "node", id, parent, kind, name, span: sp, anonymous });
    usedIds.add(id);
    counts.nodes++;
  };
  node(file, null, "file", path.basename(file), { byte_start: 0, byte_end: bytes.length, line_start: 1, line_end: sf.getLineAndCharacterOfPosition(sf.text.length).line + 1 });

  // scope = the nearest path segment (file/type/function); parent = nearest container (may be a block).
  const ordinals = new Map(); // scopeId -> Map(name -> count)
  function nextName(scopeId, name, anon) {
    let m = ordinals.get(scopeId);
    if (!m) ordinals.set(scopeId, (m = new Map()));
    const k = (m.get(name) ?? 0) + 1;
    m.set(name, k);
    return anon || k > 1 ? `${name}#${k}` : name;
  }

  // State (SCHEMA.md §2): variables, parameters, class fields and interface/type-literal
  // fields, with their own ordinals; emitted after the file's containers (below).
  const pendingState = [];
  function visit(n, scopeId, parentId) {
    visitContainers(n, scopeId, parentId);
    const kind = stateKind(n);
    if (kind) pendingState.push([n, scopeId, parentId, kind]);
  }

  function visitContainers(n, scopeId, parentId) {
    let nextScope = scopeId;
    let nextParent = parentId;
    if (isFunctionLike(n) || isType(n)) {
      const { name, anon } = isType(n) ? typeName(n) : functionName(n);
      const seg = nextName(scopeId, name, anon);
      const id = `${scopeId} › ${seg}`;
      node(id, parentId, isType(n) ? "type" : "function", seg, span(n), anon);
      (isType(n) ? typeIds : fnIds).set(posKey(n), id);
      nextScope = id;
      nextParent = id;
    }
    const blockId = (label) => `${nextScope} › ${nextName(nextScope + "\u0000blocks", label, true)}`;
    if (ts.isCaseClause(n) || ts.isDefaultClause(n)) {
      // The clause itself is the block: its label expression belongs to the
      // enclosing container, its statements to the block.
      const label = ts.isCaseClause(n) ? "case" : "default";
      const id = blockId(label);
      node(id, nextParent, "block", label, span(n), true);
      blockIds.set(posKey(n), id);
      if (ts.isCaseClause(n)) visit(n.expression, nextScope, nextParent);
      for (const s of n.statements) visit(s, nextScope, id);
      return;
    }
    const blocks = blockChildren(n);
    if (blocks.length) {
      const bodies = new Set(blocks.map(([, b]) => b));
      ts.forEachChild(n, (c) => {
        if (bodies.has(c)) return;
        // A catch clause's block is a body; only its variable belongs to the enclosing container.
        if (ts.isCatchClause(c)) {
          if (c.variableDeclaration) visit(c.variableDeclaration, nextScope, nextParent);
          return;
        }
        visit(c, nextScope, nextParent);
      });
      for (const [label, body] of blocks) {
        const id = blockId(label);
        node(id, nextParent, "block", label, span(body), true);
        blockIds.set(posKey(body), id);
        // Visit the body node itself, so a function that IS the arm (`c ? () => a : b`) is found.
        visit(body, nextScope, id);
      }
      return;
    }
    ts.forEachChild(n, (c) => visit(c, nextScope, nextParent));
  }
  ts.forEachChild(sf, (c) => visit(c, file, file));
  // State nodes last: every function and type in the file already has its name, so a
  // variable can never take (or shift) a function's id.
  for (const [n, scopeId, parentId, kind] of pendingState) {
    const name = n.name.text;
    let seg = nextName(scopeId + "\u0000state", name, false);
    while (usedIds.has(`${scopeId} › ${seg}`)) seg = nextName(scopeId + "\u0000state", name, true);
    const id = `${scopeId} › ${seg}`;
    node(id, parentId, kind, seg, span(n), false);
    stateIds.set(posKey(n), id);
  }
}

// ================================ pass 2: edges and literals ================================
// Generic TS/JS semantics only (ADR-0048): calls resolved by the checker, one-hop
// function-value flow (BQ-15), handoffs to external code (BQ-16 as amended), literals (BQ-22).
const externals = new Set();
let groupSeq = 0;

function receipt(n, role, container) {
  const sf = n.getSourceFile();
  const ctx = ctxByFile.get(rel(sf.fileName));
  const start = n.getStart(sf);
  const s = sf.getLineAndCharacterOfPosition(start);
  const e = sf.getLineAndCharacterOfPosition(n.end);
  const { u8, shift } = ctx;
  const col = (pos, line) => u8[pos] - u8[sf.getPositionOfLineAndCharacter(line, 0)] + 1;
  return {
    role,
    file: ctx.file,
    byte_start: u8[start] + shift,
    byte_end: u8[n.end] + shift,
    line: s.line + 1,
    col: col(start, s.line),
    end_line: e.line + 1,
    end_col: col(n.end, e.line),
    container,
  };
}
function edge(kind, subkind, from, to, resolution, group, receipts) {
  emit({ type: "edge", kind, subkind, from, to, resolution, group, receipts });
  counts.edges++;
}

for (const [cfg, parsed] of [...configs].sort(([a], [b]) => (a < b ? -1 : 1))) {
  if (!parsed) continue;
  const owned = fileOrder.filter((f) => owner.get(f) === cfg && ctxByFile.has(rel(f)));
  if (owned.length === 0) continue;
  const { program, checker } = programFor(cfg);
  const sfs = owned.map((f) => program.getSourceFile(f));

  const strip = (e) => {
    while (e && (ts.isParenthesizedExpression(e) || ts.isAsExpression(e) || ts.isNonNullExpression(e) || ts.isSatisfiesExpression?.(e) || ts.isTypeAssertionExpression(e))) e = e.expression;
    return e;
  };
  const symAt = (e) => {
    try {
      if (ts.isPropertyAccessExpression(e)) return checker.getSymbolAtLocation(e.name);
      if (ts.isShorthandPropertyAssignment(e)) return checker.getShorthandAssignmentValueSymbol(e);
      return checker.getSymbolAtLocation(e);
    } catch {
      return undefined;
    }
  };
  // The declaration a symbol stands for. A destructured parameter field maps to the
  // property it destructures, so both sides of a prop meet at one declaration.
  function declOf(sym) {
    if (!sym) return undefined;
    if (sym.flags & ts.SymbolFlags.Alias) {
      try {
        sym = checker.getAliasedSymbol(sym);
      } catch {}
    }
    let d = sym.valueDeclaration ?? sym.declarations?.[0];
    if (d && ts.isBindingElement(d) && ts.isObjectBindingPattern(d.parent)) {
      const name = d.propertyName ? propName(d.propertyName) : ts.isIdentifier(d.name) ? d.name.text : null;
      const p = name && checker.getPropertyOfType(checker.getTypeAtLocation(d.parent), name);
      const pd = p && (p.valueDeclaration ?? p.declarations?.[0]);
      if (pd) d = pd;
    }
    return d;
  }
  const isProjectDecl = (d) => ctxByFile.has(rel(d.getSourceFile().fileName)) && !d.getSourceFile().isDeclarationFile;
  const fnIdOf = (n) => (n && isProjectDecl(n) ? fnIds.get(posKey(n)) : undefined);

  // Functions a declaration directly is or holds (a function, or a variable/property initialized with one).
  function fnsOfDecl(d) {
    if (!d || !isProjectDecl(d)) return [];
    const own = fnIdOf(d);
    if (own) return [own];
    if ((ts.isVariableDeclaration(d) || ts.isPropertyAssignment(d) || ts.isPropertyDeclaration(d)) && d.initializer) {
      const id = fnIdOf(strip(d.initializer));
      return id ? [id] : [];
    }
    return [];
  }
  // Function literals a function returns, one level (BQ-15: curried handlers).
  function returnedFns(fnNode) {
    if (!fnNode || !fnNode.body) return [];
    if (!ts.isBlock(fnNode.body)) {
      const id = fnIdOf(strip(fnNode.body));
      return id ? [id] : [];
    }
    const out = [];
    const walk = (n) => {
      if (n !== fnNode.body && isFunctionLike(n)) return;
      if (ts.isReturnStatement(n) && n.expression) {
        const id = fnIdOf(strip(n.expression));
        if (id) out.push(id);
      }
      ts.forEachChild(n, walk);
    };
    walk(fnNode.body);
    return out;
  }
  // Function values an expression evaluates to (no flow beyond declarations: one hop).
  function fnValues(expr, depth = 0) {
    const e = strip(expr);
    if (!e) return [];
    const own = fnIdOf(e);
    if (own) return [own];
    if (ts.isConditionalExpression(e)) return [...fnValues(e.whenTrue, depth), ...fnValues(e.whenFalse, depth)];
    if (ts.isBinaryExpression(e) && [ts.SyntaxKind.BarBarToken, ts.SyntaxKind.QuestionQuestionToken].includes(e.operatorToken.kind))
      return [...fnValues(e.left, depth), ...fnValues(e.right, depth)];
    if (ts.isBinaryExpression(e) && e.operatorToken.kind === ts.SyntaxKind.AmpersandAmpersandToken) return fnValues(e.right, depth);
    if (ts.isIdentifier(e) || ts.isPropertyAccessExpression(e)) return fnsOfDecl(declOf(symAt(e)));
    if (ts.isCallExpression(e) && depth === 0) {
      const d = declOf(symAt(strip(e.expression)));
      if (!d || !isProjectDecl(d)) return [];
      const fnNode = fnIdOf(d) ? d : (ts.isVariableDeclaration(d) || ts.isPropertyAssignment(d)) && d.initializer ? strip(d.initializer) : null;
      return fnNode && fnIdOf(fnNode) ? returnedFns(fnNode) : [];
    }
    return [];
  }
  const nameText = (n) => (n && (ts.isIdentifier(n) || ts.isStringLiteral(n) || ts.isNumericLiteral(n) || ts.isPrivateIdentifier(n)) ? n.text : null);
  const propOfType = (type, name) => {
    if (!type || !name) return undefined;
    const p = checker.getPropertyOfType(type, name) ?? checker.getPropertyOfType(checker.getApparentType(type), name);
    return p ? p.valueDeclaration ?? p.declarations?.[0] : undefined;
  };

  // ---- 2a: every function value written to a declaration (BQ-15): variables,
  // assignments, object-literal properties and JSX attributes (via contextual
  // typing), and call arguments (via the resolved signature's parameters). ----
  const writes = new Map(); // declaration node -> [{ ids, site }]
  const addWrite = (d, valueExpr, site) => {
    if (!d) return;
    const ids = fnValues(valueExpr);
    if (ids.length === 0) return;
    if (!writes.has(d)) writes.set(d, []);
    writes.get(d).push({ ids, site });
  };
  for (const sf of sfs) {
    const walk = (n) => {
      if (ts.isVariableDeclaration(n) && n.initializer && ts.isIdentifier(n.name)) addWrite(n, n.initializer, n.initializer);
      else if (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
        const left = strip(n.left);
        if (ts.isIdentifier(left) || ts.isPropertyAccessExpression(left)) addWrite(declOf(symAt(left)), n.right, n.right);
      } else if ((ts.isPropertyAssignment(n) || ts.isShorthandPropertyAssignment(n) || ts.isMethodDeclaration(n)) && ts.isObjectLiteralExpression(n.parent)) {
        let ct;
        try {
          ct = checker.getContextualType(n.parent);
        } catch {}
        const target = propOfType(ct, propName(n.name)) ?? n;
        const value = ts.isPropertyAssignment(n) ? n.initializer : ts.isShorthandPropertyAssignment(n) ? n.name : n;
        addWrite(target, value, value);
      } else if (ts.isJsxAttribute(n) && n.initializer && ts.isJsxExpression(n.initializer) && n.initializer.expression) {
        let ct;
        try {
          ct = checker.getContextualType(n.parent);
        } catch {}
        addWrite(propOfType(ct, nameText(n.name) ?? n.name.getText()), n.initializer.expression, n.initializer.expression);
      } else if ((ts.isCallExpression(n) || ts.isNewExpression(n)) && n.arguments?.length) {
        let sig;
        try {
          sig = checker.getResolvedSignature(n);
        } catch {}
        const sd = sig?.declaration;
        if (sd && !ts.isJSDocSignature?.(sd) && isProjectDecl(sd) && sd.parameters) {
          n.arguments.forEach((a, i) => {
            const p = sd.parameters[Math.min(i, sd.parameters.length - 1)];
            if (p && (i < sd.parameters.length || p.dotDotDotToken) && ts.isIdentifier(p.name)) addWrite(p, a, a);
          });
        }
      }
      ts.forEachChild(n, walk);
    };
    walk(sf);
  }

  // ---- 2b: calls, handoffs, JSX, literals ----
  function externalId(d, sym) {
    const f = d.getSourceFile().fileName.split(path.sep).join("/");
    const nm = f.lastIndexOf("/node_modules/");
    let module;
    if (nm >= 0) {
      const parts = f.slice(nm + 14).split("/");
      module = parts[0].startsWith("@") ? `${parts[0]}/${parts[1]}` : parts[0];
    } else if (/\/typescript\/lib\/lib\.[^/]*\.d\.ts$/.test(f)) module = "lib";
    else module = rel(f);
    const name = sym.getName();
    const parent = sym.parent && !sym.parent.getName().startsWith('"') ? `${sym.parent.getName()}.` : "";
    const id = `ext:ts:${module}:${parent}${name}`;
    if (!externals.has(id)) {
      externals.add(id);
      emit({ type: "node", id, parent: null, kind: "external", name: `${parent}${name}`, span: null, anonymous: false });
      counts.nodes++;
    }
    return id;
  }
  // Resolves a callee to { kind: "exact", to } | { kind: "external", to } | { kind: "indirect", d, optional } | null.
  function resolveCallee(calleeExpr, isNew) {
    const e = strip(calleeExpr);
    const own = fnIdOf(e);
    if (own) return { kind: "exact", to: own };
    let sym = symAt(e);
    if (!sym) return null;
    if (sym.flags & ts.SymbolFlags.Alias) {
      try {
        sym = checker.getAliasedSymbol(sym);
      } catch {}
    }
    const d = declOf(sym);
    if (!d) return null;
    if (!isProjectDecl(d)) return { kind: "external", to: externalId(d, sym) };
    if (isNew && (ts.isClassDeclaration(d) || ts.isClassExpression(d))) {
      const ctor = d.members.find((m) => ts.isConstructorDeclaration(m) && m.body);
      return ctor ? { kind: "exact", to: fnIdOf(ctor) } : null;
    }
    const ids = fnsOfDecl(d);
    if (ids.length) return { kind: "exact", to: ids[0] };
    return { kind: "indirect", d, optional: !!d.questionToken };
  }

  // ---- Data flow (SCHEMA.md §5): by declaration, not by program point. ----
  const isStateDecl = (d) => d && (ts.isVariableDeclaration(d) || ts.isBindingElement(d) || ts.isParameter(d) || ts.isPropertyDeclaration(d) || ts.isPropertySignature(d));
  function dataSym(e) {
    let sym = symAt(e);
    if (sym && sym.flags & ts.SymbolFlags.Alias) {
      try {
        sym = checker.getAliasedSymbol(sym);
      } catch {}
    }
    return sym;
  }
  const declOfSym = (sym) => sym && (sym.valueDeclaration ?? sym.declarations?.[0]);
  // The state node a declaration is (or its external leaf).
  function stateOf(d, sym) {
    if (!isStateDecl(d)) return undefined;
    if (isProjectDecl(d)) return stateIds.get(posKey(d));
    return sym ? externalId(d, sym) : undefined;
  }
  // The state an lvalue names: `x`, `o.p`, `a[i]` (the array).
  function stateAt(e) {
    e = strip(e);
    if (!e) return undefined;
    if (ts.isElementAccessExpression(e)) return stateAt(e.expression);
    if (!(ts.isIdentifier(e) || ts.isPropertyAccessExpression(e))) return undefined;
    const sym = dataSym(e);
    return stateOf(declOfSym(sym), sym);
  }
  function propState(type, name) {
    if (!type || !name) return undefined;
    let p;
    try {
      p = checker.getPropertyOfType(type, name) ?? checker.getPropertyOfType(checker.getApparentType(type), name);
    } catch {}
    return p ? stateOf(declOfSym(p), p) : undefined;
  }
  // Where an expression's value comes from: ["state"|"fn"|"result", id] (arguments of a
  // call bind to its parameters; they do not flow into its result).
  function sources(expr, out = []) {
    const e = strip(expr);
    if (!e) return out;
    const own = fnIdOf(e);
    if (own) {
      out.push(["fn", own]);
      return out;
    }
    if (ts.isIdentifier(e) || ts.isPropertyAccessExpression(e)) {
      const sym = dataSym(e);
      const d = declOfSym(sym);
      const f = d && fnIdOf(d);
      if (f) out.push(["fn", f]);
      else {
        const s = stateOf(d, sym);
        if (s) out.push(["state", s]);
      }
      return out;
    }
    if (ts.isCallExpression(e) || ts.isNewExpression(e)) {
      const r = resolveCallee(e.expression, ts.isNewExpression(e));
      if (r && (r.kind === "exact" || r.kind === "external") && r.to) out.push(["result", r.to]);
      return out;
    }
    if (ts.isElementAccessExpression(e)) return sources(e.expression, out);
    if (ts.isConditionalExpression(e)) {
      sources(e.whenTrue, out);
      sources(e.whenFalse, out);
      return out;
    }
    if (ts.isTypeOfExpression(e) || ts.isStringLiteralLike(e) || ts.isNumericLiteral(e) || isFunctionLike(e)) return out;
    ts.forEachChild(e, (c) => {
      if (!ts.isTypeNode(c)) sources(c, out);
    });
    return out;
  }
  // Value edges from `srcs` into a state (`assign`/`result`), a parameter (`binds`), or a
  // function returning them (`return`/`result`).
  function flowInto(srcs, to, into, rc) {
    for (const [k, id] of srcs) {
      if (k === "result") edge("value_flow", "result", id, to, "exact", null, [rc]);
      else if (into === "fn") {
        if (k === "state") edge("value_flow", "return", id, to, "exact", null, [rc]);
      } else if (into === "param") edge("binds", "value", id, to, "exact", null, [rc]);
      else edge("value_flow", "assign", id, to, "exact", null, [rc]);
    }
  }
  const isAssignOp = (k) => k >= ts.SyntaxKind.FirstAssignment && k <= ts.SyntaxKind.LastAssignment;

  for (const sf of sfs) {
    const file = rel(sf.fileName);
    const fnStack = [file];
    const ctrStack = [file];
    const writePos = new Set(); // expressions in a write position: not reads
    const markWrite = (e) => {
      e = strip(e);
      if (e && ts.isElementAccessExpression(e)) return markWrite(e.expression);
      if (e) writePos.add(e);
    };
    const walk = (n) => {
      const fid = fnIds.get(posKey(n));
      const bid = blockIds.get(posKey(n));
      if (bid) ctrStack.push(bid);
      if (fid) {
        fnStack.push(fid);
        ctrStack.push(fid);
      }
      const from = fnStack[fnStack.length - 1];
      const ctr = ctrStack[ctrStack.length - 1];

      // ---- data: writes, value edges, reads ----
      if ((ts.isVariableDeclaration(n) || ts.isBindingElement(n)) && ts.isIdentifier(n.name)) {
        writePos.add(n.name);
        const target = stateIds.get(posKey(n));
        if (target) {
          const srcs = n.initializer ? sources(n.initializer) : [];
          if (ts.isBindingElement(n) && ts.isObjectBindingPattern(n.parent)) {
            // Destructuring reads the property it names.
            let pt;
            try {
              pt = checker.getTypeAtLocation(n.parent);
            } catch {}
            const ps = propState(pt, n.propertyName ? propName(n.propertyName) : n.name.text);
            if (ps) srcs.push(["state", ps]);
          }
          if (n.initializer || srcs.length) {
            const rc = receipt(n, "write", ctr);
            edge("write", null, from, target, "exact", null, [rc]);
            flowInto(srcs, target, "state", rc);
          }
        }
      } else if (ts.isBinaryExpression(n) && isAssignOp(n.operatorToken.kind)) {
        const target = stateAt(n.left);
        if (target) {
          markWrite(n.left);
          const rc = receipt(n, "write", ctr);
          edge("write", null, from, target, "exact", null, [rc]);
          if (n.operatorToken.kind !== ts.SyntaxKind.EqualsToken) edge("read", null, from, target, "exact", null, [rc]);
          flowInto(sources(n.right), target, "state", rc);
        }
      } else if ((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n)) && [ts.SyntaxKind.PlusPlusToken, ts.SyntaxKind.MinusMinusToken].includes(n.operator)) {
        const target = stateAt(n.operand);
        if (target) {
          markWrite(n.operand);
          const rc = receipt(n, "write", ctr);
          edge("read", null, from, target, "exact", null, [rc]);
          edge("write", null, from, target, "exact", null, [rc]);
        }
      } else if ((ts.isPropertyAssignment(n) || ts.isShorthandPropertyAssignment(n)) && ts.isObjectLiteralExpression(n.parent)) {
        let ct;
        try {
          ct = checker.getContextualType(n.parent);
        } catch {}
        const target = propState(ct, propName(n.name));
        if (target) flowInto(sources(ts.isPropertyAssignment(n) ? n.initializer : n.name), target, "state", receipt(n, "write", ctr));
      } else if (ts.isJsxAttribute(n) && n.initializer && ts.isJsxExpression(n.initializer) && n.initializer.expression) {
        let ct;
        try {
          ct = checker.getContextualType(n.parent);
        } catch {}
        const target = propState(ct, nameText(n.name) ?? n.name.getText());
        if (target) flowInto(sources(n.initializer.expression), target, "param", receipt(n, "arg", ctr));
      } else if (ts.isReturnStatement(n) && n.expression && from !== file) {
        flowInto(sources(n.expression), from, "fn", receipt(n, "return", ctr));
      } else if ((ts.isIdentifier(n) || ts.isPropertyAccessExpression(n)) && !writePos.has(n) && !(n.parent && n.parent.name === n)) {
        const s = stateAt(n);
        if (s) edge("read", null, from, s, "exact", null, [receipt(n, "read", ctr)]);
      }
      if (fid && ts.isArrowFunction(n) && !ts.isBlock(n.body)) flowInto(sources(n.body), fid, "fn", receipt(n.body, "return", fid));

      if (ts.isCallExpression(n) || ts.isNewExpression(n)) {
        const r = resolveCallee(n.expression, ts.isNewExpression(n));
        const call = receipt(n, "call", ctr);
        // Arguments bind to the resolved signature's parameters, or to the external callee.
        let sig;
        try {
          sig = checker.getResolvedSignature(n);
        } catch {}
        const sd = sig?.declaration;
        (n.arguments ?? []).forEach((a, i) => {
          const rc = receipt(a, "arg", ctr);
          if (sd && !ts.isJSDocSignature?.(sd) && isProjectDecl(sd) && sd.parameters) {
            const p = sd.parameters[Math.min(i, sd.parameters.length - 1)];
            const ps = p && (i < sd.parameters.length || p.dotDotDotToken) ? stateIds.get(posKey(p)) : undefined;
            if (ps) flowInto(sources(a), ps, "param", rc);
          } else if (r && r.kind === "external") {
            flowInto(sources(a).filter(([k]) => k !== "fn"), r.to, "param", rc);
          }
        });
        if (!r) edge("call", null, from, null, "unresolved", null, [call]);
        else if (r.kind === "exact" || r.kind === "external") {
          edge("call", null, from, r.to, "exact", null, [call]);
          if (r.kind === "external")
            for (const a of n.arguments ?? []) for (const id of fnValues(a)) edge("value_flow", "handoff", from, id, "exact", null, [receipt(a, "arg", ctr)]);
        } else {
          const group = ++groupSeq;
          const seen = new Set();
          for (const w of writes.get(r.d) ?? [])
            for (const id of w.ids) {
              if (seen.has(id)) continue;
              seen.add(id);
              edge("call", null, from, id, "possible", group, [call, receipt(w.site, "value", undefined)]);
            }
          if (n.questionDotToken || r.optional) edge("call", null, from, null, "possible", group, [call]);
          else if (seen.size === 0) edge("call", null, from, null, "unresolved", group, [call]);
        }
      } else if (ts.isJsxOpeningElement(n) || ts.isJsxSelfClosingElement(n)) {
        // JSX hands the component (or, for an intrinsic tag, its function-valued
        // attributes) to the external JSX factory (R4-B).
        const intrinsic = ts.isIdentifier(n.tagName) && /^[a-z]/.test(n.tagName.text);
        if (intrinsic) {
          for (const a of n.attributes.properties)
            if (ts.isJsxAttribute(a) && a.initializer && ts.isJsxExpression(a.initializer) && a.initializer.expression)
              for (const id of fnValues(a.initializer.expression)) edge("value_flow", "handoff", from, id, "exact", null, [receipt(a, "arg", ctr)]);
        } else {
          for (const id of fnsOfDecl(declOf(symAt(n.tagName)))) edge("value_flow", "handoff", from, id, "exact", null, [receipt(n.tagName, "jsx", ctr)]);
        }
      } else if (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n)) {
        emit({ type: "literal", value: n.text, from, receipt: receipt(n, "literal", ctr) });
        counts.literals++;
      } else if (ts.isNumericLiteral(n)) {
        // Integers only, in canonical decimal (the scanner already normalizes 0x1F to "31").
        const v = Number(n.text);
        if (Number.isSafeInteger(v)) {
          emit({ type: "literal", value: String(v), number: true, from, receipt: receipt(n, "literal", ctr) });
          counts.literals++;
        }
      }

      // A function returning function literals (curried handlers, BQ-15 / R3-G #5).
      if (fid) for (const id of returnedFns(n)) edge("value_flow", "returns", fid, id, "exact", null, [receipt(n, "returns", ctr)]);

      ts.forEachChild(n, walk);
      if (fid) {
        fnStack.pop();
        ctrStack.pop();
      }
      if (bid) ctrStack.pop();
    };
    walk(sf);
  }
}

emit({ type: "end", files: counts.files, nodes: counts.nodes, edges: counts.edges, literals: counts.literals, diagnostics: counts.diagnostics });
