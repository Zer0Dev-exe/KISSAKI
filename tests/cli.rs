use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kissaki"));
    command.stdin(Stdio::null());
    command
}

struct InputFixture(PathBuf);

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

impl InputFixture {
    fn new() -> Result<Self> {
        Self::at(SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos())
    }

    fn at(timestamp: u128) -> Result<Self> {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "kissaki-input-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

#[test]
fn fixtures_with_identical_clock_ticks_remain_isolated() -> Result<()> {
    let first = InputFixture::at(0)?;
    let second = InputFixture::at(0)?;
    assert_ne!(first.0, second.0);
    fs::write(first.path("first.txt"), "first fixture")?;
    assert!(!second.path("first.txt").exists());
    drop(first);
    assert!(second.0.is_dir());
    Ok(())
}

impl Drop for InputFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn invalid_seeds_do_not_create_output_files() -> Result<()> {
    let fixture = InputFixture::new()?;
    let output_path = fixture.path("output.txt");
    for arguments in [
        vec![],
        vec!["-u", "http://127.0.0.1/"],
        vec![
            "-u",
            "https://example.com/",
            "-u",
            "https://example.com/other",
            "--max-pages",
            "1",
        ],
    ] {
        let output = command()
            .args(arguments)
            .arg("-o")
            .arg(&output_path)
            .output()?;
        assert!(!output.status.success());
        assert!(!output_path.exists());
    }
    Ok(())
}

#[test]
fn list_files_share_one_input_byte_budget() -> Result<()> {
    let fixture = InputFixture::new()?;
    let first = fixture.path("first.txt");
    let second = fixture.path("second.txt");
    let output_path = fixture.path("output.txt");
    fs::write(
        &first,
        format!("#{}\nhttps://example.com/\n", " ".repeat(600_000)),
    )?;
    fs::write(&second, format!("#{}\n", " ".repeat(600_000)))?;
    let output = command()
        .arg("-l")
        .arg(first)
        .arg("-l")
        .arg(second)
        .arg("-o")
        .arg(&output_path)
        .output()?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("combined target input exceeds 1 MiB")
    );
    assert!(!output_path.exists());
    Ok(())
}

#[test]
fn stdin_and_arguments_share_the_seed_count_budget() -> Result<()> {
    let mut child = command()
        .args(["-u", "https://example.com/", "--max-pages", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(b"https://example.com/second\n")?;
    }
    let output = child.wait_with_output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("seed count exceeds max-pages"));
    assert!(output.stdout.is_empty());
    Ok(())
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
