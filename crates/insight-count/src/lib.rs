//! Independent function counter (R3-11).
//!
//! A completely separate parser (tree-sitter) counts the functions written in
//! the source, so an extractor that silently drops functions can't hide it.
//! The counting rules are in `docs/SCHEMA.md` ("Counting rules for the M0 check"):
//! a function is any function-like syntax **with a body**, named or anonymous.
//! Signatures without bodies are excluded. Rust macro-call arguments are
//! re-parsed (as call arguments, else as block statements) and counted, since
//! that code is written in the source; if neither parse is clean, nothing
//! inside is counted. `macro_rules!` bodies are templates and are not counted.
//! Functions a macro generates from its own template are excluded (R3-11).

use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    TypeScript,
    Tsx,
    Rust,
    C,
    Cpp,
}

impl Lang {
    /// Picks the grammar from a file extension; `None` if not counted.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?;
        if path.ends_with(".d.ts") {
            return None;
        }
        match ext {
            "ts" | "mts" | "cts" | "js" | "mjs" | "cjs" => Some(Lang::TypeScript),
            "tsx" | "jsx" => Some(Lang::Tsx),
            "rs" => Some(Lang::Rust),
            "c" => Some(Lang::C),
            // Headers are parsed as C++ (a near-superset); a C-only header that uses a C++
            // keyword as a name shows up as a mismatch, never as a silent pass.
            "cc" | "cpp" | "cxx" | "c++" | "h" | "hh" | "hpp" | "hxx" | "h++" | "ipp" | "tpp" | "inl" => Some(Lang::Cpp),
            _ => None,
        }
    }

    fn language(self) -> Language {
        match self {
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::C => tree_sitter_c::LANGUAGE.into(),
            Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }
}

/// Counts function-like syntax with a body in `source`.
pub fn count_functions(lang: Lang, source: &str) -> anyhow::Result<usize> {
    count_functions_outside(lang, source, &[])
}

/// Counts functions that start outside every `[start, end)` byte range in `skip`:
/// the regions the build configuration excluded (inactive `#if` blocks). Which
/// regions are inactive is a preprocessor fact the extractor records; whether a
/// function is written there is still counted independently.
pub fn count_functions_outside(lang: Lang, source: &str, skip: &[(usize, usize)]) -> anyhow::Result<usize> {
    Ok(function_ends(lang, source, skip)?.len())
}

/// The end byte of each counted function, for reporting where a count differs.
pub fn function_ends(lang: Lang, source: &str, skip: &[(usize, usize)]) -> anyhow::Result<Vec<usize>> {
    Ok(function_spans(lang, source, skip)?.into_iter().map(|(_, e)| e).collect())
}

/// The byte span `(start, end)` of each counted function outside `skip`. (Functions found
/// by re-parsing macro arguments span the macro's argument list.)
pub fn function_spans(lang: Lang, source: &str, skip: &[(usize, usize)]) -> anyhow::Result<Vec<(usize, usize)>> {
    let mut parser = Parser::new();
    parser.set_language(&lang.language())?;
    let tree = parser.parse(source, None).ok_or_else(|| anyhow::anyhow!("tree-sitter could not parse"))?;
    let mut spans = Vec::new();
    walk(tree.root_node(), source, lang, &mut spans, skip);
    Ok(spans)
}

/// Byte ranges where the counter's own parse failed (tree-sitter `ERROR`/missing nodes),
/// for C and C++ only: unexpanded macros make some source unparsable without a
/// preprocessor, so the counter cannot vouch for what is there. The M0 check leaves
/// functions touching these ranges out of both counts and reports them as unchecked.
/// Rust and TypeScript parse without a preprocessor, so their counts stay strict.
pub fn unparsed_ranges(lang: Lang, source: &str) -> anyhow::Result<Vec<(usize, usize)>> {
    if !matches!(lang, Lang::C | Lang::Cpp) {
        return Ok(Vec::new());
    }
    let mut parser = Parser::new();
    parser.set_language(&lang.language())?;
    let tree = parser.parse(source, None).ok_or_else(|| anyhow::anyhow!("tree-sitter could not parse"))?;
    fn go(n: Node, out: &mut Vec<(usize, usize)>) {
        if n.is_error() || n.is_missing() {
            out.push((n.start_byte(), n.end_byte().max(n.start_byte() + 1)));
            return;
        }
        if n.has_error() {
            let mut c = n.walk();
            for ch in n.children(&mut c) {
                go(ch, out);
            }
        }
    }
    let mut out = Vec::new();
    go(tree.root_node(), &mut out);
    Ok(out)
}

