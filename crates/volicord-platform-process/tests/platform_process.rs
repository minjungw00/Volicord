use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use volicord_platform_process::{
    configure_child_stdin_pipe, configure_child_stdout_pipe, read_child_stdout_available, PipeRead,
    ProcessContainment,
};

const FIXTURE_PREFIX: &str = "process_fixture-";
const FIXTURE_VERSION: &[u8] = b"volicord-platform-process-fixture-current\n";
const SCENARIO_ARGUMENT: &str = "--platform-process-fixture";
const POLL_INTERVAL: Duration = Duration::from_millis(2);
const TEST_DEADLINE: Duration = Duration::from_secs(2);

static FIXTURE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[test]
fn containment_configures_attaches_terminates_and_reaps_a_child() {
    let mut containment = ProcessContainment::new().expect("create containment");
    let mut command = fixture_command("spawn-descendant");
    containment.configure_command(&mut command);
    command.stdin(Stdio::piped()).stdout(Stdio::piped());
    let mut child = command.spawn().expect("spawn contained fixture");
    containment.attach(&child).expect("attach child");
    let mut stdin = child.stdin.take().expect("contained stdin");
    let mut stdout = child.stdout.take().expect("contained stdout");
    configure_child_stdin_pipe(&stdin).expect("configure contained stdin");
    configure_child_stdout_pipe(&stdout).expect("configure contained stdout");
    stdin.write_all(b"s").expect("signal descendant spawn");
    drop(stdin);
    assert_eq!(
        read_until_data(&mut stdout),
        b"spawned\n",
        "contained child did not report its descendant"
    );

    containment
        .terminate_tree()
        .expect("terminate process tree");
    containment
        .terminate_tree()
        .expect("repeat process-tree termination");
    let status = wait_for_exit(&mut child);
    assert!(!status.success());
    wait_for_eof(&mut stdout);
}

#[cfg(windows)]
#[test]
fn dropping_containment_closes_the_job_once_and_terminates_the_child() {
    let mut containment = ProcessContainment::new().expect("create containment");
    let mut command = fixture_command("wait-stdin");
    containment.configure_command(&mut command);
    command.stdin(Stdio::piped());
    let mut child = command.spawn().expect("spawn contained fixture");
    containment.attach(&child).expect("attach child");

    drop(containment);
    let status = wait_for_exit(&mut child);
    assert!(!status.success());
}

#[test]
fn pipe_polling_distinguishes_no_data_data_and_eof() {
    let mut waiting = fixture_command("wait-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn waiting fixture");
    let mut waiting_stdout = waiting.stdout.take().expect("waiting stdout");
    let waiting_stdin = waiting.stdin.as_ref().expect("waiting stdin");
    configure_child_stdout_pipe(&waiting_stdout).expect("configure stdout");
    configure_child_stdin_pipe(waiting_stdin).expect("configure stdin");
    assert_eq!(
        read_child_stdout_available(&mut waiting_stdout, &mut [0_u8; 16]).expect("poll empty pipe"),
        PipeRead::NoData
    );
    terminate_direct_child(&mut waiting);

    let mut writing = fixture_command("write-then-wait")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn writing fixture");
    let mut writing_stdout = writing.stdout.take().expect("writing stdout");
    configure_child_stdout_pipe(&writing_stdout).expect("configure stdout");
    let deadline = Instant::now() + TEST_DEADLINE;
    let mut buffer = [0_u8; 16];
    let count = loop {
        match read_child_stdout_available(&mut writing_stdout, &mut buffer).expect("poll data pipe")
        {
            PipeRead::Data(count) => break count,
            PipeRead::NoData if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            other => panic!("expected pipe data before deadline, got {other:?}"),
        }
    };
    assert_eq!(&buffer[..count], b"ready");
    terminate_direct_child(&mut writing);

    let mut closed = fixture_command("close-stdout")
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn closing fixture");
    let mut closed_stdout = closed.stdout.take().expect("closed stdout");
    configure_child_stdout_pipe(&closed_stdout).expect("configure stdout");
    assert!(closed.wait().expect("wait for closed fixture").success());
    assert_eq!(
        read_child_stdout_available(&mut closed_stdout, &mut buffer).expect("poll closed pipe"),
        PipeRead::Eof
    );
}

#[cfg(windows)]
#[test]
fn windows_broken_pipe_is_reported_as_eof() {
    let mut child = fixture_command("close-stdout")
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn closing fixture");
    let mut stdout = child.stdout.take().expect("closed stdout");
    configure_child_stdout_pipe(&stdout).expect("configure stdout");
    assert!(child.wait().expect("wait for closing fixture").success());

    assert_eq!(
        read_child_stdout_available(&mut stdout, &mut [0_u8; 16]).expect("poll broken pipe"),
        PipeRead::Eof
    );
}

fn fixture_command(scenario: &str) -> Command {
    let path = FIXTURE_PATH.get_or_init(fixture_path);
    let mut command = Command::new(path);
    command.arg(SCENARIO_ARGUMENT).arg(scenario);
    command
}

fn fixture_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("current test executable");
    let dependencies = test_executable
        .parent()
        .expect("test dependencies directory");
    let mut candidates = std::fs::read_dir(dependencies)
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
                "current platform-process fixture was not built under {}",
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

fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + TEST_DEADLINE;
    loop {
        match child.try_wait().expect("poll fixture child") {
            Some(status) => return status,
            None if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("fixture child was not reaped before the test deadline");
            }
        }
    }
}

fn read_until_data(stdout: &mut std::process::ChildStdout) -> Vec<u8> {
    let deadline = Instant::now() + TEST_DEADLINE;
    let mut buffer = [0_u8; 64];
    loop {
        match read_child_stdout_available(stdout, &mut buffer).expect("poll contained stdout") {
            PipeRead::Data(count) => return buffer[..count].to_vec(),
            PipeRead::NoData if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            other => panic!("expected contained stdout data before deadline, got {other:?}"),
        }
    }
}

fn wait_for_eof(stdout: &mut std::process::ChildStdout) {
    let deadline = Instant::now() + TEST_DEADLINE;
    let mut buffer = [0_u8; 64];
    loop {
        match read_child_stdout_available(stdout, &mut buffer).expect("poll contained EOF") {
            PipeRead::Eof => return,
            PipeRead::Data(_) | PipeRead::NoData if Instant::now() < deadline => {
                thread::sleep(POLL_INTERVAL);
            }
            other => panic!("expected contained pipe EOF before deadline, got {other:?}"),
        }
    }
}

fn terminate_direct_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
