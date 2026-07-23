use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{Duration, Instant},
};

use volicord_test_process::{
    BoundedCommand, BoundedProcessFailureKind, ProcessDeadline, ProcessPipe,
};

const FIXTURE_PREFIX: &str = "test_process_fixture-";
const FIXTURE_VERSION: &[u8] = b"volicord-test-process-fixture-current\n";
const SCENARIO_ARGUMENT: &str = "--test-process-fixture";
const NORMAL_DEADLINE: ProcessDeadline =
    ProcessDeadline::new(Duration::from_secs(3), Duration::from_secs(2));
const TIMEOUT_DEADLINE: ProcessDeadline =
    ProcessDeadline::new(Duration::from_millis(100), Duration::from_secs(2));
const NORMAL_CAPTURE_LIMIT: usize = 1024 * 1024;

static FIXTURE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[test]
fn successful_command_captures_stdout_and_stderr() {
    let output = fixture("stdout-stderr")
        .require_success(true)
        .run()
        .expect("successful fixture");
    assert!(output.status().success());
    assert_eq!(output.stdout().bytes(), b"fixture stdout\n");
    assert_eq!(output.stderr().bytes(), b"fixture stderr\n");
    assert!(!output.stdout().is_truncated());
    assert!(!output.stderr().is_truncated());
}

#[test]
fn stdin_is_delivered_completely() {
    let input = b"stdin bytes across the bounded process boundary\n".to_vec();
    let output = fixture("echo-stdin")
        .stdin(input.clone())
        .require_success(true)
        .run()
        .expect("stdin fixture");
    assert_eq!(output.stdout().bytes(), input);
}

#[test]
fn nonzero_exit_is_retained_or_classified_as_requested() {
    let output = fixture("exit-23").run().expect("nonzero status allowed");
    assert_eq!(output.status().code(), Some(23));

    let failure = fixture("exit-23")
        .require_success(true)
        .run()
        .expect_err("required success must reject nonzero status");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::UnsuccessfulExit);
    assert_eq!(failure.status().and_then(|status| status.code()), Some(23));
}

#[test]
fn timeout_is_bounded_and_reaps_the_child() {
    let started = Instant::now();
    let failure = fixture_with_deadline("hang", TIMEOUT_DEADLINE)
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

#[test]
fn stdout_truncation_retains_the_prefix_and_omitted_count() {
    let output = BoundedCommand::new(fixture_path(), NORMAL_DEADLINE, 16, 1024)
        .arg(SCENARIO_ARGUMENT)
        .arg("stdout-bytes")
        .arg("100")
        .require_success(true)
        .run()
        .expect("stdout truncation fixture");
    assert_eq!(output.stdout().bytes(), &[b'o'; 16]);
    assert_eq!(output.stdout().omitted_bytes(), 84);
    assert!(output.stdout().is_truncated());
}

#[test]
fn stderr_truncation_retains_the_prefix_and_omitted_count() {
    let output = BoundedCommand::new(fixture_path(), NORMAL_DEADLINE, 1024, 13)
        .arg(SCENARIO_ARGUMENT)
        .arg("stderr-bytes")
        .arg("80")
        .require_success(true)
        .run()
        .expect("stderr truncation fixture");
    assert_eq!(output.stderr().bytes(), &[b'e'; 13]);
    assert_eq!(output.stderr().omitted_bytes(), 67);
    assert!(output.stderr().is_truncated());
}

#[test]
fn sustained_stderr_does_not_block_active_stdout() {
    let output = fixture("sustained-stderr")
        .require_success(true)
        .run()
        .expect("concurrent stdout and stderr fixture");
    assert_eq!(output.stdout().bytes(), b"stdout remained active\n");
    assert!(output.stderr().bytes().ends_with(b"stderr complete\n"));
}

#[test]
fn descendant_retaining_pipes_times_out_and_is_cleaned_up() {
    let failure = fixture_with_deadline("descendant-retains-pipes", TIMEOUT_DEADLINE)
        .run()
        .expect_err("pipe-holding descendant must consume the lifecycle deadline");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::Timeout);
    assert!(
        failure.cleanup_detail().is_none(),
        "{:?}",
        failure.cleanup_detail()
    );
    assert!(failure.status().is_some(), "direct child was not reaped");
    assert!(failure.elapsed() < Duration::from_secs(5));
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn unix_process_group_containment_terminates_the_pipe_holding_descendant() {
    use rustix::{
        io::Errno,
        process::{test_kill_process, Pid},
    };

    let failure = fixture_with_deadline("descendant-retains-pipes", TIMEOUT_DEADLINE)
        .run()
        .expect_err("pipe-holding descendant must time out");
    let stdout = std::str::from_utf8(failure.stdout().bytes()).expect("fixture stdout");
    let raw_pid = stdout
        .trim()
        .strip_prefix("descendant=")
        .expect("descendant marker")
        .parse::<i32>()
        .expect("descendant PID");
    let pid = Pid::from_raw(raw_pid).expect("nonzero descendant PID");
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}

#[cfg(windows)]
#[test]
fn native_windows_job_object_containment_closes_descendant_pipes() {
    let failure = fixture_with_deadline("descendant-retains-pipes", TIMEOUT_DEADLINE)
        .run()
        .expect_err("pipe-holding descendant must time out");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::Timeout);
    assert!(failure.cleanup_detail().is_none());
    assert!(failure.stdout().bytes().starts_with(b"descendant="));
}