/// The syntax tree's named nodes between two lines, indented, for debugging a mismatch.
pub fn outline(lang: Lang, source: &str, from: usize, to: usize) -> anyhow::Result<String> {
    let mut parser = Parser::new();
    parser.set_language(&lang.language())?;
    let tree = parser.parse(source, None).ok_or_else(|| anyhow::anyhow!("tree-sitter could not parse"))?;
    let mut out = String::new();
    fn go(n: Node, depth: usize, from: usize, to: usize, out: &mut String) {
        let (s, e) = (n.start_position().row + 1, n.end_position().row + 1);
        if e < from || s > to {
            return;
        }
        if n.is_named() {
            out.push_str(&format!("{}{} {}-{}\n", "  ".repeat(depth), n.kind(), s, e));
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            go(ch, depth + 1, from, to, out);
        }
    }
    go(tree.root_node(), 0, from, to, &mut out);
    Ok(out)
}

fn walk(node: Node, src: &str, lang: Lang, n: &mut Vec<(usize, usize)>, skip: &[(usize, usize)]) {
    if lang == Lang::Rust {
        if node.kind() == "macro_definition" {
            return;
        }
        if node.kind() == "token_tree" {
            if node.parent().is_some_and(|p| p.kind() == "macro_invocation") {
                let k = count_token_tree(node, src);
                n.extend(std::iter::repeat_n((node.start_byte(), node.end_byte()), k));
            }
            return;
        }
    }
    if is_function(node, lang) && !skip.iter().any(|(s, e)| (*s..*e).contains(&node.start_byte())) {
        n.push((node.start_byte(), node.end_byte()));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, lang, n, skip);
    }
}

/// Counts functions in one token tree (`(…)`/`[…]`/`{…}`) of macro arguments:
/// 1. the whole content re-parsed as call arguments or block statements;
/// 2. else each comma-separated piece as an expression, a pattern, or the
///    value after a leading `key:` / `key =>` / `key =` (DSL-style macros);
/// 3. else the bracketed groups inside that piece, recursively.
fn count_token_tree(tt: Node, src: &str) -> usize {
    let text = &src[tt.start_byte()..tt.end_byte()];
    let inner = &text[1..text.len().saturating_sub(1)];
    if let Some(n) = reparse(inner, &[("fn __m() { __f(", "); }"), ("fn __m() {\n", "\n}")]) {
        return n;
    }
    let mut cursor = tt.walk();
    let children: Vec<Node> = tt.children(&mut cursor).collect();
    let inner_children = &children[1.min(children.len())..children.len().saturating_sub(1)];
    let mut total = 0;
    for piece in inner_children.split(|c| c.kind() == ",") {
        let (Some(first), Some(last)) = (piece.first(), piece.last()) else { continue };
        let seg = &src[first.start_byte()..last.end_byte()];
        let pieces = [("fn __m() { __f(", "); }"), ("fn __m() { let ", " = x; }")];
        if let Some(n) = reparse(seg, &pieces) {
            total += n;
            continue;
        }
        let value = piece.iter().position(|c| matches!(c.kind(), ":" | "=>" | "=")).and_then(|i| piece.get(i + 1));
        if let Some(n) = value.and_then(|v| reparse(&src[v.start_byte()..last.end_byte()], &pieces[..1])) {
            total += n;
            continue;
        }
        total += piece.iter().filter(|c| c.kind() == "token_tree").map(|c| count_token_tree(*c, src)).sum::<usize>();
    }
    total
}

/// Parses `code` inside each wrapper in turn; the first clean parse gives the count.
fn reparse(code: &str, wrappers: &[(&str, &str)]) -> Option<usize> {
    for (pre, post) in wrappers {
        let wrapped = format!("{pre}{code}{post}");
        let mut parser = Parser::new();
        parser.set_language(&Lang::Rust.language()).ok()?;
        let Some(tree) = parser.parse(&wrapped, None) else { continue };
        if tree.root_node().has_error() {
            continue;
        }
        let mut n = Vec::new();
        walk(tree.root_node(), &wrapped, Lang::Rust, &mut n, &[]);
        return Some(n.len() - 1); // the wrapper `__m`
    }
    None
}

