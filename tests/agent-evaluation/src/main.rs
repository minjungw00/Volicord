use std::{env, path::PathBuf, process::ExitCode};

use volicord_agent_evaluation::{
    fixture_evaluation, load_live_config, result_schema_text, run_live, write_result_create_new,
    EvaluationResult, HarnessError, HarnessResult, RunStatus,
};

const DEFAULT_SEED: u64 = 20_260_716;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agent evaluation error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> HarnessResult<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };
    match command.as_str() {
        "fixture" => {
            let options = parse_options(args.collect())?;
            let seed = options.seed.unwrap_or(DEFAULT_SEED);
            let repetitions = options.repetitions.unwrap_or(1);
            let result = fixture_evaluation(seed, repetitions)?;
            emit_result(&result, options.output.as_ref())
        }
        "live" => {
            let options = parse_options(args.collect())?;
            if options.seed.is_some() || options.repetitions.is_some() {
                return Err(HarnessError::new(
                    "live seed and repetitions come from the live configuration",
                ));
            }
            let config_path = options
                .config
                .as_ref()
                .ok_or_else(|| HarnessError::new("live requires --config PATH"))?;
            let config = load_live_config(config_path)?;
            let result = run_live(&config)?;
            emit_result(&result, options.output.as_ref())?;
            if result.status == RunStatus::Incomplete {
                return Err(HarnessError::new(
                    "live trial matrix is incomplete; inspect trial_failures in the result",
                ));
            }
            Ok(())
        }
        "schema" => {
            if args.next().is_some() {
                return Err(HarnessError::new("schema takes no arguments"));
            }
            println!("{}", result_schema_text());
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => Err(HarnessError::new(format!(
            "unknown command {command}; use --help"
        ))),
    }
}

#[derive(Default)]
struct Options {
    seed: Option<u64>,
    repetitions: Option<u32>,
    config: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn parse_options(args: Vec<String>) -> HarnessResult<Options> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let flag = &args[index];
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| HarnessError::new(format!("{flag} requires one following value")))?;
        index += 1;
        match flag.as_str() {
            "--seed" if options.seed.is_none() => {
                options.seed = Some(
                    value
                        .parse()
                        .map_err(|_| HarnessError::new("--seed must be an unsigned integer"))?,
                );
            }
            "--repetitions" if options.repetitions.is_none() => {
                options.repetitions =
                    Some(value.parse().map_err(|_| {
                        HarnessError::new("--repetitions must be an unsigned integer")
                    })?);
            }
            "--config" if options.config.is_none() => {
                options.config = Some(PathBuf::from(value));
            }
            "--output" if options.output.is_none() => {
                options.output = Some(PathBuf::from(value));
            }
            "--seed" | "--repetitions" | "--config" | "--output" => {
                return Err(HarnessError::new(format!(
                    "{flag} may be provided only once"
                )));
            }
            _ => return Err(HarnessError::new(format!("unknown option {flag}"))),
        }
    }
    Ok(options)
}

fn emit_result(result: &EvaluationResult, output: Option<&PathBuf>) -> HarnessResult<()> {
    if let Some(path) = output {
        write_result_create_new(path, result)?;
    }
    let text = serde_json::to_string_pretty(result)
        .map_err(|error| HarnessError::new(format!("result serialization failed: {error}")))?;
    println!("{text}");
    Ok(())
}

fn print_help() {
    println!(
        "volicord-agent-evaluation\n\n\
         Deterministic fixture validation:\n  \
         cargo run -p volicord-agent-evaluation -- fixture [--seed N] [--repetitions N] [--output ABSENT_PATH]\n\n\
         Real host/model evaluation (never run by ordinary tests):\n  \
         cargo run -p volicord-agent-evaluation -- live --config LIVE_CONFIG.json [--output ABSENT_PATH]\n\n\
         Print the result JSON Schema:\n  \
         cargo run -p volicord-agent-evaluation -- schema\n\n\
         The live driver is an external executable. It receives one JSON request on stdin, runs in a fresh\n\
         materialized repository, and returns one content-free aggregate observation JSON object on stdout.\n\
         Credentials are inherited from the operator environment and do not belong in the configuration."
    );
}
