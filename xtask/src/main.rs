use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.as_slice() != ["docs-check"] {
        eprintln!("usage: cargo run -p xtask -- docs-check");
        return ExitCode::from(2);
    }

    let result = match env::current_dir() {
        Ok(root) => xtask::run_docs_check(&root),
        Err(error) => Err(error.into()),
    };

    match result {
        Ok(report) if report.is_ok() => {
            println!("docs-check passed");
            ExitCode::SUCCESS
        }
        Ok(report) => {
            eprintln!("docs-check failed with {} error(s):", report.errors().len());
            for error in report.errors() {
                eprintln!("- {error}");
            }
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("docs-check failed: {error}");
            ExitCode::from(1)
        }
    }
}
