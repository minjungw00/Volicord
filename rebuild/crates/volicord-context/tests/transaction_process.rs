use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use volicord_context::{
    DeterministicIdGenerator, ErrorKind, FixedClock, OperationId, ProjectId, Store, TimestampMicros,
};

const MODE: &str = "VOLICORD_TRANSACTION_PROCESS_MODE";
const STORE_PATH: &str = "VOLICORD_TRANSACTION_PROCESS_STORE";
const MARKER_ROOT: &str = "VOLICORD_TRANSACTION_PROCESS_MARKERS";
const BEFORE_PROJECT_BYTE: u8 = 91;
const AFTER_PROJECT_BYTE: u8 = 92;

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

fn store(path: &Path, ids: &[[u8; 16]]) -> Result<Store, volicord_context::Error> {
    Store::open_with(
        path,
        DeterministicIdGenerator::new(ids.iter().copied()),
        FixedClock::new(TimestampMicros::from_unix_micros(1_790_000_000_000_000)),
    )
}

fn wait_for(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {}", path.display()).into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn spawn_helper(
    mode: &str,
    store_path: &Path,
    marker_root: &Path,
) -> Result<Child, Box<dyn std::error::Error>> {
    Ok(Command::new(std::env::current_exe()?)
        .arg("--exact")
        .arg("transaction_process_helper")
        .arg("--nocapture")
        .env(MODE, mode)
        .env(STORE_PATH, store_path)
        .env(MARKER_ROOT, marker_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?)
}

#[test]
fn transaction_process_helper() -> Result<(), Box<dyn std::error::Error>> {
    let Some(mode) = std::env::var_os(MODE) else {
        return Ok(());
    };
    let store_path = PathBuf::from(std::env::var_os(STORE_PATH).ok_or("missing store path")?);
    let marker_root = PathBuf::from(std::env::var_os(MARKER_ROOT).ok_or("missing marker root")?);
    match mode.to_str().ok_or("non-UTF8 process mode")? {
        "before_commit" => {
            let mut value = store(&store_path, &[[BEFORE_PROJECT_BYTE; 16]])?;
            fs::write(marker_root.join("ready"), b"ready")?;
            wait_for(&marker_root.join("go"))?;
            fs::write(marker_root.join("calling"), b"calling")?;
            value.create_project(operation(90), "Must roll back")?;
            Err("locked operation unexpectedly committed".into())
        }
        "after_commit" => {
            let mut value = store(&store_path, &[[AFTER_PROJECT_BYTE; 16]])?;
            value.create_project(operation(91), "Committed before response")?;
            fs::write(marker_root.join("committed"), b"committed")?;
            loop {
                thread::park();
            }
        }
        _ => Err("unknown process mode".into()),
    }
}

#[test]
fn hard_termination_preserves_only_committed_operations() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempdir()?;

    let before_path = root.path().join("before.sqlite3");
    drop(store(&before_path, &[])?);
    let before_markers = root.path().join("before-markers");
    fs::create_dir(&before_markers)?;
    let mut before = spawn_helper("before_commit", &before_path, &before_markers)?;
    wait_for(&before_markers.join("ready"))?;
    let lock = Connection::open(&before_path)?;
    lock.execute_batch("BEGIN EXCLUSIVE")?;
    fs::write(before_markers.join("go"), b"go")?;
    wait_for(&before_markers.join("calling"))?;
    thread::sleep(Duration::from_millis(100));
    before.kill()?;
    assert!(!before.wait()?.success());
    lock.execute_batch("ROLLBACK")?;
    drop(lock);

    let mut recovered = store(&before_path, &[[BEFORE_PROJECT_BYTE; 16]])?;
    assert_eq!(
        recovered
            .get_project(ProjectId::from_bytes([BEFORE_PROJECT_BYTE; 16]))
            .err()
            .ok_or("pre-commit termination retained a Project")?
            .kind(),
        ErrorKind::NotFound
    );
    assert!(
        !recovered
            .create_project(operation(90), "Must roll back")?
            .replayed
    );

    let after_path = root.path().join("after.sqlite3");
    let after_markers = root.path().join("after-markers");
    fs::create_dir(&after_markers)?;
    let mut after = spawn_helper("after_commit", &after_path, &after_markers)?;
    wait_for(&after_markers.join("committed"))?;
    after.kill()?;
    assert!(!after.wait()?.success());

    let mut reopened = store(&after_path, &[])?;
    assert_eq!(
        reopened
            .get_project(ProjectId::from_bytes([AFTER_PROJECT_BYTE; 16]))?
            .display_name,
        "Committed before response"
    );
    assert!(
        reopened
            .create_project(operation(91), "Committed before response")?
            .replayed
    );
    assert_eq!(
        reopened
            .create_project(operation(91), "Changed after response loss")
            .err()
            .ok_or("changed replay after response loss was accepted")?
            .kind(),
        ErrorKind::DomainConflict
    );
    Ok(())
}
