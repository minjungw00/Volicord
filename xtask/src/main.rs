use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.as_slice() {
        [command] if command == "docs-check" => {
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
        [command] if command == "maintainability-report" => {
            let result = match env::current_dir() {
                Ok(root) => xtask::run_maintainability_report(&root),
                Err(error) => Err(error.into()),
            };

            match result {
                Ok(report) => {
                    print!("{}", report.render());
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("maintainability-report failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [command] if command == "release-version-check" => run_release_version_check_command(None),
        [command, option, tag] if command == "release-version-check" && option == "--tag" => {
            run_release_version_check_command(Some(tag))
        }
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- <docs-check|maintainability-report|release-version-check [--tag TAG]>"
            );
            ExitCode::from(2)
        }
    }
}

fn run_release_version_check_command(tag: Option<&str>) -> ExitCode {
    let result = match env::current_dir() {
        Ok(root) => xtask::run_release_version_check(&root, tag),
        Err(error) => Err(error.into()),
    };

    match result {
        Ok(report) => {
            if let Some(tag) = report.checked_tag() {
                println!(
                    "release-version-check passed: {tag} matches workspace version {}; {} member package(s) inherit it",
                    report.workspace_version(),
                    report.member_package_count()
                );
            } else {
                println!(
                    "release-version-check passed: workspace version {}; {} member package(s) inherit it",
                    report.workspace_version(),
                    report.member_package_count()
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("release-version-check failed: {error}");
            ExitCode::from(1)
        }
    }
}
