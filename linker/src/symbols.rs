//! Symbol table and validation for the pl24r linker.
//!
//! Collects symbols from parsed .spc modules, builds a global symbol table,
//! and validates cross-module references (duplicates, unresolved externs,
//! entry point, unused exports).

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::parser::{Item, Module};

/// The kind of symbol defined in a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Proc,
    Global,
    Data,
    Const,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Proc => write!(f, "proc"),
            SymbolKind::Global => write!(f, "global"),
            SymbolKind::Data => write!(f, "data"),
            SymbolKind::Const => write!(f, "const"),
        }
    }
}

/// A symbol defined in a specific module.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub module: String,
    pub exported: bool,
}

/// Result of symbol table validation.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTable {
    /// All symbols across all modules, keyed by symbol name.
    /// Only exported symbols are included.
    pub exports: HashMap<String, Symbol>,
    /// Warnings (e.g., unused exports).
    pub warnings: Vec<String>,
}

/// An error found during symbol validation.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolError {
    pub message: String,
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SymbolError {}

/// Collect all symbols defined in a module, respecting export rules.
fn collect_module_symbols(module: &Module) -> Vec<Symbol> {
    let export_all = !module.has_metadata;
    let export_set: HashSet<&str> = module.exports.iter().map(|s| s.as_str()).collect();

    let mut symbols = Vec::new();

    for item in &module.items {
        let (name, kind) = match item {
            Item::Proc(p) => (p.name.as_str(), SymbolKind::Proc),
            Item::Global(g) => (g.name.as_str(), SymbolKind::Global),
            Item::Data(d) => (d.name.as_str(), SymbolKind::Data),
            Item::Const(c) => (c.name.as_str(), SymbolKind::Const),
            Item::Comment(_) => continue,
        };

        let exported = export_all || export_set.contains(name);

        symbols.push(Symbol {
            name: name.to_string(),
            kind,
            module: module.name.clone(),
            exported,
        });
    }

    symbols
}

