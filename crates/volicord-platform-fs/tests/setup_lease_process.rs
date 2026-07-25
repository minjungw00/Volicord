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
    RuntimeHomeSetupLease, RuntimeHomeSetupLeaseOutcome, RuntimeHomeSetupOperation,
    RuntimeHomeSetupWaitPolicy,
};

const CHILD_RUNTIME_HOME: &str = "VOLICORD_SETUP_LEASE_CHILD_RUNTIME_HOME";
const CHILD_READY: &str = "VOLICORD_SETUP_LEASE_CHILD_READY";

fn acquire(path: &Path) -> Result<RuntimeHomeSetupLeaseOutcome, Box<dyn Error>> {
    Ok(RuntimeHomeSetupLease::acquire(
        path,
        RuntimeHomeSetupOperation::Init,
        RuntimeHomeSetupWaitPolicy::Immediate,
    )?)
}

#[test]
fn process_termination_releases_the_setup_lease() -> Result<(), Box<dyn Error>> {
    if let Some(target) = env::var_os(CHILD_RUNTIME_HOME) {
        let RuntimeHomeSetupLeaseOutcome::Acquired(_lease) = acquire(&PathBuf::from(target))?
        else {
            return Err("child could not acquire the requested setup lease".into());
        };
        println!("{CHILD_READY}");
        std::io::stdout().flush()?;
        let mut until_parent_closes_or_terminates = Vec::new();
        std::io::stdin().read_to_end(&mut until_parent_closes_or_terminates)?;
        return Ok(());
    }

    let fixture = tempdir()?;
    let target = fixture.path().join("runtime-home");
    let mut child = Command::new(env::current_exe()?)
        .arg("--exact")
        .arg("process_termination_releases_the_setup_lease")
        .arg("--nocapture")
        .env(CHILD_RUNTIME_HOME, &target)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or("child setup-lease stdout was not piped")?;
    let mut lines = BufReader::new(stdout).lines();
    loop {
        let line = lines
            .next()
            .ok_or("child exited before signaling setup-lease acquisition")??;
        if line.trim() == CHILD_READY {
            break;
        }
    }

    assert!(matches!(
        acquire(&target)?,
        RuntimeHomeSetupLeaseOutcome::Busy(_)
    ));
    child.kill()?;
    let status = child.wait()?;
    assert!(!status.success());
    assert!(matches!(
        acquire(&target)?,
        RuntimeHomeSetupLeaseOutcome::Acquired(_)
    ));
    Ok(())
}
