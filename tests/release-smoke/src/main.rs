use std::{env, ffi::OsString, path::Path, process::ExitCode};

use volicord_release_smoke::{
    is_codex_fixture_executable, run_release_smoke, CODEX_FIXTURE_VERSION,
};

fn main() -> ExitCode {
    let executable = match env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!("release smoke failed to resolve its executable: {error}");
            return ExitCode::from(1);
        }
    };
    let args = env::args_os().skip(1).collect::<Vec<_>>();

    if is_codex_fixture_executable(&executable) {
        return run_codex_fixture(&args);
    }

    match args.as_slice() {
        [option, binary] if option == "--bin" => run_smoke(Path::new(binary), &executable),
        _ => {
            eprintln!("usage: volicord-release-smoke --bin PATH");
            ExitCode::from(2)
        }
    }
}

fn run_codex_fixture(args: &[OsString]) -> ExitCode {
    match args {
        [argument] if argument == "--version" => {
            println!("{CODEX_FIXTURE_VERSION}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("codex fixture supports only --version");
            ExitCode::from(2)
        }
    }
}

fn run_smoke(binary: &Path, fixture_executable: &Path) -> ExitCode {
    match run_release_smoke(binary, fixture_executable) {
        Ok(report) => {
            println!(
                "release smoke passed: {} used MCP {} and exposed {} tool(s)",
                report.binary().display(),
                report.protocol_revision(),
                report.tool_count()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("release smoke failed: {error:#}");
            ExitCode::from(1)
        }
    }
}
