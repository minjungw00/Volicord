use std::{collections::BTreeMap, env, path::PathBuf, process::ExitCode};

use volicord_release_validation_tests::{
    error::{ValidationError, ValidationResult},
    evaluation::canonical_now,
    gate::{run_gate, GateRequest},
    io::ValidationContext,
    schema::GateVerdict,
};

const USAGE: &str = "Usage: host-release-gate --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json";

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("host-release-gate: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> ValidationResult<u8> {
    let options = parse_options()?;
    let current_dir = env::current_dir()?;
    let context = ValidationContext::from_process(&current_dir)?;
    let manifest = run_gate(
        &context,
        &GateRequest {
            candidate_descriptor: options["--candidate"].clone(),
            cell_directory: options["--cell-dir"].clone(),
            manifest_output: options["--manifest-out"].clone(),
            evaluated_at: canonical_now(),
        },
    )?;
    println!(
        "host release gate {}: {}",
        manifest.verdict.as_str(),
        options["--manifest-out"].display()
    );
    Ok((manifest.verdict == GateVerdict::Fail) as u8)
}

fn parse_options() -> ValidationResult<BTreeMap<&'static str, PathBuf>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && matches!(args[0].to_str(), Some("-h" | "--help")) {
        println!("{USAGE}");
        std::process::exit(0);
    }
    if args.len() != 6 {
        return Err(ValidationError::new(USAGE));
    }
    let mut options = BTreeMap::new();
    for pair in args.chunks_exact(2) {
        let flag = pair[0]
            .to_str()
            .ok_or_else(|| ValidationError::new("option names must be UTF-8"))?;
        let flag = match flag {
            "--candidate" => "--candidate",
            "--cell-dir" => "--cell-dir",
            "--manifest-out" => "--manifest-out",
            _ => return Err(ValidationError::new(format!("unknown option: {flag}"))),
        };
        if options.insert(flag, PathBuf::from(&pair[1])).is_some() {
            return Err(ValidationError::new(format!("duplicate option: {flag}")));
        }
    }
    for required in ["--candidate", "--cell-dir", "--manifest-out"] {
        if !options.contains_key(required) {
            return Err(ValidationError::new(format!(
                "missing required option: {required}"
            )));
        }
    }
    Ok(options)
}
