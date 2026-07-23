use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};

use volicord_release_smoke::{codex_executable_name, run_release_smoke, CODEX_FIXTURE_VERSION};
use volicord_test_process::{BoundedCommand, BoundedProcessFailureKind, ProcessDeadline};
use volicord_types::AgentToolId;

const FIXTURE_PREFIX: &str = "release_smoke_fixture-";
const FIXTURE_ARGUMENT: &str = "--release-smoke-test-fixture";
const FIXTURE_VERSION: &[u8] = b"volicord-release-smoke-test-fixture-current\n";
const CAPTURE_LIMIT: usize = 1024 * 1024;
const NORMAL_DEADLINE: ProcessDeadline =
    ProcessDeadline::new(Duration::from_secs(5), Duration::from_secs(2));
const TIMEOUT_DEADLINE: ProcessDeadline =
    ProcessDeadline::new(Duration::from_millis(100), Duration::from_secs(2));

static FIXTURE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[test]
fn successful_flow_exercises_the_supplied_binary() {
    let report = run_release_smoke(fixture_path(), release_smoke_binary())
        .expect("successful release smoke fixture");
    assert_eq!(
        report.binary(),
        fs::canonicalize(fixture_path())
            .expect("canonical fixture path")
            .as_path()
    );
    assert!(!report.protocol_revision().is_empty());
    assert_eq!(report.tool_count(), AgentToolId::ALL.len());
}

#[test]
fn missing_supplied_binary_is_rejected() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let error = run_release_smoke(
        &temporary.path().join("missing-volicord"),
        release_smoke_binary(),
    )
    .expect_err("missing binary must fail");
    assert!(error.to_string().contains("release binary does not exist"));
}

#[test]
fn unlaunchable_supplied_binary_is_rejected() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let binary = temporary.path().join(if cfg!(windows) {
        "not-volicord.exe"
    } else {
        "not-volicord"
    });
    fs::write(&binary, b"not an executable").expect("write unlaunchable fixture");
    let error = run_release_smoke(&binary, release_smoke_binary())
        .expect_err("unlaunchable binary must fail");
    assert!(error
        .to_string()
        .contains("failed to launch release binary --help"));
}

#[test]
fn copied_codex_fixture_reports_its_stable_version() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let codex = copy_codex_fixture(temporary.path());
    let output = bounded_fixture_command(&codex)
        .arg("--version")
        .require_success(true)
        .run()
        .expect("codex fixture version");
    assert_eq!(
        output.stdout().bytes(),
        format!("{CODEX_FIXTURE_VERSION}\n").as_bytes()
    );
    assert!(output.stderr().bytes().is_empty());
}

#[test]
fn copied_codex_fixture_rejects_unsupported_invocations() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let codex = copy_codex_fixture(temporary.path());
    let failure = bounded_fixture_command(&codex)
        .arg("unsupported")
        .require_success(true)
        .run()
        .expect_err("unsupported Codex fixture invocation must fail");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::UnsuccessfulExit);
    assert!(failure
        .stderr()
        .render_lossy()
        .contains("supports only --version"));
}

#[test]
fn bounded_timeout_terminates_and_reaps_the_fixture_process() {
    let started = Instant::now();
    let failure = BoundedCommand::new(
        fixture_path(),
        TIMEOUT_DEADLINE,
        CAPTURE_LIMIT,
        CAPTURE_LIMIT,
    )
    .arg(FIXTURE_ARGUMENT)
    .arg("hang")
    .run()
    .expect_err("hanging fixture must time out");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::Timeout);
    assert!(failure.status().is_some(), "timed out child was not reaped");
    assert!(
        failure.cleanup_detail().is_none(),
        "{:?}",
        failure.cleanup_detail()
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

fn release_smoke_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_volicord-release-smoke"))
}

fn bounded_fixture_command(program: &Path) -> BoundedCommand {
    BoundedCommand::new(program, NORMAL_DEADLINE, CAPTURE_LIMIT, CAPTURE_LIMIT)
}

fn copy_codex_fixture(directory: &Path) -> PathBuf {
    let destination = directory.join(codex_executable_name());
    fs::copy(release_smoke_binary(), &destination).expect("copy Codex fixture executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&destination)
            .expect("Codex fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&destination, permissions).expect("Codex fixture permissions");
    }
    destination
}

fn fixture_path() -> &'static Path {
    FIXTURE_PATH.get_or_init(find_fixture_path)
}

fn find_fixture_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    let dependencies = test_executable
        .parent()
        .expect("test dependencies directory");
    let mut candidates = fs::read_dir(dependencies)
        .expect("read test dependencies directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_fixture_candidate(path))
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .find(|path| {
            Command::new(path)
                .arg(FIXTURE_ARGUMENT)
                .arg("version")
                .output()
                .is_ok_and(|output| output.status.success() && output.stdout == FIXTURE_VERSION)
        })
        .unwrap_or_else(|| {
            panic!(
                "release-smoke fixture was not built under {}",
                dependencies.display()
            )
        })
}

fn is_fixture_candidate(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(hash_and_suffix) = name.strip_prefix(FIXTURE_PREFIX) else {
        return false;
    };
    let hash = hash_and_suffix
        .strip_suffix(std::env::consts::EXE_SUFFIX)
        .unwrap_or(hash_and_suffix);
    !hash.is_empty() && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