/// True if a declarator is or contains (through pointers, references, parentheses and
/// qualifiers) a function declarator or a conversion operator.
fn has_function_declarator(d: Node) -> bool {
    if matches!(d.kind(), "function_declarator" | "operator_cast") {
        return true;
    }
    let mut cursor = d.walk();
    d.named_children(&mut cursor).any(has_function_declarator)
}

fn is_function(node: Node, lang: Lang) -> bool {
    // Keyword tokens share names with node kinds (`function`), so only named nodes count.
    if !node.is_named() {
        return false;
    }
    match lang {
        Lang::TypeScript | Lang::Tsx => matches!(
            node.kind(),
            "function_declaration"
                | "generator_function_declaration"
                | "function_expression"
                | "function"
                | "generator_function"
                | "arrow_function"
        ) || (node.kind() == "method_definition" && node.child_by_field_name("body").is_some()),
        Lang::Rust => {
            (node.kind() == "function_item" && node.child_by_field_name("body").is_some())
                || node.kind() == "closure_expression"
        }
        // A definition with a written body (`= default` / `= delete` have none), and lambdas.
        // A function definition always has a function declarator; without one, tree-sitter
        // has misread an unexpanded macro (`struct EXPORT Name { … }` as a "function" Name).
        Lang::C | Lang::Cpp => {
            (node.kind() == "function_definition"
                && node.child_by_field_name("body").is_some_and(|b| matches!(b.kind(), "compound_statement" | "try_statement"))
                && node.child_by_field_name("declarator").is_some_and(has_function_declarator))
                || node.kind() == "lambda_expression"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_typescript_functions_with_bodies_only() {
        let src = r#"
            function a() {}
            declare function sig(): void;
            const b = () => 1;
            const c = function () {};
            class K { constructor() {} m() {} get g() { return 1 } abstract x(): void; }
            interface I { m(): void }
            [1].map((x) => x);
        "#;
        // a, b, c, constructor, m, get g, the map callback
        assert_eq!(count_functions(Lang::TypeScript, src).unwrap(), 7);
    }

    #[test]
    fn counts_tsx_handlers() {
        let src = "const X = () => <div onClick={() => go()} />;";
        assert_eq!(count_functions(Lang::Tsx, src).unwrap(), 2);
    }

    #[test]
    fn counts_rust_functions_and_closures_outside_macros() {
        let src = r#"
            fn a() { let f = |x: i32| x + 1; }
            trait T { fn sig(&self); fn dflt(&self) {} }
            fn b() { println!("{}", (|| 1)()); }
            fn c() { assert!(v.iter().any(|x| matches!(x, A | B))); }
            macro_rules! m { ($x:expr) => { |a| a } }
            fn d() { thread_local! { static X: u8 = { let f = || 1; f() }; } }
        "#;
        // a, the closure in a, dflt, b, the println! closure, c, its any() closure,
        // d, and the closure in thread_local! (its body re-parses as block statements).
        // The macro_rules! template closure is not counted.
        assert_eq!(count_functions(Lang::Rust, src).unwrap(), 9);
    }

    #[test]
    fn counts_closures_in_dsl_and_pattern_macro_args() {
        let src = r#"
            fn a() {
                let v = json!({ "x": o.map(|c| c.x), "k": |y| y, "s": it.map(|s| json!({ "r": s.r() })) });
                let m = matches!(g.map(|s| &s.m), Some(M::C { .. }));
            }
        "#;
        // a + three closures in json! + one in matches!
        assert_eq!(count_functions(Lang::Rust, src).unwrap(), 5);
    }

    #[test]
    fn counts_cpp_definitions_and_lambdas() {
        let src = r#"
            int f(int);
            int f(int x) { return x; }
            struct S {
                S() = default;
                ~S() {}
                virtual void p() = 0;
                void m() const { auto g = [](int y) { return y; }; }
                template <class T> T t(T v) { return v; }
            };
            void S2::out() try { } catch (...) { }
            struct EXPORT_API Vec { float x; };
            #define M() { }
        "#;
        // f, ~S, m, the lambda, t, out
        assert_eq!(count_functions(Lang::Cpp, src).unwrap(), 6);
        let skip = [(src.find("void S2").unwrap(), src.find("#define").unwrap())];
        assert_eq!(count_functions_outside(Lang::Cpp, src, &skip).unwrap(), 5);
    }
}
