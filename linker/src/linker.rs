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
}
