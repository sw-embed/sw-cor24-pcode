use pa24r::*;

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
