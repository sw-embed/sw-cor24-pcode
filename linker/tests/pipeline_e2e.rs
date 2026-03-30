//! End-to-end pipeline tests using realistic runtime and app .spc files.
//!
//! These tests verify the full linker pipeline with fixtures based on the
//! real pr24p Pascal runtime library (phase 0) and a hand-written app that
//! exercises write_int, write_bool, and write_ln.

use std::process::Command;

fn pl24r() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pl24r"))
}

fn fixture(name: &str) -> String {
    format!("tests/fixtures/{name}")
}

/// Link the full pr24p runtime with an app and verify the output is valid .spc.
#[test]
fn test_e2e_runtime_app_link() {
    let out = pl24r()
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "link failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);

    // All runtime procs present.
    assert!(stdout.contains(".proc _p24p_write_int 1"));
    assert!(stdout.contains(".proc _p24p_write_bool 1"));
    assert!(stdout.contains(".proc _p24p_write_ln 0"));

    // App main present.
    assert!(stdout.contains(".proc main 0"));

    // Main proc comes first (VM starts execution at code offset 0).
    let write_int_pos = stdout.find(".proc _p24p_write_int").unwrap();
    let write_bool_pos = stdout.find(".proc _p24p_write_bool").unwrap();
    let write_ln_pos = stdout.find(".proc _p24p_write_ln").unwrap();
    let main_pos = stdout.find(".proc main").unwrap();
    assert!(main_pos < write_int_pos);
    assert!(main_pos < write_bool_pos);
    assert!(main_pos < write_ln_pos);

    // Module metadata is stripped.
    assert!(!stdout.contains(".module"));
    assert!(!stdout.contains(".export"));
    assert!(!stdout.contains(".extern"));
    assert!(!stdout.contains(".endmodule"));

    // Labels from runtime procedures are preserved.
    assert!(stdout.contains("positive:"));
    assert!(stdout.contains("extract:"));
    assert!(stdout.contains("print:"));
    assert!(stdout.contains("done:"));
    assert!(stdout.contains("print_false:"));
    assert!(stdout.contains("bool_done:"));

    // Inline comments are preserved.
    assert!(stdout.contains("; '-'"));
    assert!(stdout.contains("; LF"));
}

/// Verify the linked output reparses cleanly as valid .spc.
#[test]
fn test_e2e_output_reparses() {
    let out = pl24r()
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Write to temp file, re-link as single module to verify it parses.
    let tmpdir = std::env::temp_dir().join("pl24r_e2e_test");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let combined_path = tmpdir.join("combined.spc");
    std::fs::write(&combined_path, stdout.as_bytes()).unwrap();

    // The combined file should be valid .spc that pl24r can parse.
    let reparse = pl24r()
        .arg(combined_path.to_str().unwrap())
        .output()
        .unwrap();

    assert!(
        reparse.status.success(),
        "reparse failed: {}",
        String::from_utf8_lossy(&reparse.stderr)
    );

    // Cleanup.
    std::fs::remove_file(&combined_path).ok();
    std::fs::remove_dir(&tmpdir).ok();
}

/// Verify verbose output shows complete diagnostics for the e2e pipeline.
#[test]
fn test_e2e_verbose_diagnostics() {
    let out = pl24r()
        .arg("-v")
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .output()
        .unwrap();

    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Parse results for both files.
    assert!(stderr.contains("e2e_runtime.spc"));
    assert!(stderr.contains("module='runtime'"));
    assert!(stderr.contains("e2e_app.spc"));
    assert!(stderr.contains("module='app'"));

    // Symbol table shows all 3 exported runtime symbols.
    assert!(stderr.contains("_p24p_write_int"));
    assert!(stderr.contains("_p24p_write_bool"));
    assert!(stderr.contains("_p24p_write_ln"));

    // Linking stats: 4 procs (write_int, write_bool, write_ln, main).
    assert!(stderr.contains("4 procs"));
}

/// Link to output file and verify the file contents match stdout mode.
#[test]
fn test_e2e_file_output_matches_stdout() {
    let stdout_out = pl24r()
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .output()
        .unwrap();
    assert!(stdout_out.status.success());

    let tmpdir = std::env::temp_dir().join("pl24r_e2e_file_test");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let outpath = tmpdir.join("output.spc");

    let file_out = pl24r()
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .arg("-o")
        .arg(outpath.to_str().unwrap())
        .output()
        .unwrap();
    assert!(file_out.status.success());

    let file_content = std::fs::read_to_string(&outpath).unwrap();
    let stdout_content = String::from_utf8_lossy(&stdout_out.stdout);
    assert_eq!(file_content, stdout_content.as_ref());

    // Cleanup.
    std::fs::remove_file(&outpath).ok();
    std::fs::remove_dir(&tmpdir).ok();
}

/// Verify that linking with reversed input order still produces correct output.
/// The linker should always put the main module first regardless of input order.
#[test]
fn test_e2e_input_order_independent() {
    // runtime first
    let out1 = pl24r()
        .arg(fixture("e2e_runtime.spc"))
        .arg(fixture("e2e_app.spc"))
        .output()
        .unwrap();
    assert!(out1.status.success());

    // app first
    let out2 = pl24r()
        .arg(fixture("e2e_app.spc"))
        .arg(fixture("e2e_runtime.spc"))
        .output()
        .unwrap();
    assert!(out2.status.success());

    let stdout1 = String::from_utf8_lossy(&out1.stdout);
    let stdout2 = String::from_utf8_lossy(&out2.stdout);

    // Both should have main first (VM starts at code offset 0).
    let main_pos1 = stdout1.find(".proc main").unwrap();
    let first_proc1 = stdout1.find(".proc ").unwrap();
    assert_eq!(main_pos1, first_proc1);

    let main_pos2 = stdout2.find(".proc main").unwrap();
    let first_proc2 = stdout2.find(".proc ").unwrap();
    assert_eq!(main_pos2, first_proc2);
}

/// Verify linking runtime alone (without main) produces a clear error.
#[test]
fn test_e2e_runtime_only_error() {
    let out = pl24r().arg(fixture("e2e_runtime.spc")).output().unwrap();

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("main"));
}
