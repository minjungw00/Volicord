#![forbid(unsafe_code)]

use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

use tempfile::tempdir;
use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};
use volicord_platform_process::{
    configure_child_stderr_pipe, configure_child_stdout_pipe, read_child_stderr_available,
    read_child_stdout_available, PipeRead,
};

const FIXTURE_PREFIX: &str = "mutation_lease_fixture-";
const FIXTURE_PROBE: &[u8] = b"volicord-mutation-lease-fixture-current\n";
const SCENARIO_ARGUMENT: &str = "--mutation-lease-fixture";
const READY: &[u8] = b"mutation-lease-ready";
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(2);
const CAPTURE_LIMIT: usize = 64 * 1024;

static FIXTURE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
enum ChildExit {
    Normal,
    Error,
    Panic,
    Terminated,
}

impl ChildExit {
    const ALL: [Self; 4] = [Self::Normal, Self::Error, Self::Panic, Self::Terminated];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Error => "error",
            Self::Panic => "panic",
            Self::Terminated => "terminated",
        }
    }
}

fn acquire(
    path: &Path,
    mode: RuntimeHomeMutationLeaseMode,
    wait_policy: RuntimeHomeMutationWaitPolicy,
) -> Result<RuntimeHomeMutationLeaseOutcome, Box<dyn Error>> {
    Ok(RuntimeHomeMutationLease::acquire(path, mode, wait_policy)?)
}

struct LeaseChild {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    stdout_configured: bool,
    stderr_configured: bool,
    status: Option<ExitStatus>,
    stdout_capture: Vec<u8>,
    stderr_capture: Vec<u8>,
    stdout_omitted: usize,
    stderr_omitted: usize,
    finalized: bool,
}

