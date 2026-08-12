use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;
use volicord_context::{
    DeterministicIdGenerator, FixedClock, OperationId, ProjectId, Store, TimestampMicros,
};

const HELPER_ENV: &str = "VOLICORD_CONTEXT_PROCESS_REOPEN_HELPER";
const STORE_ENV: &str = "VOLICORD_CONTEXT_PROCESS_REOPEN_STORE";

#[test]
fn process_reopen_helper() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os(HELPER_ENV).is_none() {
        return Ok(());
    }
    let path =
        PathBuf::from(std::env::var_os(STORE_ENV).ok_or("missing explicit helper store path")?);
    let mut store = Store::open_with(
        path,
        DeterministicIdGenerator::new([[17; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_700_000_000_000_000)),
    )?;
    let result = store.create_project(OperationId::from_bytes([18; 16]), "Child process")?;
    assert_eq!(result.value.id, ProjectId::from_bytes([17; 16]));
    Ok(())
}

#[test]
fn explicit_path_survives_process_reopen_without_cwd_or_runtime_home_discovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let unrelated_cwd = root.path().join("unrelated-cwd");
    let explicit_root = root.path().join("explicit-root");
    let fake_legacy_home = root.path().join("legacy-home-must-not-be-read");
    fs::create_dir(&unrelated_cwd)?;
    fs::create_dir(&explicit_root)?;
    fs::create_dir(&fake_legacy_home)?;
    fs::write(fake_legacy_home.join("sentinel"), b"must remain untouched")?;
    fs::write(
        unrelated_cwd.join("context.sqlite3"),
        b"not the explicit store",
    )?;
    let store_path = explicit_root.join("context.sqlite3");

    let executable = std::env::current_exe()?;
    let output = Command::new(executable)
        .arg("--exact")
        .arg("process_reopen_helper")
        .arg("--nocapture")
        .current_dir(&unrelated_cwd)
        .env(HELPER_ENV, "1")
        .env(STORE_ENV, &store_path)
        .env("VOLICORD_HOME", &fake_legacy_home)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "helper failed with {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let store = Store::open_with(
        &store_path,
        DeterministicIdGenerator::new([]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_700_000_000_000_000)),
    )?;
    assert_eq!(
        store
            .get_project(ProjectId::from_bytes([17; 16]))?
            .display_name,
        "Child process"
    );
    assert_eq!(
        fs::read(fake_legacy_home.join("sentinel"))?,
        b"must remain untouched"
    );
    assert_eq!(
        fs::read(unrelated_cwd.join("context.sqlite3"))?,
        b"not the explicit store"
    );
    Ok(())
}
