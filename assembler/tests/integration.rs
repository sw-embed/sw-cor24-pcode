use pa24r::*;
#[allow(unused_imports)]
use std::collections::BTreeSet;

#[test]
fn assemble_hello() {
    let source = include_str!("../../vm/hello.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.entry_point, 0, "main should be at offset 0");
    assert_eq!(result.data.len(), 7, "Hello\\n\\0 = 7 bytes");
    assert!(!result.code.is_empty());
}

#[test]
fn assemble_hello_to_p24() {
    let source = include_str!("../../vm/hello.spc");
    let binary = assemble_to_p24(source).expect("assembly should succeed");
    assert_eq!(&binary[0..4], &P24_MAGIC);
    assert_eq!(binary[4], P24_VERSION);
    assert!(binary.len() > P24_HEADER_SIZE);
}

#[test]
fn round_trip_hello() {
    let source = include_str!("../../vm/hello.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty());

    let binary = assemble_to_p24(source).unwrap();
    let loaded = load_p24(&binary).unwrap();

    assert_eq!(loaded.entry_point, result.entry_point);
    assert_eq!(loaded.code, result.code);
    assert_eq!(loaded.data, result.data);
    assert_eq!(loaded.global_count, result.global_count);
}

#[test]
fn assemble_arith() {
    let source = include_str!("../../vm/tests/t02-arith.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_globals() {
    let source = include_str!("../../vm/tests/t03-globals.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    assert_eq!(result.global_count, 1);
}

#[test]
fn assemble_loop() {
    let source = include_str!("../../vm/tests/t04-loop.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_stack() {
    let source = include_str!("../../vm/tests/t05-stack.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_compare() {
    let source = include_str!("../../vm/tests/t06-compare.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_memory() {
    let source = include_str!("../../vm/tests/t07-memory.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_nested() {
    let source = include_str!("../../vm/tests/t08-nested.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_bitwise() {
    let source = include_str!("../../vm/tests/t09-bitwise.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn assemble_traps() {
    let source = include_str!("../../vm/tests/t10-traps.spc");
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn round_trip_all_tests() {
    let test_files = [
        include_str!("../../vm/tests/t01-hello.spc"),
        include_str!("../../vm/tests/t02-arith.spc"),
        include_str!("../../vm/tests/t03-globals.spc"),
        include_str!("../../vm/tests/t04-loop.spc"),
        include_str!("../../vm/tests/t05-stack.spc"),
        include_str!("../../vm/tests/t06-compare.spc"),
        include_str!("../../vm/tests/t07-memory.spc"),
        include_str!("../../vm/tests/t08-nested.spc"),
        include_str!("../../vm/tests/t09-bitwise.spc"),
        include_str!("../../vm/tests/t10-traps.spc"),
    ];

    for (i, source) in test_files.iter().enumerate() {
        let binary = assemble_to_p24(source)
            .unwrap_or_else(|e| panic!("test file {i}: assembly failed: {e:?}"));
        let loaded =
            load_p24(&binary).unwrap_or_else(|e| panic!("test file {i}: load failed: {e}"));
        let result = assemble(source);
        assert_eq!(loaded.code, result.code, "test file {i}: code mismatch");
        assert_eq!(loaded.data, result.data, "test file {i}: data mismatch");
    }
}

#[test]
fn load_p24_errors() {
    // Too short
    assert!(matches!(load_p24(&[]), Err(LoadError::TooShort)));
    assert!(matches!(load_p24(&[0; 10]), Err(LoadError::TooShort)));

    // Bad magic
    let mut bad = vec![0u8; 18];
    assert!(matches!(load_p24(&bad), Err(LoadError::BadMagic)));

    // Bad version
    bad[0..4].copy_from_slice(&P24_MAGIC);
    bad[4] = 99;
    assert!(matches!(load_p24(&bad), Err(LoadError::BadVersion(99))));

    // Truncated
    bad[4] = P24_VERSION;
    bad[8] = 10; // code_size = 10 but body is empty
    assert!(matches!(load_p24(&bad), Err(LoadError::Truncated)));
}

#[test]
fn error_unknown_mnemonic() {
    let source = ".proc main 0\nbogus\nhalt\n.end\n";
    let result = assemble(source);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("unknown mnemonic"));
}

#[test]
fn error_unresolved_symbol() {
    let source = ".proc main 0\ncall nonexistent\nhalt\n.end\n";
    let result = assemble(source);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].message.contains("unresolved symbol"));
}

#[test]
fn error_missing_main() {
    let source = ".proc foo 0\nhalt\n.end\n";
    let result = assemble(source);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("missing .proc main"))
    );
}

#[test]
fn metadata_directives_skipped() {
    let source = "\
.module test
.export main
.extern _runtime_fn
.proc main 0
    halt
.end
.endmodule
";
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    // enter 0 (auto from .proc), halt, leave (auto from .end)
    assert_eq!(result.code, vec![0x40, 0x00, 0x00, 0x41]);
}

#[test]
fn const_symbol_resolution() {
    let source = "\
.const ANSWER 42
.proc main 0
    push ANSWER
    halt
.end
";
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    // enter 0 (auto), push 42, halt, leave (auto)
    assert_eq!(result.code, vec![0x40, 0x00, 0x01, 42, 0, 0, 0x00, 0x41]);
}

/// Ground truth test: verify all opcode values match pvm.s exactly.
/// If pvm.s changes, this test must be updated to match.
/// Format: (mnemonic, expected_byte, expected_encoding_size)
#[test]
fn opcode_table_matches_pvm_s() {
    // Source: pvm.s opcode comments (grep for '; 0x')
    let pvm_s_opcodes: &[(&str, u8, usize)] = &[
        // Stack (0x00-0x06)
        ("halt", 0x00, 1),
        ("push", 0x01, 4),
        ("push_s", 0x02, 2),
        ("dup", 0x03, 1),
        ("drop", 0x04, 1),
        ("swap", 0x05, 1),
        ("over", 0x06, 1),
        // Arithmetic (0x10-0x15)
        ("add", 0x10, 1),
        ("sub", 0x11, 1),
        ("mul", 0x12, 1),
        ("div", 0x13, 1),
        ("mod", 0x14, 1),
        ("neg", 0x15, 1),
        // Logic (0x16-0x1B)
        ("and", 0x16, 1),
        ("or", 0x17, 1),
        ("xor", 0x18, 1),
        ("not", 0x19, 1),
        ("shl", 0x1A, 1),
        ("shr", 0x1B, 1),
        // Comparison (0x20-0x25)
        ("eq", 0x20, 1),
        ("ne", 0x21, 1),
        ("lt", 0x22, 1),
        ("le", 0x23, 1),
        ("gt", 0x24, 1),
        ("ge", 0x25, 1),
        // Control flow (0x30-0x36)
        ("jmp", 0x30, 4),
        ("jz", 0x31, 4),
        ("jnz", 0x32, 4),
        ("call", 0x33, 4),
        ("ret", 0x34, 2),
        ("calln", 0x35, 5),
        ("trap", 0x36, 2),
        // Local/Global/Nonlocal (0x40-0x4B)
        ("enter", 0x40, 2),
        ("leave", 0x41, 1),
        ("loadl", 0x42, 2),
        ("storel", 0x43, 2),
        ("loadg", 0x44, 4),
        ("storeg", 0x45, 4),
        ("addrl", 0x46, 2),
        ("addrg", 0x47, 4),
        ("loada", 0x48, 2),
        ("storea", 0x49, 2),
        ("loadn", 0x4A, 3),
        ("storen", 0x4B, 3),
        // Memory indirect (0x50-0x53)
        ("load", 0x50, 1),
        ("store", 0x51, 1),
        ("loadb", 0x52, 1),
        ("storeb", 0x53, 1),
        // System (0x60)
        ("sys", 0x60, 2),
    ];

    for &(mnemonic, expected_byte, expected_size) in pvm_s_opcodes {
        let op = Opcode::from_mnemonic(mnemonic)
            .unwrap_or_else(|| panic!("mnemonic '{mnemonic}' not recognized"));
        assert_eq!(
            op as u8, expected_byte,
            "opcode '{mnemonic}': expected 0x{expected_byte:02X}, got 0x{:02X}",
            op as u8
        );
        assert_eq!(
            op.size(),
            expected_size,
            "opcode '{mnemonic}': expected size {expected_size}, got {}",
            op.size()
        );
    }
}

// ---------------------------------------------------------------------------
// Sync tests: pvm.s, pvmasm.s, and pasm.s must agree on dispatch table size,
// bounds-check constant, and mnemonic entries.
// ---------------------------------------------------------------------------

/// Extract the dispatch-table `.word` entries from an assembly source between
/// `dispatch_table:` and the next `; ====` separator.
fn dispatch_entries(src: &str) -> Vec<String> {
    let mut in_table = false;
    let mut entries = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed == "dispatch_table:" {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        if trimmed.starts_with("; ====") {
            break;
        }
        if trimmed.starts_with(".word ") {
            entries.push(trimmed.to_string());
        }
    }
    entries
}

/// Extract the bounds-check constant from `lc r2, <N>` that precedes
/// `brt opcode_ok`.
fn bounds_check_value(src: &str) -> Option<u32> {
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("brt opcode_ok") {
            for j in (0..i).rev() {
                let t = lines[j].trim();
                if t.starts_with("lc r2,") {
                    let val = t.strip_prefix("lc r2,")?.trim();
                    return val.parse().ok();
                }
            }
        }
    }
    None
}

/// Extract mnemonic entries from `; "name" opcode=N type=T` comment lines.
/// Returns set of (name, opcode).
fn mnemonic_entries(src: &str) -> BTreeSet<(String, u32)> {
    let mut set = BTreeSet::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("; \"")
            && let Some(quote_end) = rest.find('"')
        {
            let name = &rest[..quote_end];
            if let Some(opcode_part) = rest.find("opcode=") {
                let after = &rest[opcode_part + 7..];
                let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(opcode) = num_str.parse::<u32>() {
                    set.insert((name.to_string(), opcode));
                }
            }
        }
    }
    set
}

#[test]
fn vm_dispatch_tables_in_sync() {
    let pvm = include_str!("../../vm/pvm.s");
    let pvmasm = include_str!("../../vm/pvmasm.s");

    let pvm_entries = dispatch_entries(pvm);
    let pvmasm_entries = dispatch_entries(pvmasm);

    assert_eq!(
        pvm_entries.len(),
        pvmasm_entries.len(),
        "dispatch table size mismatch: pvm.s has {} entries, pvmasm.s has {}",
        pvm_entries.len(),
        pvmasm_entries.len()
    );

    for (i, (a, b)) in pvm_entries.iter().zip(pvmasm_entries.iter()).enumerate() {
        assert_eq!(
            a, b,
            "dispatch table entry 0x{i:02X} differs: pvm.s has {a}, pvmasm.s has {b}"
        );
    }
}

#[test]
fn vm_bounds_checks_in_sync() {
    let pvm = include_str!("../../vm/pvm.s");
    let pvmasm = include_str!("../../vm/pvmasm.s");

    let pvm_bound = bounds_check_value(pvm).expect("could not find bounds check in pvm.s");
    let pvmasm_bound = bounds_check_value(pvmasm).expect("could not find bounds check in pvmasm.s");

    assert_eq!(
        pvm_bound, pvmasm_bound,
        "bounds check mismatch: pvm.s uses {pvm_bound}, pvmasm.s uses {pvmasm_bound}"
    );

    let table_size = dispatch_entries(pvm).len() as u32;
    assert_eq!(
        pvm_bound, table_size,
        "bounds check ({pvm_bound}) != dispatch table entry count ({table_size})"
    );
}

#[test]
fn assembler_mnemonic_tables_in_sync() {
    let pasm = include_str!("../../vm/pasm.s");
    let pvmasm = include_str!("../../vm/pvmasm.s");

    let pasm_mnemonics = mnemonic_entries(pasm);
    let pvmasm_mnemonics = mnemonic_entries(pvmasm);

    let only_in_pasm: Vec<_> = pasm_mnemonics.difference(&pvmasm_mnemonics).collect();
    let only_in_pvmasm: Vec<_> = pvmasm_mnemonics.difference(&pasm_mnemonics).collect();

    assert!(
        only_in_pasm.is_empty() && only_in_pvmasm.is_empty(),
        "mnemonic table mismatch:\n  only in pasm.s: {only_in_pasm:?}\n  only in pvmasm.s: {only_in_pvmasm:?}"
    );
}

// ---------------------------------------------------------------------------
// P24 v2 format tests: unit mode with export/import tables
// ---------------------------------------------------------------------------

#[test]
fn unit_mode_collects_exports() {
    let source = "\
.unit mathlib
.export add_nums 2

.proc add_nums 2
    loada 0
    loada 1
    add
    ret 2
.end

.proc main 0
    halt
.end
";
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let info = result.unit_info.as_ref().expect("should have unit_info");
    assert_eq!(info.name, "mathlib");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, "add_nums");
    assert_eq!(info.exports[0].offset, 0); // first proc
}

#[test]
fn unit_mode_collects_imports() {
    let source = "\
.unit app
.import mathlib
.extern add_nums 2

.proc main 0
    push 3
    push 4
    xcall add_nums
    halt
.end
";
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let info = result.unit_info.as_ref().expect("should have unit_info");
    assert_eq!(info.name, "app");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].unit_name, "mathlib");
    assert_eq!(info.imports[0].proc_name, "add_nums");
    assert_eq!(info.imports[0].slot, 0);
    assert_eq!(info.imported_units, vec!["mathlib"]);
}

#[test]
fn unit_mode_xcall_encoding() {
    let source = "\
.unit app
.import mathlib
.extern gcd 2
.extern factorial 1

.proc main 0
    push 1
    push 2
    xcall gcd
    push 5
    xcall factorial
    halt
.end
";
    let result = assemble(source);
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    // Find xcall opcodes in code
    let code = &result.code;
    let mut xcall_slots = Vec::new();
    let mut pc = 0;
    while pc < code.len() {
        if code[pc] == 0x74 {
            let slot = code[pc + 1] as u16 | ((code[pc + 2] as u16) << 8);
            xcall_slots.push(slot);
            pc += 3;
        } else {
            pc += opcode_size(code[pc]);
        }
    }
    assert_eq!(xcall_slots, vec![0, 1], "gcd=slot0, factorial=slot1");
}

#[test]
fn v2_binary_round_trip() {
    let source = "\
.unit mathlib
.export double 1

.proc double 1
    loada 0
    dup
    add
    ret 1
.end

.proc main 0
    halt
.end
";
    let binary = assemble_to_p24(source).expect("assembly should succeed");

    // Check v2 magic + version
    assert_eq!(&binary[0..4], &P24_MAGIC);
    assert_eq!(binary[4], P24_VERSION_2);

    // Round-trip load
    let loaded = load_p24(&binary).expect("load should succeed");
    assert_eq!(loaded.version, P24_VERSION_2);
    let info = loaded.unit_info.as_ref().expect("should have unit_info");
    assert_eq!(info.name, "mathlib");
    assert_eq!(info.exports.len(), 1);
    assert_eq!(info.exports[0].name, "double");

    // Code should match
    let result = assemble(source);
    assert_eq!(loaded.code, result.code);
    assert_eq!(loaded.data, result.data);
}

#[test]
fn v2_with_imports_round_trip() {
    let source = "\
.unit app
.import mathlib
.extern double 1

.proc main 0
    push 21
    xcall double
    halt
.end
";
    let binary = assemble_to_p24(source).expect("assembly should succeed");
    assert_eq!(binary[4], P24_VERSION_2);

    let loaded = load_p24(&binary).expect("load should succeed");
    let info = loaded.unit_info.as_ref().expect("should have unit_info");
    assert_eq!(info.name, "app");
    assert_eq!(info.imports.len(), 1);
    assert_eq!(info.imports[0].unit_name, "mathlib");
    assert_eq!(info.imports[0].proc_name, "double");
    assert_eq!(info.imports[0].slot, 0);
}

#[test]
fn non_unit_mode_still_v1() {
    // Without .unit directive, should emit v1 (existing behavior)
    let source = "\
.module test
.export main
.proc main 0
    halt
.end
.endmodule
";
    let binary = assemble_to_p24(source).expect("assembly should succeed");
    assert_eq!(binary[4], P24_VERSION, "should be v1 without .unit");

    let loaded = load_p24(&binary).expect("load should succeed");
    assert_eq!(loaded.version, P24_VERSION);
    assert!(loaded.unit_info.is_none());
}

#[test]
fn v1_still_loads() {
    // Existing v1 binaries must continue to load
    let source = "\
.proc main 0
    push 42
    halt
.end
";
    let binary = assemble_to_p24(source).expect("assembly should succeed");
    assert_eq!(binary[4], P24_VERSION);
    let loaded = load_p24(&binary).expect("v1 load should succeed");
    assert_eq!(loaded.version, P24_VERSION);
    assert!(loaded.unit_info.is_none());
}

#[test]
fn fnv1a_hash_deterministic() {
    assert_eq!(fnv1a_16(b"gcd"), fnv1a_16(b"gcd"));
    assert_ne!(fnv1a_16(b"gcd"), fnv1a_16(b"factorial"));
    assert_ne!(fnv1a_16(b"mathlib"), fnv1a_16(b"app"));
}

#[test]
fn xcall_opcode_value_and_size() {
    assert_eq!(Opcode::XCall as u8, 0x74);
    assert_eq!(Opcode::XCall.size(), 3);
    assert_eq!(Opcode::XCall.encoding(), Encoding::Imm16);
}

#[test]
fn encoding_imm16_size() {
    assert_eq!(Encoding::Imm16.size(), 3);
}