impl LeaseChild {
    fn spawn(
        target: &Path,
        mode: RuntimeHomeMutationLeaseMode,
        exit: ChildExit,
    ) -> Result<Self, Box<dyn Error>> {
        let child = Command::new(fixture_path())
            .arg(SCENARIO_ARGUMENT)
            .arg("hold")
            .arg(target)
            .arg(mode.as_str())
            .arg(exit.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut lease_child = Self {
            child: Some(child),
            stdin: None,
            stdout: None,
            stderr: None,
            stdout_configured: false,
            stderr_configured: false,
            status: None,
            stdout_capture: Vec::new(),
            stderr_capture: Vec::new(),
            stdout_omitted: 0,
            stderr_omitted: 0,
            finalized: false,
        };
        let child = lease_child
            .child
            .as_mut()
            .expect("newly spawned fixture child");
        lease_child.stdin = child.stdin.take();
        lease_child.stdout = child.stdout.take();
        lease_child.stderr = child.stderr.take();
        if lease_child.stdin.is_none()
            || lease_child.stdout.is_none()
            || lease_child.stderr.is_none()
        {
            let diagnostics = lease_child.terminate_and_capture();
            return Err(format!(
                "mutation-lease fixture did not expose every requested pipe\n{diagnostics}"
            )
            .into());
        }
        if let Err(error) = lease_child.configure_pipes() {
            let diagnostics = lease_child.terminate_and_capture();
            return Err(format!("{error}\n{diagnostics}").into());
        }
        if let Err(error) = lease_child.wait_until_ready() {
            let diagnostics = lease_child.terminate_and_capture();
            return Err(format!("{error}\n{diagnostics}").into());
        }
        Ok(lease_child)
    }

    fn configure_pipes(&mut self) -> io::Result<()> {
        configure_child_stdout_pipe(
            self.stdout
                .as_ref()
                .ok_or_else(|| io::Error::other("mutation-lease fixture stdout is missing"))?,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
        self.stdout_configured = true;
        configure_child_stderr_pipe(
            self.stderr
                .as_ref()
                .ok_or_else(|| io::Error::other("mutation-lease fixture stderr is missing"))?,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
        self.stderr_configured = true;
        Ok(())
    }

    fn wait_until_ready(&mut self) -> io::Result<()> {
        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            self.pump_output()?;
            if let Some(line_end) = self.stdout_capture.iter().position(|byte| *byte == b'\n') {
                let line = self.stdout_capture[..line_end]
                    .strip_suffix(b"\r")
                    .unwrap_or(&self.stdout_capture[..line_end]);
                if line == READY {
                    return Ok(());
                }
                return Err(io::Error::other(format!(
                    "mutation-lease fixture emitted an unexpected readiness line: {:?}",
                    String::from_utf8_lossy(line)
                )));
            }
            if let Some(status) = self.poll_status()? {
                return Err(io::Error::other(format!(
                    "mutation-lease fixture exited before readiness with status {status}"
                )));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "mutation-lease fixture did not signal readiness within {READY_TIMEOUT:?}"
                    ),
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn finish(mut self, exit: ChildExit) -> Result<(), Box<dyn Error>> {
        self.stdin.take();
        if matches!(exit, ChildExit::Terminated) {
            self.kill()?;
        }
        let status = match self.wait_for_completion(EXIT_TIMEOUT) {
            Ok(status) => status,
            Err(error) => {
                let diagnostics = self.terminate_and_capture();
                return Err(format!("{error}\n{diagnostics}").into());
            }
        };
        let diagnostics = self.diagnostics(status);
        match exit {
            ChildExit::Normal if !status.success() => {
                return Err(format!("normal fixture exit failed\n{diagnostics}").into())
            }
            ChildExit::Error | ChildExit::Panic | ChildExit::Terminated if status.success() => {
                return Err(format!(
                    "{} fixture exit unexpectedly succeeded\n{diagnostics}",
                    exit.as_str()
                )
                .into())
            }
            _ => {}
        }
        match exit {
            ChildExit::Error => assert!(
                self.stderr_capture
                    .windows(b"injected fixture error".len())
                    .any(|window| window == b"injected fixture error"),
                "error fixture stderr was not captured\n{diagnostics}"
            ),
            ChildExit::Panic => assert!(
                self.stderr_capture
                    .windows(b"injected fixture panic".len())
                    .any(|window| window == b"injected fixture panic"),
                "panic fixture stderr was not captured\n{diagnostics}"
            ),
            ChildExit::Normal | ChildExit::Terminated => {}
        }
        self.finalized = true;
        Ok(())
    }

    fn wait_for_completion(&mut self, timeout: Duration) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            self.pump_output()?;
            if let Some(status) = self.poll_status()? {
                if self.stdout.is_none() && self.stderr.is_none() {
                    return Ok(status);
                }
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("mutation-lease fixture did not finish within {timeout:?}"),
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn pump_output(&mut self) -> io::Result<()> {
        if self.stdout_configured {
            pump_pipe(
                &mut self.stdout,
                &mut self.stdout_capture,
                &mut self.stdout_omitted,
            )?;
        }
        if self.stderr_configured {
            pump_pipe(
                &mut self.stderr,
                &mut self.stderr_capture,
                &mut self.stderr_omitted,
            )?;
        }
        Ok(())
    }

    fn poll_status(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        if let Some(status) = child.try_wait()? {
            self.status = Some(status);
            self.child.take();
            return Ok(Some(status));
        }
        Ok(None)
    }

    fn kill(&mut self) -> io::Result<()> {
        if let Some(child) = self.child.as_mut() {
            child.kill()?;
        }
        Ok(())
    }

    fn terminate_and_capture(&mut self) -> String {
        self.stdin.take();
        if !self.stdout_configured {
            self.stdout.take();
        }
        if !self.stderr_configured {
            self.stderr.take();
        }
        let kill_error = self.kill().err();
        let completion_error = self.wait_for_completion(CLEANUP_TIMEOUT).err();
        let status = self.status;
        if status.is_some() && self.stdout.is_none() && self.stderr.is_none() {
            self.finalized = true;
        }
        format!(
            "cleanup kill error: {}\ncleanup completion error: {}\n{}",
            kill_error.map_or_else(|| "none".to_owned(), |error| error.to_string()),
            completion_error.map_or_else(|| "none".to_owned(), |error| error.to_string()),
            self.diagnostics_optional(status)
        )
    }

    fn diagnostics(&self, status: ExitStatus) -> String {
        self.diagnostics_optional(Some(status))
    }

    fn diagnostics_optional(&self, status: Option<ExitStatus>) -> String {
        format!(
            "status: {}\nstdout:\n{}{}\nstderr:\n{}{}",
            status.map_or_else(|| "not reaped".to_owned(), |status| status.to_string()),
            String::from_utf8_lossy(&self.stdout_capture),
            omitted_suffix(self.stdout_omitted),
            String::from_utf8_lossy(&self.stderr_capture),
            omitted_suffix(self.stderr_omitted),
        )
    }
}

impl Drop for LeaseChild {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.terminate_and_capture();
        }
    }
}

trait ChildPipe {
    fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<PipeRead>;
}

impl ChildPipe for ChildStdout {
    fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<PipeRead> {
        read_child_stdout_available(self, buffer)
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

impl ChildPipe for ChildStderr {
    fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<PipeRead> {
        read_child_stderr_available(self, buffer)
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

fn pump_pipe<P: ChildPipe>(
    pipe: &mut Option<P>,
    capture: &mut Vec<u8>,
    omitted: &mut usize,
) -> io::Result<()> {
    let mut buffer = [0_u8; 4 * 1024];
    loop {
        let Some(open_pipe) = pipe.as_mut() else {
            return Ok(());
        };
        match open_pipe.read_available(&mut buffer)? {
            PipeRead::Data(count) => append_bounded(capture, omitted, &buffer[..count]),
            PipeRead::NoData => return Ok(()),
            PipeRead::Eof => {
                pipe.take();
                return Ok(());
            }
        }
    }
}

fn append_bounded(capture: &mut Vec<u8>, omitted: &mut usize, bytes: &[u8]) {
    let retained = CAPTURE_LIMIT.saturating_sub(capture.len()).min(bytes.len());
    capture.extend_from_slice(&bytes[..retained]);
    *omitted = omitted.saturating_add(bytes.len().saturating_sub(retained));
}

fn omitted_suffix(omitted: usize) -> String {
    if omitted == 0 {
        String::new()
    } else {
        format!("\n[{omitted} additional byte(s) omitted]")
    }
}

fn fixture_path() -> &'static Path {
    FIXTURE_PATH.get_or_init(|| {
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
                    .arg("probe")
                    .output()
                    .is_ok_and(|output| output.status.success() && output.stdout == FIXTURE_PROBE)
            })
            .unwrap_or_else(|| {
                panic!(
                    "current mutation-lease fixture was not built under {}",
                    dependencies.display()
                )
            })
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

fn expected_admission(
    held: RuntimeHomeMutationLeaseMode,
    requested: RuntimeHomeMutationLeaseMode,
) -> bool {
    held == RuntimeHomeMutationLeaseMode::SharedWriter
        && requested == RuntimeHomeMutationLeaseMode::SharedWriter
}

#[test]
fn cross_process_mutation_lease_protocol() -> Result<(), Box<dyn Error>> {
    let modes = [
        RuntimeHomeMutationLeaseMode::SharedWriter,
        RuntimeHomeMutationLeaseMode::ExclusiveSetup,
    ];
    for held in modes {
        for requested in modes {
            for wait_policy in [
                RuntimeHomeMutationWaitPolicy::Immediate,
                RuntimeHomeMutationWaitPolicy::Bounded {
                    timeout: Duration::from_millis(25),
                },
            ] {
                let fixture = tempdir()?;
                let target = fixture.path().join("runtime-home");
                let child = LeaseChild::spawn(&target, held, ChildExit::Normal)?;
                let outcome = acquire(&target, requested, wait_policy)?;

                assert_eq!(
                    matches!(outcome, RuntimeHomeMutationLeaseOutcome::Acquired(_)),
                    expected_admission(held, requested),
                    "held={held:?}, requested={requested:?}, wait_policy={wait_policy:?}"
                );
                if let RuntimeHomeMutationLeaseOutcome::Busy(busy) = &outcome {
                    assert_eq!(busy.requested_mode(), requested);
                    assert_eq!(busy.wait_policy(), wait_policy);
                }
                drop(outcome);
                child.finish(ChildExit::Normal)?;
                assert!(matches!(
                    acquire(&target, requested, RuntimeHomeMutationWaitPolicy::Immediate)?,
                    RuntimeHomeMutationLeaseOutcome::Acquired(_)
                ));
                fixture.close()?;
            }
        }
    }

    for mode in modes {
        for exit in ChildExit::ALL {
            let fixture = tempdir()?;
            let target = fixture.path().join("runtime-home");
            let child = LeaseChild::spawn(&target, mode, exit)?;
            let conflicting = match mode {
                RuntimeHomeMutationLeaseMode::SharedWriter => {
                    RuntimeHomeMutationLeaseMode::ExclusiveSetup
                }
                RuntimeHomeMutationLeaseMode::ExclusiveSetup => {
                    RuntimeHomeMutationLeaseMode::SharedWriter
                }
            };
            assert!(matches!(
                acquire(
                    &target,
                    conflicting,
                    RuntimeHomeMutationWaitPolicy::Immediate
                )?,
                RuntimeHomeMutationLeaseOutcome::Busy(_)
            ));
            child.finish(exit)?;
            assert!(matches!(
                acquire(
                    &target,
                    conflicting,
                    RuntimeHomeMutationWaitPolicy::Immediate
                )?,
                RuntimeHomeMutationLeaseOutcome::Acquired(_)
            ));
            fixture.close()?;
        }
    }
    Ok(())
}

#[test]
fn parent_unwind_reaps_fixture_and_releases_lease() -> Result<(), Box<dyn Error>> {
    let fixture = tempdir()?;
    let target = fixture.path().join("runtime-home");
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _child = LeaseChild::spawn(
            &target,
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            ChildExit::Normal,
        )
        .expect("spawn mutation-lease fixture before parent unwind");
        panic!("injected parent test unwind");
    }));
    assert!(unwind.is_err());
    assert!(matches!(
        acquire(
            &target,
            RuntimeHomeMutationLeaseMode::SharedWriter,
            RuntimeHomeMutationWaitPolicy::Immediate
        )?,
        RuntimeHomeMutationLeaseOutcome::Acquired(_)
    ));
    fixture.close()?;
    Ok(())
}