#[test]
fn stdin_write_failure_still_reaps_and_cleans_up() {
    let failure = fixture("exit-immediately")
        .stdin(vec![b'x'; 1024 * 1024])
        .run()
        .expect_err("closed child stdin must fail delivery");
    assert_eq!(failure.kind(), BoundedProcessFailureKind::StdinWrite);
    assert_eq!(failure.pipe(), Some(ProcessPipe::Stdin));
    assert!(
        failure.status().is_some(),
        "failed writer child was not reaped"
    );
    assert!(
        failure.cleanup_detail().is_none(),
        "{:?}",
        failure.cleanup_detail()
    );
}

#[test]
fn paths_and_arguments_containing_spaces_are_preserved() {
    let temporary = tempfile::Builder::new()
        .prefix("volicord test process ")
        .tempdir()
        .expect("temporary directory");
    let directory = temporary.path().join("working directory with spaces");
    fs::create_dir(&directory).expect("create working directory");
    let copied_fixture = temporary.path().join(if cfg!(windows) {
        "fixture executable with spaces.exe"
    } else {
        "fixture executable with spaces"
    });
    fs::copy(fixture_path(), &copied_fixture).expect("copy fixture");
    let output = BoundedCommand::new(
        &copied_fixture,
        NORMAL_DEADLINE,
        NORMAL_CAPTURE_LIMIT,
        NORMAL_CAPTURE_LIMIT,
    )
    .arg(SCENARIO_ARGUMENT)
    .arg("paths-and-arguments")
    .arg("argument value with spaces")
    .current_dir(&directory)
    .require_success(true)
    .run()
    .expect("spaced path fixture");
    let stdout = String::from_utf8(output.stdout().bytes().to_vec()).expect("fixture UTF-8");
    assert!(stdout.contains(&format!("cwd={}", directory.display())));
    assert!(stdout.contains("arg=argument value with spaces"));
}

#[test]
fn environment_addition_and_removal_are_explicit() {
    let output = fixture("environment")
        .env("VOLICORD_TEST_PROCESS_ADDED", "added value")
        .env("VOLICORD_TEST_PROCESS_REMOVED", "present")
        .env_remove("VOLICORD_TEST_PROCESS_REMOVED")
        .require_success(true)
        .run()
        .expect("environment fixture");
    assert_eq!(
        output.stdout().bytes(),
        b"added=added value\nremoved=true\n"
    );
}

fn fixture(scenario: &str) -> BoundedCommand {
    fixture_with_deadline(scenario, NORMAL_DEADLINE)
}

fn fixture_with_deadline(scenario: &str, deadline: ProcessDeadline) -> BoundedCommand {
    BoundedCommand::new(
        fixture_path(),
        deadline,
        NORMAL_CAPTURE_LIMIT,
        NORMAL_CAPTURE_LIMIT,
    )
    .arg(SCENARIO_ARGUMENT)
    .arg(scenario)
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
                .arg(SCENARIO_ARGUMENT)
                .arg("version")
                .output()
                .is_ok_and(|output| output.status.success() && output.stdout == FIXTURE_VERSION)
        })
        .unwrap_or_else(|| {
            panic!(
                "current test-process fixture was not built under {}",
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
