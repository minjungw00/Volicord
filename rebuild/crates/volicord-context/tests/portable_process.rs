use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;
use volicord_context::{
    DeterministicIdGenerator, ErrorKind, FixedClock, OperationId, Store, TimestampMicros,
};

const MODE: &str = "VOLICORD_PORTABLE_PROCESS_MODE";
const STORE_PATH: &str = "VOLICORD_PORTABLE_PROCESS_STORE";
const BUNDLE_PATH: &str = "VOLICORD_PORTABLE_PROCESS_BUNDLE";
const PROJECT_BYTE: u8 = 71;

fn operation(value: u8) -> OperationId {
    OperationId::from_bytes([value; 16])
}

#[test]
fn portable_process_helper() -> Result<(), Box<dyn std::error::Error>> {
    let Some(mode) = std::env::var_os(MODE) else {
        return Ok(());
    };
    let store_path = PathBuf::from(std::env::var_os(STORE_PATH).ok_or("missing store path")?);
    let bundle_path = PathBuf::from(std::env::var_os(BUNDLE_PATH).ok_or("missing bundle path")?);
    let mut store = Store::open(&store_path)?;
    match mode.to_str().ok_or("non-UTF8 mode")? {
        "publication" => {
            let error = store
                .export_bundle(
                    volicord_context::ProjectId::from_bytes([PROJECT_BYTE; 16]),
                    bundle_path,
                )
                .err()
                .ok_or("publication fault did not occur")?;
            assert_eq!(error.kind(), ErrorKind::StorageUnavailable);
        }
        "import" => {
            let error = store
                .import_bundle(operation(90), bundle_path)
                .err()
                .ok_or("import fault did not occur")?;
            assert_eq!(error.kind(), ErrorKind::TransactionFailure);
        }
        _ => return Err("unknown helper mode".into()),
    }
    Ok(())
}

fn run_helper(
    mode: &str,
    store: &PathBuf,
    bundle: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(std::env::current_exe()?)
        .arg("--exact")
        .arg("portable_process_helper")
        .arg("--nocapture")
        .env(MODE, mode)
        .env(STORE_PATH, store)
        .env(BUNDLE_PATH, bundle)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "portable helper failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

#[test]
fn process_faults_preserve_published_bundle_and_prior_import_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempdir()?;
    let source_path = root.path().join("source.sqlite3");
    let bundle_path = root.path().join("context.json");
    let mut source = Store::open_with(
        &source_path,
        DeterministicIdGenerator::new([[PROJECT_BYTE; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_770_000_000_000_000)),
    )?;
    let project = source
        .create_project(operation(1), "Process portable")?
        .value;
    source.export_bundle(project.id, &bundle_path)?;
    let published = fs::read(&bundle_path)?;
    let temporary = root.path().join(".context.json.volicord-context.tmp");
    fs::create_dir(&temporary)?;
    drop(source);
    run_helper("publication", &source_path, &bundle_path)?;
    assert_eq!(fs::read(&bundle_path)?, published);
    fs::remove_dir(&temporary)?;

    let destination_path = root.path().join("destination.sqlite3");
    let mut destination = Store::open_with(
        &destination_path,
        DeterministicIdGenerator::new([[72; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(1_770_000_000_000_000)),
    )?;
    let prior = destination
        .create_project(operation(2), "Prior process state")?
        .value;
    drop(destination);
    let connection = Connection::open(&destination_path)?;
    connection.execute_batch(
        "CREATE TRIGGER interrupt_bundle_import BEFORE INSERT ON projects
         BEGIN SELECT RAISE(ABORT, 'process interruption'); END;",
    )?;
    drop(connection);
    run_helper("import", &destination_path, &bundle_path)?;
    let destination = Store::open(&destination_path)?;
    assert_eq!(
        destination
            .get_project(project.id)
            .err()
            .ok_or("interrupted process import mutated state")?
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        destination.get_project(prior.id)?.display_name,
        "Prior process state"
    );
    Ok(())
}