/// Build a global symbol table from multiple parsed modules and validate it.
///
/// Returns the symbol table on success, or a list of errors on failure.
/// Warnings (unused exports) are included in the successful result.
pub fn build_symbol_table(modules: &[Module]) -> Result<SymbolTable, Vec<SymbolError>> {
    let mut errors = Vec::new();
    let mut exports: HashMap<String, Symbol> = HashMap::new();

    // Collect all extern references across all modules.
    let mut all_externs: HashSet<String> = HashSet::new();
    for module in modules {
        for ext in &module.externs {
            all_externs.insert(ext.clone());
        }
    }

    // Phase 1: Collect exported symbols, detect duplicates.
    for module in modules {
        let symbols = collect_module_symbols(module);
        for sym in symbols {
            if !sym.exported {
                continue;
            }
            if let Some(existing) = exports.get(&sym.name) {
                errors.push(SymbolError {
                    message: format!(
                        "duplicate symbol '{}': exported by both '{}' and '{}'",
                        sym.name, existing.module, sym.module
                    ),
                });
            } else {
                exports.insert(sym.name.clone(), sym);
            }
        }
    }

    // Phase 2: Verify unresolved externs.
    for ext in &all_externs {
        if !exports.contains_key(ext) {
            errors.push(SymbolError {
                message: format!("unresolved extern '{ext}': not exported by any module"),
            });
        }
    }

    // Phase 3: Verify exactly one main proc exists.
    let main_modules: Vec<&str> = modules
        .iter()
        .filter(|m| {
            m.items
                .iter()
                .any(|item| matches!(item, Item::Proc(p) if p.name == "main"))
        })
        .map(|m| m.name.as_str())
        .collect();

    match main_modules.len() {
        0 => errors.push(SymbolError {
            message: "no 'main' procedure found in any module".to_string(),
        }),
        1 => {} // exactly right
        _ => errors.push(SymbolError {
            message: format!(
                "multiple 'main' procedures found in modules: {}",
                main_modules.join(", ")
            ),
        }),
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Phase 4: Warn on unused exports (exported but never referenced as extern).
    let mut warnings = Vec::new();
    for (name, sym) in &exports {
        if name == "main" {
            continue; // main is always used as the entry point
        }
        if !all_externs.contains(name) {
            warnings.push(format!(
                "unused export '{}' ({}) in module '{}'",
                name, sym.kind, sym.module
            ));
        }
    }
    warnings.sort();

    Ok(SymbolTable { exports, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_valid_multi_module_resolution() {
        let app = parse(
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
        )
        .unwrap();

        let runtime = parse(
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
        )
        .unwrap();

        let table = build_symbol_table(&[app, runtime]).unwrap();
        assert!(table.exports.contains_key("main"));
        assert!(table.exports.contains_key("_p24p_write_int"));
        assert_eq!(table.exports["main"].module, "app");
        assert_eq!(table.exports["_p24p_write_int"].module, "runtime");
        assert!(table.warnings.is_empty());
    }

    #[test]
    fn test_duplicate_symbol_detection() {
        let mod_a = parse(
            "\
.module alpha
.export foo

.proc foo 0
    halt
.end

.proc main 0
    halt
.end

.endmodule
",
            "alpha.spc",
        )
        .unwrap();

        let mod_b = parse(
            "\
.module beta
.export foo

.proc foo 0
    halt
.end

.endmodule
",
            "beta.spc",
        )
        .unwrap();

        let err = build_symbol_table(&[mod_a, mod_b]).unwrap_err();
        assert_eq!(err.len(), 1);
        assert!(err[0].message.contains("duplicate symbol 'foo'"));
        assert!(err[0].message.contains("alpha"));
        assert!(err[0].message.contains("beta"));
    }

    #[test]
    fn test_unresolved_extern_detection() {
        let app = parse(
            "\
.module app
.export main
.extern nonexistent

.proc main 0
    call nonexistent
    halt
.end

.endmodule
",
            "app.spc",
        )
        .unwrap();

        let err = build_symbol_table(&[app]).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.message.contains("unresolved extern 'nonexistent'"))
        );
    }

    #[test]
    fn test_missing_main_detection() {
        let lib = parse(
            "\
.module mylib
.export helper

.proc helper 0
    halt
.end

.endmodule
",
            "mylib.spc",
        )
        .unwrap();

        let err = build_symbol_table(&[lib]).unwrap_err();
        assert!(
            err.iter()
                .any(|e| e.message.contains("no 'main' procedure"))
        );
    }

    #[test]
    fn test_multiple_main_detection() {
        let mod_a = parse(
            "\
.module alpha
.export main

.proc main 0
    halt
.end

.endmodule
",
            "alpha.spc",
        )
        .unwrap();

        let mod_b = parse(
            "\
.module beta
.export main

.proc main 0
    halt
.end

.endmodule
",
            "beta.spc",
        )
        .unwrap();

        let err = build_symbol_table(&[mod_a, mod_b]).unwrap_err();
        // Should report both duplicate and multiple main
        assert!(
            err.iter()
                .any(|e| e.message.contains("duplicate symbol 'main'"))
        );
        assert!(
            err.iter()
                .any(|e| e.message.contains("multiple 'main' procedures"))
        );
    }

    #[test]
    fn test_export_all_fallback() {
        // No .module metadata — all symbols should be exported
        let legacy = parse(
            "\
.global count 1
.data msg 72,101,108,108,111,0

.proc main 0
    halt
.end

.proc helper 0
    ret 0
.end
",
            "legacy.spc",
        )
        .unwrap();

        let table = build_symbol_table(&[legacy]).unwrap();
        assert!(table.exports.contains_key("count"));
        assert!(table.exports.contains_key("msg"));
        assert!(table.exports.contains_key("main"));
        assert!(table.exports.contains_key("helper"));
        assert_eq!(table.exports["count"].kind, SymbolKind::Global);
        assert_eq!(table.exports["msg"].kind, SymbolKind::Data);
        assert_eq!(table.exports["main"].kind, SymbolKind::Proc);
    }

    #[test]
    fn test_unused_export_warning() {
        let app = parse(
            "\
.module app
.export main
.export unused_func

.proc main 0
    halt
.end

.proc unused_func 0
    ret 0
.end

.endmodule
",
            "app.spc",
        )
        .unwrap();

        let table = build_symbol_table(&[app]).unwrap();
        assert_eq!(table.warnings.len(), 1);
        assert!(table.warnings[0].contains("unused export 'unused_func'"));
    }

    #[test]
    fn test_selective_exports() {
        // Module with metadata exports only listed symbols
        let lib = parse(
            "\
.module mylib
.export public_func

.proc public_func 0
    call internal_func
    ret 0
.end

.proc internal_func 0
    ret 0
.end

.proc main 0
    halt
.end

.endmodule
",
            "mylib.spc",
        )
        .unwrap();

        let table = build_symbol_table(&[lib]).unwrap();
        assert!(table.exports.contains_key("public_func"));
        // internal_func is not exported
        assert!(!table.exports.contains_key("internal_func"));
        // main is not in the export list, so it's not exported
        // But we still detect main for entry point validation
    }

    #[test]
    fn test_extern_resolved_by_export_all() {
        let app = parse(
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
        )
        .unwrap();

        // Legacy module without metadata — exports everything
        let lib = parse(
            "\
.proc helper 0
    ret 0
.end
",
            "lib.spc",
        )
        .unwrap();

        let table = build_symbol_table(&[app, lib]).unwrap();
        assert!(table.exports.contains_key("helper"));
        assert_eq!(table.exports["helper"].module, "lib");
    }

    #[test]
    fn test_multiple_errors_reported() {
        let app = parse(
            "\
.module app
.extern missing1
.extern missing2

.proc not_main 0
    halt
.end

.endmodule
",
            "app.spc",
        )
        .unwrap();

        let err = build_symbol_table(&[app]).unwrap_err();
        // Should report: no main, two unresolved externs
        assert!(err.len() >= 3);
        assert!(err.iter().any(|e| e.message.contains("no 'main'")));
        assert!(err.iter().any(|e| e.message.contains("missing1")));
        assert!(err.iter().any(|e| e.message.contains("missing2")));
    }

    #[test]
    fn test_symbol_display() {
        assert_eq!(format!("{}", SymbolKind::Proc), "proc");
        assert_eq!(format!("{}", SymbolKind::Global), "global");
        assert_eq!(format!("{}", SymbolKind::Data), "data");
        assert_eq!(format!("{}", SymbolKind::Const), "const");
    }
}
