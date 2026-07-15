use std::{env, ffi::OsString, path::PathBuf, process::ExitCode};

use volicord_release_validation_tests::{
    candidate::{create_candidate, CandidateRequest},
    error::{ValidationError, ValidationResult},
    io::ValidationContext,
};

const USAGE: &str = "Usage: host-release-candidate --candidate-id CANDIDATE_ID --candidate-path CANDIDATE_BINARY --candidate-out CANDIDATE.json";

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("host-release-candidate: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> ValidationResult<u8> {
    match parse_options(env::args_os().skip(1).collect())? {
        ParsedOptions::Help => {
            println!("{USAGE}");
            Ok(0)
        }
        ParsedOptions::Create(options) => {
            let current_dir = env::current_dir()?;
            let context = ValidationContext::from_process(&current_dir)?;
            let candidate = create_candidate(
                &context,
                &CandidateRequest {
                    candidate_id: options.candidate_id,
                    candidate_path: options.candidate_path,
                    candidate_output: options.candidate_output.clone(),
                },
            )?;
            println!(
                "host release candidate {}: {}",
                candidate.candidate_id,
                options.candidate_output.display()
            );
            Ok(0)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CreateOptions {
    candidate_id: String,
    candidate_path: PathBuf,
    candidate_output: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedOptions {
    Help,
    Create(CreateOptions),
}

fn parse_options(args: Vec<OsString>) -> ValidationResult<ParsedOptions> {
    if args.len() == 1 && matches!(args[0].to_str(), Some("-h" | "--help")) {
        return Ok(ParsedOptions::Help);
    }
    if args.len() != 6 {
        return Err(ValidationError::new(USAGE));
    }

    let mut candidate_id = None;
    let mut candidate_path = None;
    let mut candidate_output = None;
    for pair in args.chunks_exact(2) {
        let flag = pair[0]
            .to_str()
            .ok_or_else(|| ValidationError::new("option names must be UTF-8"))?;
        match flag {
            "--candidate-id" => {
                if candidate_id
                    .replace(
                        pair[1]
                            .to_str()
                            .ok_or_else(|| {
                                ValidationError::new("candidate ID must be exact UTF-8")
                            })?
                            .to_owned(),
                    )
                    .is_some()
                {
                    return Err(ValidationError::new("duplicate option: --candidate-id"));
                }
            }
            "--candidate-path" => {
                if candidate_path.replace(PathBuf::from(&pair[1])).is_some() {
                    return Err(ValidationError::new("duplicate option: --candidate-path"));
                }
            }
            "--candidate-out" => {
                if candidate_output.replace(PathBuf::from(&pair[1])).is_some() {
                    return Err(ValidationError::new("duplicate option: --candidate-out"));
                }
            }
            _ => return Err(ValidationError::new(format!("unknown option: {flag}"))),
        }
    }

    Ok(ParsedOptions::Create(CreateOptions {
        candidate_id: candidate_id
            .ok_or_else(|| ValidationError::new("missing required option: --candidate-id"))?,
        candidate_path: candidate_path
            .ok_or_else(|| ValidationError::new("missing required option: --candidate-path"))?,
        candidate_output: candidate_output
            .ok_or_else(|| ValidationError::new("missing required option: --candidate-out"))?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_create_options_and_help() {
        let parsed = parse_options(
            [
                "--candidate-id",
                "candidate-1",
                "--candidate-path",
                "/tmp/volicord",
                "--candidate-out",
                "/tmp/CANDIDATE.json",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
        )
        .expect("valid options");
        assert_eq!(
            parsed,
            ParsedOptions::Create(CreateOptions {
                candidate_id: "candidate-1".to_owned(),
                candidate_path: PathBuf::from("/tmp/volicord"),
                candidate_output: PathBuf::from("/tmp/CANDIDATE.json"),
            })
        );
        assert_eq!(
            parse_options(vec![OsString::from("--help")]).expect("help"),
            ParsedOptions::Help
        );
    }

    #[test]
    fn rejects_unknown_duplicate_and_missing_options() {
        let unknown = parse_options(
            [
                "--candidate-id",
                "candidate-1",
                "--candidate-path",
                "/tmp/volicord",
                "--unknown",
                "/tmp/CANDIDATE.json",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
        )
        .expect_err("unknown option");
        assert!(unknown.detail().contains("unknown option"));

        let duplicate = parse_options(
            [
                "--candidate-id",
                "candidate-1",
                "--candidate-id",
                "candidate-2",
                "--candidate-out",
                "/tmp/CANDIDATE.json",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
        )
        .expect_err("duplicate option");
        assert!(duplicate.detail().contains("duplicate option"));

        let missing = parse_options(Vec::new()).expect_err("missing options");
        assert_eq!(missing.detail(), USAGE);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_utf8_candidate_id() {
        use std::os::unix::ffi::OsStringExt;

        let error = parse_options(vec![
            OsString::from("--candidate-id"),
            OsString::from_vec(vec![0xff]),
            OsString::from("--candidate-path"),
            OsString::from("/tmp/volicord"),
            OsString::from("--candidate-out"),
            OsString::from("/tmp/CANDIDATE.json"),
        ])
        .expect_err("non-UTF-8 candidate ID");
        assert!(error.detail().contains("candidate ID must be exact UTF-8"));
    }
}
