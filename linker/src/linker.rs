//! Core linker for pl24r — the COR24 p-code linker.
//!
//! Merges multiple parsed .spc modules into a single linked output,
//! ordering modules correctly and merging declarations by type.
//! Language-agnostic: operates on .spc files from any COR24 compiler frontend.

use crate::parser::{Const, Data, Global, Item, Module, Proc, Statement};

/// The merged output of linking multiple .spc modules.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkedOutput {
    /// Header comments (preserved from all modules).
    pub comments: Vec<String>,
    /// Merged global declarations.
    pub globals: Vec<Global>,
    /// Merged data declarations.
    pub data: Vec<Data>,
    /// Merged constant declarations.
    pub consts: Vec<Const>,
    /// All procedures in link order (runtime/libs first, main module last).
    pub procs: Vec<Proc>,
}

/// Error encountered during linking.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkError {
    pub message: String,
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LinkError {}

/// Link multiple parsed modules into a single merged output.
///
/// Modules are reordered: modules without `main` come first (in input order),
/// then the module containing `main` comes last as the entry point.
/// Linker-only metadata (.module, .endmodule, .export, .extern) is stripped.
pub fn link(modules: &[Module]) -> Result<LinkedOutput, Vec<LinkError>> {
    let mut errors = Vec::new();

    // Find the app module (contains main).
    let main_index = modules.iter().position(|m| {
        m.items
            .iter()
            .any(|item| matches!(item, Item::Proc(p) if p.name == "main"))
    });

    let Some(main_index) = main_index else {
        errors.push(LinkError {
            message: "no module contains a 'main' procedure".to_string(),
        });
        return Err(errors);
    };

    // Order: main module first (VM starts execution at code offset 0),
    // then remaining modules in input order.
    let mut ordered: Vec<&Module> = Vec::with_capacity(modules.len());
    ordered.push(&modules[main_index]);
    for (i, m) in modules.iter().enumerate() {
        if i != main_index {
            ordered.push(m);
        }
    }

    let mut comments = Vec::new();
    let mut globals = Vec::new();
    let mut data = Vec::new();
    let mut consts = Vec::new();
    let mut procs = Vec::new();

    let mut seen_globals = std::collections::HashSet::new();
    let mut seen_consts = std::collections::HashMap::new();

    for module in &ordered {
        // Add a module separator comment for debuggability.
        comments.push(format!("; --- module: {} ---", module.name));

        for item in &module.items {
            match item {
                Item::Comment(c) => comments.push(c.clone()),
                Item::Global(g) => {
                    if seen_globals.insert(g.name.clone()) {
                        globals.push(g.clone());
                    }
                    // Duplicate globals with same name are silently merged
                    // (symbol table already validated no conflicts).
                }
                Item::Data(d) => {
                    data.push(d.clone());
                }
                Item::Const(c) => {
                    if let Some(prev_val) = seen_consts.get(&c.name) {
                        if *prev_val != c.value {
                            errors.push(LinkError {
                                message: format!(
                                    "conflicting constant '{}': '{}' vs '{}'",
                                    c.name, prev_val, c.value
                                ),
                            });
                        }
                    } else {
                        seen_consts.insert(c.name.clone(), c.value.clone());
                        consts.push(c.clone());
                    }
                }
                Item::Proc(p) => {
                    procs.push(p.clone());
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(LinkedOutput {
        comments,
        globals,
        data,
        consts,
        procs,
    })
}

/// Emit a linked output as .spc text, ready for pasm.
pub fn emit(output: &LinkedOutput) -> String {
    let mut lines = Vec::new();

    // Header comments.
    for c in &output.comments {
        lines.push(c.clone());
    }
    if !output.comments.is_empty() {
        lines.push(String::new());
    }

    // Globals section.
    for g in &output.globals {
        lines.push(format!(".global {} {}", g.name, g.nwords));
    }
    if !output.globals.is_empty() {
        lines.push(String::new());
    }

    // Data section.
    for d in &output.data {
        let bytes_str: Vec<String> = d.bytes.iter().map(|b| b.to_string()).collect();
        lines.push(format!(".data {} {}", d.name, bytes_str.join(",")));
    }
    if !output.data.is_empty() {
        lines.push(String::new());
    }

    // Constants section.
    for c in &output.consts {
        lines.push(format!(".const {} {}", c.name, c.value));
    }
    if !output.consts.is_empty() {
        lines.push(String::new());
    }

    // Procedures.
    for p in &output.procs {
        lines.push(format!(".proc {} {}", p.name, p.nlocals));
        for stmt in &p.body {
            match stmt {
                Statement::Label(l) => lines.push(format!("{l}:")),
                Statement::Instruction(i) => {
                    let mut s = format!("    {}", i.mnemonic);
                    if let Some(op) = &i.operand {
                        s.push(' ');
                        s.push_str(op);
                    }
                    if let Some(c) = &i.comment {
                        // Pad to align comments.
                        let pad = if s.len() < 24 { 24 - s.len() } else { 1 };
                        s.push_str(&" ".repeat(pad));
                        s.push_str(c);
                    }
                    lines.push(s);
                }
                Statement::Comment(c) => lines.push(format!("    {c}")),
                Statement::Blank => lines.push(String::new()),
            }
        }
        lines.push(".end".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

/// The merged output of linking modules in unit mode.
/// Preserves .unit/.import/.export/.extern directives for the assembler.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitLinkedOutput {
    /// Unit name (from first module with .unit, or derived from main module).
    pub unit_name: String,
    /// Imported unit names.
    pub imports: Vec<String>,
    /// Exported symbol names.
    pub exports: Vec<String>,
    /// External symbol names (unresolved — require .import).
    pub externs: Vec<String>,
    /// The merged linked output (same structure as static link).
    pub linked: LinkedOutput,
}

/// Link multiple modules in unit mode.
///
/// Like `link()`, but validates that:
/// - Every .export references a defined proc or global
/// - Every .extern is either defined internally (resolved, removed from externs)
///   or has a matching .import (left as .extern for the assembler)
/// - No .extern without .import
pub fn link_unit(
    modules: &[Module],
    unit_name: Option<&str>,
) -> Result<UnitLinkedOutput, Vec<LinkError>> {
    // First do the normal link (merges, orders main first).
    let linked = link(modules)?;

    // Collect all defined proc and global names.
    let mut defined: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in &linked.procs {
        defined.insert(p.name.clone());
    }
    for g in &linked.globals {
        defined.insert(g.name.clone());
    }

    // Merge exports from all modules.
    let mut exports: Vec<String> = Vec::new();
    for m in modules {
        for e in &m.exports {
            if !exports.contains(e) {
                exports.push(e.clone());
            }
        }
    }

    // Merge imports from all modules.
    let mut imports: Vec<String> = Vec::new();
    for m in modules {
        for i in &m.imports {
            if !imports.contains(i) {
                imports.push(i.clone());
            }
        }
    }

    // Validate exports.
    let mut errors = Vec::new();
    for e in &exports {
        if !defined.contains(e) {
            errors.push(LinkError {
                message: format!(".export '{e}' not defined in any module"),
            });
        }
    }

    // Process externs: resolve internally or validate .import exists.
    let mut unresolved_externs: Vec<String> = Vec::new();
    for m in modules {
        for ext in &m.externs {
            if defined.contains(ext) {
                // Resolved internally — no extern needed.
                continue;
            }
            if imports.is_empty() {
                errors.push(LinkError {
                    message: format!(".extern '{ext}' not defined and no .import declared"),
                });
            } else if !unresolved_externs.contains(ext) {
                unresolved_externs.push(ext.clone());
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Determine unit name.
    let name = if let Some(n) = unit_name {
        n.to_string()
    } else {
        modules
            .iter()
            .find(|m| m.is_unit)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| {
                modules
                    .iter()
                    .find(|m| {
                        m.items
                            .iter()
                            .any(|i| matches!(i, Item::Proc(p) if p.name == "main"))
                    })
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "unit".to_string())
            })
    };

    Ok(UnitLinkedOutput {
        unit_name: name,
        imports,
        exports,
        externs: unresolved_externs,
        linked,
    })
}

/// Emit unit-linked output as .spc text with .unit/.import/.export/.extern preserved.
pub fn emit_unit(output: &UnitLinkedOutput) -> String {
    let mut lines = Vec::new();

    // Unit directive.
    lines.push(format!(".unit {}", output.unit_name));
    lines.push(String::new());

    // Imports.
    for imp in &output.imports {
        lines.push(format!(".import {imp}"));
    }

    // Exports.
    for exp in &output.exports {
        lines.push(format!(".export {exp}"));
    }

    // Externs (unresolved — need xcall).
    for ext in &output.externs {
        lines.push(format!(".extern {ext}"));
    }

    if !output.imports.is_empty() || !output.exports.is_empty() || !output.externs.is_empty() {
        lines.push(String::new());
    }

    // The rest is the same as emit() but without comments header.
    let linked = &output.linked;

    // Globals section.
    for g in &linked.globals {
        lines.push(format!(".global {} {}", g.name, g.nwords));
    }
    if !linked.globals.is_empty() {
        lines.push(String::new());
    }

    // Data section.
    for d in &linked.data {
        let bytes_str: Vec<String> = d.bytes.iter().map(|b| b.to_string()).collect();
        lines.push(format!(".data {} {}", d.name, bytes_str.join(",")));
    }
    if !linked.data.is_empty() {
        lines.push(String::new());
    }

    // Constants section.
    for c in &linked.consts {
        lines.push(format!(".const {} {}", c.name, c.value));
    }
    if !linked.consts.is_empty() {
        lines.push(String::new());
    }

    // Procedures.
    for p in &linked.procs {
        lines.push(format!(".proc {} {}", p.name, p.nlocals));
        for stmt in &p.body {
            match stmt {
                Statement::Label(l) => lines.push(format!("{l}:")),
                Statement::Instruction(i) => {
                    let mut s = format!("    {}", i.mnemonic);
                    if let Some(op) = &i.operand {
                        s.push(' ');
                        s.push_str(op);
                    }
                    if let Some(c) = &i.comment {
                        let pad = if s.len() < 24 { 24 - s.len() } else { 1 };
                        s.push_str(&" ".repeat(pad));
                        s.push_str(c);
                    }
                    lines.push(s);
                }
                Statement::Comment(c) => lines.push(format!("    {c}")),
                Statement::Blank => lines.push(String::new()),
            }
        }
        lines.push(".end".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn parse_ok(source: &str, filename: &str) -> Module {
        parse(source, filename).unwrap()
    }

    #[test]
    fn test_link_ordering_main_first() {
        let app = parse_ok(
            "\
.module app
.export main
.extern _p24p_write_int

.proc main 0
    push 42
    call _p24p_write_int
    halt
.end

.endmodule
",
            "app.spc",
        );

        let runtime = parse_ok(
            "\
.module runtime
.export _p24p_write_int

.proc _p24p_write_int 1
    enter 1
    loada 0
    halt
.end

.endmodule
",
            "runtime.spc",
        );

        // App first in input — main should stay first (VM starts at offset 0).
        let linked = link(&[app, runtime]).unwrap();
        assert_eq!(linked.procs.len(), 2);
        assert_eq!(linked.procs[0].name, "main");
        assert_eq!(linked.procs[1].name, "_p24p_write_int");
    }

    #[test]
    fn test_link_globals_merged() {
        let mod_a = parse_ok(
            "\
.module app
.export main

.global x 1
.global y 2

.proc main 0
    halt
.end

.endmodule
",
            "app.spc",
        );

        let mod_b = parse_ok(
            "\
.module lib
.export helper

.global z 1

.proc helper 0
    ret 0
.end

.endmodule
",
            "lib.spc",
        );

        let linked = link(&[mod_a, mod_b]).unwrap();
        assert_eq!(linked.globals.len(), 3);
        // Main module globals come first (main module is first in output).
        assert_eq!(linked.globals[0].name, "x");
        assert_eq!(linked.globals[1].name, "y");
        assert_eq!(linked.globals[2].name, "z");
    }

    #[test]
    fn test_link_duplicate_globals_merged() {
        let mod_a = parse_ok(
            "\
.global shared 1
.proc main 0
    halt
.end
",
            "a.spc",
        );

        let mod_b = parse_ok(
            "\
.global shared 1
.proc helper 0
    ret 0
.end
",
            "b.spc",
        );

        let linked = link(&[mod_a, mod_b]).unwrap();
        // Only one copy of 'shared'.
        assert_eq!(
            linked.globals.iter().filter(|g| g.name == "shared").count(),
            1
        );
    }

    #[test]
    fn test_link_const_conflict() {
        let mod_a = parse_ok(
            "\
.const MAX 255
.proc main 0
    halt
.end
",
            "a.spc",
        );

        let mod_b = parse_ok(
            "\
.const MAX 100
.proc helper 0
    ret 0
.end
",
            "b.spc",
        );

        let err = link(&[mod_a, mod_b]).unwrap_err();
        assert!(err[0].message.contains("conflicting constant 'MAX'"));
    }

    #[test]
    fn test_link_const_same_value_ok() {
        let mod_a = parse_ok(
            "\
.const MAX 255
.proc main 0
    halt
.end
",
            "a.spc",
        );

        let mod_b = parse_ok(
            "\
.const MAX 255
.proc helper 0
    ret 0
.end
",
            "b.spc",
        );

        let linked = link(&[mod_a, mod_b]).unwrap();
        assert_eq!(linked.consts.len(), 1);
        assert_eq!(linked.consts[0].name, "MAX");
    }

    #[test]
    fn test_link_data_merged() {
        let mod_a = parse_ok(
            "\
.data msg 72,101,108,108,111,0
.proc main 0
    halt
.end
",
            "a.spc",
        );

        let mod_b = parse_ok(
            "\
.data greeting 87,111,114,108,100,0
.proc helper 0
    ret 0
.end
",
            "b.spc",
        );

        let linked = link(&[mod_a, mod_b]).unwrap();
        assert_eq!(linked.data.len(), 2);
    }

    #[test]
    fn test_link_comments_preserved() {
        let mod_a = parse_ok(
            "\
; Module A header
.proc main 0
    halt
.end
",
            "a.spc",
        );

        let linked = link(&[mod_a]).unwrap();
        assert!(
            linked
                .comments
                .iter()
                .any(|c| c.contains("Module A header"))
        );
        assert!(
            linked
                .comments
                .iter()
                .any(|c| c.contains("--- module: a ---"))
        );
    }

    #[test]
    fn test_link_no_main_error() {
        let lib = parse_ok(
            "\
.module lib
.export helper

.proc helper 0
    ret 0
.end

.endmodule
",
            "lib.spc",
        );

        let err = link(&[lib]).unwrap_err();
        assert!(
            err[0]
                .message
                .contains("no module contains a 'main' procedure")
        );
    }

    #[test]
    fn test_emit_roundtrip() {
        let runtime = parse_ok(
            "\
; runtime library

.proc _p24p_write_int 1
    enter 1
    loada 0
    halt
.end
",
            "runtime.spc",
        );

        let app = parse_ok(
            "\
; app module

.global count 1

.data msg 72,101,108,108,111,0

.const MAX 255

.proc main 0
    push 42
    call _p24p_write_int
    halt
.end
",
            "app.spc",
        );

        let linked = link(&[runtime, app]).unwrap();
        let output = emit(&linked);

        // Output should parse back cleanly.
        let reparsed = parse(&output, "linked.spc").unwrap();
        assert!(
            reparsed
                .items
                .iter()
                .any(|i| matches!(i, Item::Proc(p) if p.name == "_p24p_write_int"))
        );
        assert!(
            reparsed
                .items
                .iter()
                .any(|i| matches!(i, Item::Proc(p) if p.name == "main"))
        );
        assert!(
            reparsed
                .items
                .iter()
                .any(|i| matches!(i, Item::Global(g) if g.name == "count"))
        );
        assert!(
            reparsed
                .items
                .iter()
                .any(|i| matches!(i, Item::Data(d) if d.name == "msg"))
        );
        assert!(
            reparsed
                .items
                .iter()
                .any(|i| matches!(i, Item::Const(c) if c.name == "MAX"))
        );
    }

    #[test]
    fn test_emit_proc_with_labels() {
        let m = parse_ok(
            "\
.proc _p24p_write_int 1
    enter 1
    loada 0
    push 0
    lt
    jz positive
    push 45
    sys 1
    neg
positive:
    storel 0
done:
    drop
    leave
    ret 1
.end
",
            "runtime.spc",
        );

        let app = parse_ok(
            "\
.proc main 0
    push 42
    call _p24p_write_int
    halt
.end
",
            "app.spc",
        );

        let linked = link(&[m, app]).unwrap();
        let output = emit(&linked);

        // Labels should be preserved.
        assert!(output.contains("positive:"));
        assert!(output.contains("done:"));

        // Should reparse cleanly.
        let reparsed = parse(&output, "linked.spc").unwrap();
        let proc = reparsed.items.iter().find_map(|i| match i {
            Item::Proc(p) if p.name == "_p24p_write_int" => Some(p),
            _ => None,
        });
        assert!(proc.is_some());
        let labels: Vec<_> = proc
            .unwrap()
            .body
            .iter()
            .filter_map(|s| match s {
                Statement::Label(l) => Some(l.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, vec!["positive", "done"]);
    }

    #[test]
    fn test_emit_strips_module_metadata() {
        let app = parse_ok(
            "\
.module app
.export main
.extern _p24p_write_int

.proc main 0
    halt
.end

.endmodule
",
            "app.spc",
        );

        let linked = link(&[app]).unwrap();
        let output = emit(&linked);

        // Output should not contain linker metadata.
        assert!(!output.contains(".module"));
        assert!(!output.contains(".export"));
        assert!(!output.contains(".extern"));
        assert!(!output.contains(".endmodule"));
    }

    #[test]
    fn test_emit_instruction_with_comment() {
        let m = parse_ok(
            "\
.proc main 0
    push 45              ; '-'
    sys 1                ; PUTC
    halt
.end
",
            "test.spc",
        );

        let linked = link(&[m]).unwrap();
        let output = emit(&linked);

        assert!(output.contains("; '-'"));
        assert!(output.contains("; PUTC"));
    }

    #[test]
    fn test_link_three_modules() {
        let runtime = parse_ok(
            "\
.module runtime
.export _p24p_write_int
.export _p24p_write_ln

.proc _p24p_write_int 1
    enter 1
    loada 0
    halt
.end

.proc _p24p_write_ln 0
    push 10
    sys 1
    ret 0
.end

.endmodule
",
            "runtime.spc",
        );

        let mathlib = parse_ok(
            "\
.module mathlib
.export square

.proc square 1
    loada 0
    dup
    mul
    ret 1
.end

.endmodule
",
            "mathlib.spc",
        );

        let app = parse_ok(
            "\
.module app
.export main
.extern _p24p_write_int
.extern _p24p_write_ln
.extern square

.global result 1

.proc main 0
    push 7
    call square
    storeg result
    loadg result
    call _p24p_write_int
    call _p24p_write_ln
    halt
.end

.endmodule
",
            "app.spc",
        );

        let linked = link(&[app, runtime, mathlib]).unwrap();
        let output = emit(&linked);

        // Verify ordering: main proc first (VM starts at offset 0),
        // runtime and mathlib procs after.
        let main_pos = output.find(".proc main").unwrap();
        let write_int_pos = output.find(".proc _p24p_write_int").unwrap();
        let square_pos = output.find(".proc square").unwrap();
        assert!(main_pos < write_int_pos);
        assert!(main_pos < square_pos);

        // Verify globals are at top (before procs).
        let global_pos = output.find(".global result").unwrap();
        assert!(global_pos < main_pos);

        // Should reparse.
        let reparsed = parse(&output, "linked.spc").unwrap();
        assert_eq!(
            reparsed
                .items
                .iter()
                .filter(|i| matches!(i, Item::Proc(_)))
                .count(),
            4
        );
    }

    // ── Unit mode tests ──

    #[test]
    fn test_link_unit_preserves_directives() {
        let gcd_mod = parse_ok(
            "\
.unit mathlib
.export gcd 2

.proc main 0
    halt
.end

.proc gcd 2
    loada 1
    jz gcd_base
    loada 0
    loada 1
    mod
    loada 1
    call gcd
    ret 2
gcd_base:
    loada 0
    ret 2
.end

.endunit
",
            "gcd.spc",
        );

        let result = link_unit(&[gcd_mod], None).unwrap();
        assert_eq!(result.unit_name, "mathlib");
        assert_eq!(result.exports, vec!["gcd"]);
        assert!(result.externs.is_empty());
        assert!(result.imports.is_empty());

        let output = emit_unit(&result);
        assert!(output.contains(".unit mathlib"));
        assert!(output.contains(".export gcd"));
        assert!(output.contains(".proc main"));
        assert!(output.contains(".proc gcd"));
    }

    #[test]
    fn test_link_unit_with_imports() {
        let app = parse_ok(
            "\
.unit app
.import mathlib
.extern gcd 2

.proc main 0
    push_s 12
    push_s 8
    xcall gcd
    halt
.end

.endunit
",
            "app.spc",
        );

        let result = link_unit(&[app], None).unwrap();
        assert_eq!(result.unit_name, "app");
        assert_eq!(result.imports, vec!["mathlib"]);
        assert_eq!(result.externs, vec!["gcd"]);
        assert!(result.exports.is_empty());

        let output = emit_unit(&result);
        assert!(output.contains(".unit app"));
        assert!(output.contains(".import mathlib"));
        assert!(output.contains(".extern gcd"));
    }

    #[test]
    fn test_link_unit_resolves_internal_extern() {
        // Two modules: app uses .extern helper, but helper is defined in lib.
        // In unit mode, helper resolves internally and should NOT appear as .extern.
        let app = parse_ok(
            "\
.module app
.export main
.extern helper

.proc main 0
    call helper
    halt
.end

.endmodule
",
            "app.spc",
        );

        let lib = parse_ok(
            "\
.module lib
.export helper

.proc helper 0
    push_s 42
    sys 1
    ret 0
.end

.endmodule
",
            "lib.spc",
        );

        let result = link_unit(&[app, lib], Some("myunit")).unwrap();
        assert_eq!(result.unit_name, "myunit");
        // helper resolved internally — no unresolved externs
        assert!(result.externs.is_empty());
    }

    #[test]
    fn test_link_unit_error_undefined_export() {
        let m = parse_ok(
            "\
.unit bad
.export nonexistent

.proc main 0
    halt
.end

.endunit
",
            "bad.spc",
        );

        let err = link_unit(&[m], None).unwrap_err();
        assert!(err[0].message.contains("not defined"));
    }

    #[test]
    fn test_link_unit_error_extern_without_import() {
        let m = parse_ok(
            "\
.unit bad
.extern missing_fn

.proc main 0
    xcall missing_fn
    halt
.end

.endunit
",
            "bad.spc",
        );

        let err = link_unit(&[m], None).unwrap_err();
        assert!(err[0].message.contains("no .import"));
    }

    #[test]
    fn test_link_unit_output_assembles() {
        // Verify unit-linked output can be assembled by pa24r.
        // main does not have to be first — placed first here for simplicity.
        let m = parse_ok(
            "\
.unit mathlib
.export double 1

.proc main 0
    halt
.end

.proc double 1
    loada 0
    dup
    add
    ret 1
.end

.endunit
",
            "mathlib.spc",
        );

        let result = link_unit(&[m], None).unwrap();
        let output = emit_unit(&result);

        // Verify it assembles to a v2 .p24
        let binary = pa24r::assemble_to_p24(&output).expect("should assemble");
        assert_eq!(binary[4], pa24r::P24_VERSION_2);
        let loaded = pa24r::load_p24(&binary).unwrap();
        let info = loaded.unit_info.as_ref().expect("should have unit_info");
        assert_eq!(info.name, "mathlib");
        assert_eq!(info.exports.len(), 1);
        assert_eq!(info.exports[0].name, "double");
    }
}
