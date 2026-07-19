#![forbid(unsafe_code)]

use std::{error::Error, process::Command};

fn run(args: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_volicord"))
        .args(args)
        .output()?)
}

#[test]
fn hidden_guard_help_lists_only_codex_record_phases() -> Result<(), Box<dyn Error>> {
    let output = run(&["_hook", "--help"])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    for phase in ["pre-tool", "post-tool", "prompt-capture"] {
        assert!(stdout.contains(phase));
        let phase_help = run(&["_hook", phase, "--help"])?;
        assert!(phase_help.status.success());
        let phase_stdout = String::from_utf8(phase_help.stdout)?;
        assert!(phase_stdout.contains("--host <HOST>"));
        assert!(phase_stdout.contains("--integration-profile <INTEGRATION_PROFILE>"));
        assert!(phase_stdout.contains("--host-output <HOST_OUTPUT>"));
        assert!(phase_stdout.contains("possible values: codex"));
        assert!(phase_stdout.contains("possible values: record"));
    }
    Ok(())
}

#[test]
fn hidden_guard_rejects_unknown_guard_phase_before_setup() -> Result<(), Box<dyn Error>> {
    let output = run(&["_hook", "unknown-phase", "--help"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("unrecognized subcommand"));
    Ok(())
}
