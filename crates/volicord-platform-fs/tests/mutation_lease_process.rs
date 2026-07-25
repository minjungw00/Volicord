#![forbid(unsafe_code)]

use std::{
    env,
    error::Error,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tempfile::tempdir;
use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};

const CHILD_RUNTIME_HOME: &str = "VOLICORD_MUTATION_LEASE_CHILD_RUNTIME_HOME";
const CHILD_MODE: &str = "VOLICORD_MUTATION_LEASE_CHILD_MODE";
const CHILD_READY: &str = "VOLICORD_MUTATION_LEASE_CHILD_READY";

fn acquire(
    path: &Path,
    mode: RuntimeHomeMutationLeaseMode,
) -> Result<RuntimeHomeMutationLeaseOutcome, Box<dyn Error>> {
    Ok(RuntimeHomeMutationLease::acquire(
        path,
        mode,
        RuntimeHomeMutationWaitPolicy::Immediate,
    )?)
}

fn mode_from_child_environment() -> Result<RuntimeHomeMutationLeaseMode, Box<dyn Error>> {
    match env::var(CHILD_MODE)?.as_str() {
        "shared_writer" => Ok(RuntimeHomeMutationLeaseMode::SharedWriter),
        "exclusive_setup" => Ok(RuntimeHomeMutationLeaseMode::ExclusiveSetup),
        mode => Err(format!("unsupported child mutation lease mode: {mode}").into()),
    }
}

#[test]
fn process_termination_releases_shared_and_exclusive_leases() -> Result<(), Box<dyn Error>> {
    if let Some(target) = env::var_os(CHILD_RUNTIME_HOME) {
        let mode = mode_from_child_environment()?;
        let RuntimeHomeMutationLeaseOutcome::Acquired(_lease) =
            acquire(&PathBuf::from(target), mode)?
        else {
            return Err("child could not acquire the requested mutation lease".into());
        };
        println!("{CHILD_READY}");
        std::io::stdout().flush()?;
        let mut until_parent_closes_or_terminates = Vec::new();
        std::io::stdin().read_to_end(&mut until_parent_closes_or_terminates)?;
        return Ok(());
    }

    for (held_mode, conflicting_mode) in [
        (
            RuntimeHomeMutationLeaseMode::SharedWriter,
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
        ),
        (
            RuntimeHomeMutationLeaseMode::ExclusiveSetup,
            RuntimeHomeMutationLeaseMode::SharedWriter,
        ),
    ] {
        let fixture = tempdir()?;
        let target = fixture.path().join("runtime-home");
        let mut child = Command::new(env::current_exe()?)
            .arg("--exact")
            .arg("process_termination_releases_shared_and_exclusive_leases")
            .arg("--nocapture")
            .env(CHILD_RUNTIME_HOME, &target)
            .env(CHILD_MODE, held_mode.as_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or("child mutation-lease stdout was not piped")?;
        let mut lines = BufReader::new(stdout).lines();
        loop {
            let line = lines
                .next()
                .ok_or("child exited before signaling mutation-lease acquisition")??;
            if line.trim() == CHILD_READY {
                break;
            }
        }

        assert!(matches!(
            acquire(&target, conflicting_mode)?,
            RuntimeHomeMutationLeaseOutcome::Busy(_)
        ));
        child.kill()?;
        let status = child.wait()?;
        assert!(!status.success());
        assert!(matches!(
            acquire(&target, conflicting_mode)?,
            RuntimeHomeMutationLeaseOutcome::Acquired(_)
        ));
    }
    Ok(())
}
