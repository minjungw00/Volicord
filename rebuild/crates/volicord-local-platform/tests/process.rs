use std::{fs, thread, time::Duration};

use tempfile::tempdir;
use volicord_local_platform::{
    CancellationFlag, ProcessCompletion, ProcessRequest, ProcessStopTrigger,
    ProcessStreamCompleteness, ProcessTermination, ProcessTreeCleanup,
};

fn request(directory: &std::path::Path, script: &str, timeout: Duration) -> ProcessRequest {
    ProcessRequest::new(
        "/bin/sh",
        directory.join("stdout.log"),
        directory.join("stderr.log"),
        timeout,
        Duration::from_secs(2),
    )
    .args(["-c", script])
}

#[test]
fn bounded_stdin_is_delivered_without_argv_or_input_artifact() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let result = request(
        temporary.path(),
        "read value; printf 'received:%s' \"$value\"",
        Duration::from_secs(2),
    )
    .stdin_bytes(b"fixture-source\n".to_vec())
    .run()
    .expect("process observation");

    assert!(result.succeeded());
    assert_eq!(
        fs::read_to_string(result.stdout().path()).expect("stdout"),
        "received:fixture-source"
    );
    assert_eq!(
        fs::read_dir(temporary.path())
            .expect("artifact directory")
            .count(),
        2
    );
}

#[test]
fn complete_streams_nonzero_exit_and_duration_are_preserved() {
    let temporary = tempdir().expect("temporary directory");
    let result = request(
        temporary.path(),
        "printf 'complete stdout'; printf 'complete stderr' >&2; exit 23",
        Duration::from_secs(2),
    )
    .run()
    .expect("process observation");
    assert_eq!(
        result.completion(),
        &ProcessCompletion::Exited(ProcessTermination::ExitCode(23))
    );
    assert!(!result.succeeded());
    assert_eq!(result.stop_trigger(), None);
    assert_eq!(
        fs::read(result.stdout().path()).expect("stdout"),
        b"complete stdout"
    );
    assert_eq!(
        fs::read(result.stderr().path()).expect("stderr"),
        b"complete stderr"
    );
    assert_eq!(
        result.stdout().completeness(),
        ProcessStreamCompleteness::Complete
    );
    assert_eq!(
        result.stderr().completeness(),
        ProcessStreamCompleteness::Complete
    );
    assert!(!result.duration().is_zero());
}

#[test]
fn timeout_records_signal_and_confirmed_descendant_cleanup_separately() {
    let temporary = tempdir().expect("temporary directory");
    let result = request(
        temporary.path(),
        "sleep 30 & child=$!; printf 'descendant=%s\\n' \"$child\"; wait",
        Duration::from_millis(100),
    )
    .run()
    .expect("timeout observation");
    assert_eq!(result.stop_trigger(), Some(ProcessStopTrigger::Timeout));
    assert_eq!(
        result.completion(),
        &ProcessCompletion::Exited(ProcessTermination::Signal(9))
    );
    assert_eq!(result.cleanup(), &ProcessTreeCleanup::Confirmed);
    assert_eq!(
        result.stdout().completeness(),
        ProcessStreamCompleteness::Complete
    );
    assert!(fs::read_to_string(result.stdout().path())
        .expect("stdout")
        .starts_with("descendant="));
}

#[test]
fn cancellation_is_not_reported_as_timeout_or_success() {
    let temporary = tempdir().expect("temporary directory");
    let cancellation = CancellationFlag::default();
    let requester = cancellation.clone();
    let thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        requester.request();
    });
    let result = request(
        temporary.path(),
        "printf ready; while :; do sleep 1; done",
        Duration::from_secs(5),
    )
    .cancellation(cancellation)
    .run()
    .expect("cancellation observation");
    thread.join().expect("cancellation thread");
    assert_eq!(
        result.stop_trigger(),
        Some(ProcessStopTrigger::Cancellation)
    );
    assert_eq!(
        result.completion(),
        &ProcessCompletion::Exited(ProcessTermination::Signal(9))
    );
    assert_eq!(result.cleanup(), &ProcessTreeCleanup::Confirmed);
    assert!(!result.succeeded());
}

#[test]
fn artifact_collision_fails_before_spawn() {
    let temporary = tempdir().expect("temporary directory");
    fs::write(temporary.path().join("stdout.log"), b"preserve").expect("sentinel");
    let error = request(temporary.path(), "exit 0", Duration::from_secs(1))
        .run()
        .expect_err("existing artifact must fail");
    assert!(error.detail().contains("stdout artifact"));
    assert_eq!(
        fs::read(temporary.path().join("stdout.log")).expect("sentinel"),
        b"preserve"
    );
}
