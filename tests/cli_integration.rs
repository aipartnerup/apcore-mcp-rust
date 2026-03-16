//! Integration tests for the apcore-mcp CLI binary.
//!
//! These tests invoke the compiled binary via `std::process::Command` and
//! verify exit codes and output for various argument combinations.

use std::process::Command;

/// Path to the compiled binary (built by `cargo build`).
fn binary_path() -> std::path::PathBuf {
    // cargo test sets the target directory; the binary lives alongside test binaries.
    let mut path = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("parent of test binary")
        .parent()
        .expect("parent of deps dir")
        .to_path_buf();
    path.push("apcore-mcp");
    path
}

#[test]
fn help_exits_zero() {
    let output = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "expected exit 0, got {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("extensions-dir"), "help should mention --extensions-dir");
    assert!(stdout.contains("apcore-mcp"), "help should mention apcore-mcp");
}

#[test]
fn missing_extensions_dir_exits_nonzero() {
    let output = Command::new(binary_path())
        .output()
        .expect("failed to run binary");
    assert!(
        !output.status.success(),
        "expected non-zero exit, got {:?}",
        output.status.code()
    );
    // clap exits with code 2 for missing required args
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn nonexistent_extensions_dir_exits_one() {
    let output = Command::new(binary_path())
        .args(["--extensions-dir", "/nonexistent/path/does/not/exist"])
        .output()
        .expect("failed to run binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for nonexistent extensions dir"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "stderr should mention missing dir: {stderr}"
    );
}

#[test]
fn port_zero_exits_nonzero() {
    let output = Command::new(binary_path())
        .args(["--extensions-dir", "/tmp", "--port", "0"])
        .output()
        .expect("failed to run binary");
    assert!(
        !output.status.success(),
        "expected non-zero exit for port 0"
    );
    // clap validation exits with 2
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn name_too_long_exits_one() {
    let long_name = "x".repeat(256);
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(binary_path())
        .args([
            "--extensions-dir",
            dir.path().to_str().unwrap(),
            "--name",
            &long_name,
        ])
        .output()
        .expect("failed to run binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for name too long"
    );
}

#[test]
fn jwt_key_file_nonexistent_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(binary_path())
        .args([
            "--extensions-dir",
            dir.path().to_str().unwrap(),
            "--jwt-key-file",
            "/nonexistent/key.pem",
        ])
        .output()
        .expect("failed to run binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for nonexistent jwt key file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "stderr should mention missing key file: {stderr}"
    );
}
