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
                    eprintln!("docs-check failed with {} issue(s):", report.issues().len());
                    for issue in report.issues() {
                        eprintln!("- {issue}");
                    }
                    ExitCode::from(1)
                }
                Err(error) => {
                    eprintln!("docs-check failed: {error}");
                    ExitCode::from(1)
                }
            }
        }
        [command] if command == "docs-sync" => {
            let result = match env::current_dir() {
                Ok(root) => xtask::run_docs_sync(&root),
                Err(error) => Err(error.into()),
            };

            match result {
                Ok(report) => {
                    println!(
                        "docs-sync passed: {} file(s) updated",
                        report.updated_paths().len()
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("docs-sync failed: {error:#}");
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
        [command] if command == "architecture-check" => run_architecture_check_command(),
        [command] if command == "mcp-spec-check" => run_mcp_spec_check_command(),
        [command] if command == "mcp-spec-sync" => run_mcp_spec_sync_command(),
        [command] if command == "release-version-check" => run_release_version_check_command(None),
        [command, option, tag] if command == "release-version-check" && option == "--tag" => {
            run_release_version_check_command(Some(tag))
        }
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- <architecture-check|docs-check|docs-sync|maintainability-report|mcp-spec-check|mcp-spec-sync|release-version-check [--tag TAG]>"
            );
            ExitCode::from(2)
        }
    }
}

fn run_architecture_check_command() -> ExitCode {
    let result = match env::current_dir() {
        Ok(root) => xtask::run_architecture_check(&root),
        Err(error) => Err(error.into()),
    };

    match result {
        Ok(report) if report.is_ok() => {
            println!("architecture-check passed");
            ExitCode::SUCCESS
        }
        Ok(report) => {
            eprintln!(
                "architecture-check failed with {} issue(s):",
                report.issues().len()
            );
            for issue in report.issues() {
                eprintln!("- {issue}");
            }
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("architecture-check failed: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run_mcp_spec_check_command() -> ExitCode {
    let result = match env::current_dir() {
        Ok(root) => xtask::run_mcp_spec_check(&root),
        Err(error) => Err(error.into()),
    };

    match result {
        Ok(report) => {
            println!(
                "mcp-spec-check passed: {} pinned revision(s), {} production-supported, {} tracked pre-release",
                report.pinned_revision_count(),
                report.production_supported_count(),
                report.pre_release_only_count()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("mcp-spec-check failed: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run_mcp_spec_sync_command() -> ExitCode {
    let result = match env::current_dir() {
        Ok(root) => xtask::run_mcp_spec_sync(&root),
        Err(error) => Err(error.into()),
    };

    match result {
        Ok(report) => {
            println!(
                "mcp-spec-sync passed: {} revision(s), {} artifact(s)",
                report.revision_count(),
                report.artifact_count()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("mcp-spec-sync failed: {error:#}");
            ExitCode::from(1)
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
