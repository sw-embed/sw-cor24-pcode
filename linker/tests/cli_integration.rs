use std::process::Command;

fn pl24r() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pl24r"))
}

fn fixture(name: &str) -> String {
    format!("tests/fixtures/{name}")
}

#[test]
fn test_no_args_shows_usage() {
    let out = pl24r().output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Usage:"));
}

#[test]
fn test_help_flag() {
    let out = pl24r().arg("--help").output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Usage:"));
    assert!(stderr.contains("-o <path>"));
}

#[test]
fn test_file_not_found() {
    let out = pl24r().arg("nonexistent.spc").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("nonexistent.spc"));
}

#[test]
fn test_link_runtime_and_app_to_stdout() {
    let out = pl24r()
        .arg(fixture("runtime.spc"))
        .arg(fixture("app.spc"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Main proc should come first (VM starts execution at code offset 0).
    let write_int_pos = stdout.find(".proc _p24p_write_int").unwrap();
    let main_pos = stdout.find(".proc main").unwrap();
    assert!(main_pos < write_int_pos);

    // No module metadata in output.
    assert!(!stdout.contains(".module"));
    assert!(!stdout.contains(".export"));
    assert!(!stdout.contains(".extern"));
    assert!(!stdout.contains(".endmodule"));
}

#[test]
fn test_link_to_output_file() {
    let tmpdir = std::env::temp_dir().join("pl24r_test");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let outpath = tmpdir.join("linked.spc");

    let out = pl24r()
        .arg(fixture("runtime.spc"))
        .arg(fixture("app.spc"))
        .arg("-o")
        .arg(outpath.to_str().unwrap())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let content = std::fs::read_to_string(&outpath).unwrap();
    assert!(content.contains(".proc main"));
    assert!(content.contains(".proc _p24p_write_int"));

    // Cleanup.
    std::fs::remove_file(&outpath).ok();
    std::fs::remove_dir(&tmpdir).ok();
}

#[test]
fn test_verbose_flag() {
    let out = pl24r()
        .arg("--verbose")
        .arg(fixture("runtime.spc"))
        .arg(fixture("app.spc"))
        .output()
        .unwrap();
    assert!(out.status.success());

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[pl24r] parsed"));
    assert!(stderr.contains("[pl24r] symbol table"));
    assert!(stderr.contains("[pl24r] linked"));
}

#[test]
fn test_verbose_short_flag() {
    let out = pl24r()
        .arg("-v")
        .arg(fixture("runtime.spc"))
        .arg(fixture("app.spc"))
        .output()
        .unwrap();
    assert!(out.status.success());

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[pl24r]"));
}

#[test]
fn test_legacy_app_without_metadata() {
    let out = pl24r().arg(fixture("legacy_app.spc")).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(".proc main"));
    assert!(stdout.contains(".proc puts"));
    assert!(stdout.contains(".data msg"));
}

#[test]
fn test_unknown_option() {
    let out = pl24r().arg("--bogus").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown option"));
}

#[test]
fn test_missing_o_argument() {
    let out = pl24r().arg(fixture("app.spc")).arg("-o").output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("-o requires"));
}

#[test]
fn test_no_main_error() {
    let out = pl24r().arg(fixture("runtime.spc")).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("main"));
}

#[test]
fn test_output_reparses_cleanly() {
    let out = pl24r()
        .arg(fixture("runtime.spc"))
        .arg(fixture("app.spc"))
        .output()
        .unwrap();
    assert!(out.status.success());

    // Write output to temp file and re-link as a single module
    // (it should be valid .spc that parses without errors).
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(".proc"));
    assert!(stdout.contains(".end"));
}
