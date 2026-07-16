use std::{
    fmt,
    path::{Path, PathBuf},
};

use volicord_store::{storage_upgrade::upgrade_runtime_home_v6_to_v7, StoreError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageCommandError {
    Usage(String),
    Runtime(String),
}

impl fmt::Display for StorageCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StorageCommandError {}

impl From<StoreError> for StorageCommandError {
    fn from(error: StoreError) -> Self {
        Self::Runtime(error.to_string())
    }
}

pub fn storage_usage() -> String {
    concat!(
        "volicord storage upgrade --source-home PATH --destination-home PATH --json\n",
        "volicord storage --help\n"
    )
    .to_owned()
}

/// Runs the explicit offline v6-to-v7 copy conversion without activating either home.
pub fn run_storage_command(
    args: &[String],
    current_dir: &Path,
) -> Result<String, StorageCommandError> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            if args.len() <= 1 {
                Ok(storage_usage())
            } else {
                Err(usage_error(format!("unexpected argument: {}", args[1])))
            }
        }
        Some("upgrade") => {
            let options = parse_upgrade_options(&args[1..])?;
            let source_home = absolute_path(current_dir, &options.source_home);
            let destination_home = absolute_path(current_dir, &options.destination_home);
            let report = upgrade_runtime_home_v6_to_v7(source_home, destination_home)?;
            serde_json::to_string_pretty(&report)
                .map(|output| format!("{output}\n"))
                .map_err(|error| StorageCommandError::Runtime(error.to_string()))
        }
        Some(other) => Err(usage_error(format!("unknown storage command: {other}"))),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct UpgradeOptions {
    source_home: PathBuf,
    destination_home: PathBuf,
    json: bool,
}

fn parse_upgrade_options(args: &[String]) -> Result<UpgradeOptions, StorageCommandError> {
    let mut source_home = None;
    let mut destination_home = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source-home" => {
                if source_home.is_some() {
                    return Err(usage_error("--source-home was supplied more than once"));
                }
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.starts_with('-') && !value.trim().is_empty())
                    .ok_or_else(|| usage_error("--source-home requires a value"))?;
                source_home = Some(PathBuf::from(value));
                index += 1;
            }
            "--destination-home" => {
                if destination_home.is_some() {
                    return Err(usage_error(
                        "--destination-home was supplied more than once",
                    ));
                }
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.starts_with('-') && !value.trim().is_empty())
                    .ok_or_else(|| usage_error("--destination-home requires a value"))?;
                destination_home = Some(PathBuf::from(value));
                index += 1;
            }
            "--json" => {
                if json {
                    return Err(usage_error("--json was supplied more than once"));
                }
                json = true;
                index += 1;
            }
            "-h" | "--help" | "help" => {
                return Err(usage_error(
                    "help cannot be combined with storage upgrade options",
                ));
            }
            option if option.starts_with('-') => {
                return Err(usage_error(format!("unknown option: {option}")));
            }
            argument => return Err(usage_error(format!("unexpected argument: {argument}"))),
        }
    }
    let source_home =
        source_home.ok_or_else(|| usage_error("storage upgrade requires --source-home PATH"))?;
    let destination_home = destination_home
        .ok_or_else(|| usage_error("storage upgrade requires --destination-home PATH"))?;
    if !json {
        return Err(usage_error("storage upgrade requires --json"));
    }
    Ok(UpgradeOptions {
        source_home,
        destination_home,
        json,
    })
}

fn absolute_path(current_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

fn usage_error(message: impl Into<String>) -> StorageCommandError {
    StorageCommandError::Usage(format!("{}\n\n{}", message.into(), storage_usage()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_parser_requires_both_homes_and_json() {
        for args in [
            vec![
                "--source-home",
                "source",
                "--destination-home",
                "destination",
            ],
            vec!["--source-home", "source", "--json"],
            vec!["--destination-home", "destination", "--json"],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_upgrade_options(&args).is_err());
        }
    }

    #[test]
    fn upgrade_parser_rejects_duplicates_and_unknown_options() {
        for args in [
            vec![
                "--source-home",
                "a",
                "--source-home",
                "b",
                "--destination-home",
                "c",
                "--json",
            ],
            vec![
                "--source-home",
                "a",
                "--destination-home",
                "b",
                "--json",
                "--json",
            ],
            vec![
                "--source-home",
                "a",
                "--destination-home",
                "b",
                "--json",
                "--activate",
            ],
        ] {
            let args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_upgrade_options(&args).is_err());
        }
    }

    #[test]
    fn help_is_available_without_a_runtime_home() {
        assert_eq!(
            run_storage_command(&["--help".to_owned()], Path::new("/tmp"))
                .expect("help should render"),
            storage_usage()
        );
    }
}
