#![forbid(unsafe_code)]

use std::{
    error::Error,
    ffi::OsString,
    io::{self, Read, Write},
    path::PathBuf,
    process,
};

use volicord_platform_fs::{
    RuntimeHomeMutationLease, RuntimeHomeMutationLeaseMode, RuntimeHomeMutationLeaseOutcome,
    RuntimeHomeMutationWaitPolicy,
};

const SCENARIO_ARGUMENT: &str = "--mutation-lease-fixture";
const FIXTURE_PROBE: &[u8] = b"volicord-mutation-lease-fixture-current\n";
const READY: &str = "mutation-lease-ready";

#[derive(Clone, Copy)]
enum FixtureExit {
    Normal,
    Error,
    Panic,
    Terminated,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let Some(selector) = arguments.next() else {
        return Ok(());
    };
    if selector != SCENARIO_ARGUMENT {
        return Err(
            format!("expected fixture selector {SCENARIO_ARGUMENT:?}, got {selector:?}").into(),
        );
    }
    let scenario = next_utf8(&mut arguments, "fixture scenario")?;
    match scenario.as_str() {
        "probe" => {
            require_end(arguments)?;
            io::stdout().write_all(FIXTURE_PROBE)?;
            Ok(())
        }
        "hold" => {
            let target = PathBuf::from(next_argument(&mut arguments, "Runtime Home target")?);
            let mode = parse_mode(&next_utf8(&mut arguments, "lease mode")?)?;
            let exit = parse_exit(&next_utf8(&mut arguments, "exit branch")?)?;
            require_end(arguments)?;
            hold_lease(target, mode, exit)
        }
        other => Err(format!("unsupported mutation-lease fixture scenario: {other}").into()),
    }
}

fn next_argument(
    arguments: &mut impl Iterator<Item = OsString>,
    label: &str,
) -> Result<OsString, Box<dyn Error>> {
    arguments
        .next()
        .ok_or_else(|| format!("missing {label}").into())
}

fn next_utf8(
    arguments: &mut impl Iterator<Item = OsString>,
    label: &str,
) -> Result<String, Box<dyn Error>> {
    next_argument(arguments, label)?
        .into_string()
        .map_err(|value| format!("{label} is not UTF-8: {value:?}").into())
}

fn require_end(mut arguments: impl Iterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    if let Some(argument) = arguments.next() {
        return Err(format!("unexpected fixture argument: {argument:?}").into());
    }
    Ok(())
}

fn parse_mode(value: &str) -> Result<RuntimeHomeMutationLeaseMode, Box<dyn Error>> {
    match value {
        "shared_writer" => Ok(RuntimeHomeMutationLeaseMode::SharedWriter),
        "exclusive_setup" => Ok(RuntimeHomeMutationLeaseMode::ExclusiveSetup),
        other => Err(format!("unsupported mutation-lease fixture mode: {other}").into()),
    }
}

fn parse_exit(value: &str) -> Result<FixtureExit, Box<dyn Error>> {
    match value {
        "normal" => Ok(FixtureExit::Normal),
        "error" => Ok(FixtureExit::Error),
        "panic" => Ok(FixtureExit::Panic),
        "terminated" => Ok(FixtureExit::Terminated),
        other => Err(format!("unsupported mutation-lease fixture exit: {other}").into()),
    }
}

fn hold_lease(
    target: PathBuf,
    mode: RuntimeHomeMutationLeaseMode,
    exit: FixtureExit,
) -> Result<(), Box<dyn Error>> {
    let RuntimeHomeMutationLeaseOutcome::Acquired(_lease) =
        RuntimeHomeMutationLease::acquire(&target, mode, RuntimeHomeMutationWaitPolicy::Immediate)?
    else {
        return Err("fixture could not acquire the requested mutation lease".into());
    };
    println!("{READY}");
    io::stdout().flush()?;

    let mut until_parent_releases_or_terminates = Vec::new();
    io::stdin().read_to_end(&mut until_parent_releases_or_terminates)?;
    match exit {
        FixtureExit::Normal => Ok(()),
        FixtureExit::Error => Err("injected fixture error after mutation-lease acquisition".into()),
        FixtureExit::Panic => panic!("injected fixture panic after mutation-lease acquisition"),
        FixtureExit::Terminated => {
            Err("terminated fixture unexpectedly reached cooperative cleanup".into())
        }
    }
}
