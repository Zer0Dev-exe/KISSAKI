use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::Result;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kissaki"));
    command.stdin(Stdio::null());
    command
}

#[test]
fn help_and_version_succeed() -> Result<()> {
    let help = command().arg("--help").output()?;
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--allow-private"));
    let version = command().arg("--version").output()?;
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains(env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn rejects_invalid_input_and_private_targets() -> Result<()> {
    for arguments in [
        vec![],
        vec!["-u", "file:///etc/passwd"],
        vec!["-u", "http://127.0.0.1/"],
        vec!["-u", "https://example.com/", "--concurrency", "0"],
        vec!["-u", "https://example.com/", "--include", "["],
        vec!["-u", "https://example.com/", "-H", "Host: attacker.invalid"],
    ] {
        let output = command().args(arguments).output()?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn reads_stdin_and_does_not_overwrite_files() -> Result<()> {
    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(b"# comment\n\nfile:///etc/passwd\n")?;
    }
    let output = child.wait_with_output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("HTTP(S)"));
    let output = command()
        .args(["-u", "https://example.com/", "-o", "Cargo.toml"])
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("creating Cargo.toml"));
    Ok(())
}
