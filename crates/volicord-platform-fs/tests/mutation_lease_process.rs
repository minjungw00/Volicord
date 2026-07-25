#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use tempfile::tempdir;
use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};

const CHILD_RUNTIME_HOME: &str = "VOLICORD_MUTATION_LEASE_CHILD_RUNTIME_HOME";
const CHILD_MODE: &str = "VOLICORD_MUTATION_LEASE_CHILD_MODE";
const CHILD_EXIT: &str = "VOLICORD_MUTATION_LEASE_CHILD_EXIT";
const CHILD_READY: &str = "VOLICORD_MUTATION_LEASE_CHILD_READY";
const TEST_NAME: &str = "cross_process_mutation_lease_protocol";

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

fn child_mode() -> Result<RuntimeHomeMutationLeaseMode, Box<dyn Error>> {
    match env::var(CHILD_MODE)?.as_str() {
        "shared_writer" => Ok(RuntimeHomeMutationLeaseMode::SharedWriter),
        "exclusive_setup" => Ok(RuntimeHomeMutationLeaseMode::ExclusiveSetup),
        mode => Err(format!("unsupported child mutation lease mode: {mode}").into()),
    }
}

fn child_exit() -> Result<ChildExit, Box<dyn Error>> {
    match env::var(CHILD_EXIT)?.as_str() {
        "normal" => Ok(ChildExit::Normal),
        "error" => Ok(ChildExit::Error),
        "panic" => Ok(ChildExit::Panic),
        "terminated" => Ok(ChildExit::Terminated),
        exit => Err(format!("unsupported child mutation lease exit: {exit}").into()),
    }
}

struct LeaseChild {
    child: Child,
    lines: std::io::Lines<BufReader<std::process::ChildStdout>>,
}

impl LeaseChild {
    fn spawn(
        target: &Path,
        mode: RuntimeHomeMutationLeaseMode,
        exit: ChildExit,
    ) -> Result<Self, Box<dyn Error>> {
        let mut child = Command::new(env::current_exe()?)
            .arg("--exact")
            .arg(TEST_NAME)
            .arg("--nocapture")
            .env(CHILD_RUNTIME_HOME, target)
            .env(CHILD_MODE, mode.as_str())
            .env(CHILD_EXIT, exit.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or("child mutation-lease stdout was not piped")?;
        let mut lease_child = Self {
            child,
            lines: BufReader::new(stdout).lines(),
        };
        lease_child.wait_until_ready()?;
        Ok(lease_child)
    }

    fn wait_until_ready(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let line = self
                .lines
                .next()
                .ok_or("child exited before signaling mutation-lease acquisition")??;
            if line.trim() == CHILD_READY {
                return Ok(());
            }
        }
    }

    fn finish(mut self, exit: ChildExit) -> Result<(), Box<dyn Error>> {
        match exit {
            ChildExit::Terminated => self.child.kill()?,
            ChildExit::Normal | ChildExit::Error | ChildExit::Panic => {
                drop(self.child.stdin.take());
            }
        }
        let status = self.child.wait()?;
        match exit {
            ChildExit::Normal => {
                if !status.success() {
                    return Err("normal child exit was not successful".into());
                }
            }
            ChildExit::Error | ChildExit::Panic | ChildExit::Terminated => {
                if status.success() {
                    return Err(
                        format!("{} child exit unexpectedly succeeded", exit.as_str()).into(),
                    );
                }
            }
        }
        Ok(())
    }
}

fn run_child_protocol(target: PathBuf) -> Result<(), Box<dyn Error>> {
    let mode = child_mode()?;
    let exit = child_exit()?;
    let RuntimeHomeMutationLeaseOutcome::Acquired(_lease) =
        acquire(&target, mode, RuntimeHomeMutationWaitPolicy::Immediate)?
    else {
        return Err("child could not acquire the requested mutation lease".into());
    };
    println!("{CHILD_READY}");
    std::io::stdout().flush()?;
    let mut until_parent_closes_or_terminates = Vec::new();
    std::io::stdin().read_to_end(&mut until_parent_closes_or_terminates)?;
    match exit {
        ChildExit::Normal => Ok(()),
        ChildExit::Error => Err("injected child error after mutation-lease acquisition".into()),
        ChildExit::Panic => panic!("injected child panic after mutation-lease acquisition"),
        ChildExit::Terminated => {
            Err("termination child unexpectedly reached normal cleanup".into())
        }
    }
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
    if let Some(target) = env::var_os(CHILD_RUNTIME_HOME) {
        return run_child_protocol(PathBuf::from(target));
    }

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
                if let RuntimeHomeMutationLeaseOutcome::Busy(busy) = outcome {
                    assert_eq!(busy.requested_mode(), requested);
                    assert_eq!(busy.wait_policy(), wait_policy);
                }
                child.finish(ChildExit::Normal)?;
                assert!(matches!(
                    acquire(&target, requested, RuntimeHomeMutationWaitPolicy::Immediate)?,
                    RuntimeHomeMutationLeaseOutcome::Acquired(_)
                ));
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
        }
    }
    Ok(())
}
