use std::path::{Path, PathBuf};

use volicord_types::IntegrationProfile;

use crate::{
    cli::{
        CodexHost, ConnectionAddArgs, ConnectionListArgs, ConnectionModeArgs, ConnectionRemoveArgs,
        ConnectionReportOutputArgs, ConnectionSelectArgs, InitArgs, RuntimeHomeArgs,
    },
    host_integration::HostKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Json,
    Human(HumanOutputDetail),
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Human(HumanOutputDetail::Concise)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HumanOutputDetail {
    Concise,
    Verbose,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedConnectionOptions {
    pub(super) host_kind: Option<HostKind>,
    pub(super) repo: Option<PathBuf>,
    pub(super) explicit_runtime_home: Option<PathBuf>,
    pub(super) shared: bool,
    pub(super) read_only: bool,
    pub(super) dry_run: bool,
    pub(super) output: OutputFormat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum InitMode {
    #[default]
    Record,
}

impl InitMode {
    pub(super) fn profile_value(self) -> &'static str {
        self.integration_profile().as_str()
    }

    pub(super) fn guard_value(self) -> &'static str {
        self.profile_value()
    }

    pub(super) fn integration_profile(self) -> IntegrationProfile {
        IntegrationProfile::Record
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedInitOptions {
    pub(super) host_kind: Option<HostKind>,
    pub(super) repo: Option<PathBuf>,
    pub(super) explicit_runtime_home: Option<PathBuf>,
    pub(super) mcp_command: Option<PathBuf>,
    pub(super) mode: InitMode,
    pub(super) shared: bool,
    pub(super) dry_run: bool,
    pub(super) output: OutputFormat,
}

pub(super) fn init_options(args: InitArgs, current_dir: &Path) -> ParsedInitOptions {
    ParsedInitOptions {
        host_kind: Some(host_kind(args.host)),
        repo: Some(absolute_path(current_dir, args.repo)),
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        mcp_command: args
            .mcp_command
            .map(|path| absolute_path(current_dir, path)),
        mode: InitMode::Record,
        shared: args.shared,
        dry_run: args.dry_run,
        output: output_format(args.output),
    }
}

pub(super) fn connection_add_options(
    args: ConnectionAddArgs,
    current_dir: &Path,
) -> ParsedConnectionOptions {
    ParsedConnectionOptions {
        host_kind: args.host.map(host_kind),
        repo: args.repo,
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        shared: args.shared,
        read_only: args.read_only,
        dry_run: args.dry_run,
        output: output_format(args.output),
    }
}

pub(super) fn connection_list_options(
    args: ConnectionListArgs,
    current_dir: &Path,
) -> ParsedConnectionOptions {
    ParsedConnectionOptions {
        repo: args.repo,
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        output: if args.json {
            OutputFormat::Json
        } else {
            OutputFormat::Human(HumanOutputDetail::Concise)
        },
        ..ParsedConnectionOptions::default()
    }
}

pub(super) fn connection_select_options(
    args: ConnectionSelectArgs,
    current_dir: &Path,
) -> ParsedConnectionOptions {
    ParsedConnectionOptions {
        host_kind: args.host.map(host_kind),
        repo: args.repo,
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        shared: args.shared,
        output: output_format(args.output),
        ..ParsedConnectionOptions::default()
    }
}

pub(super) fn connection_mode_options(
    args: ConnectionModeArgs,
    current_dir: &Path,
) -> ParsedConnectionOptions {
    ParsedConnectionOptions {
        host_kind: args.host.map(host_kind),
        repo: args.repo,
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        shared: args.shared,
        output: output_format(args.output),
        ..ParsedConnectionOptions::default()
    }
}

pub(super) fn connection_remove_options(
    args: ConnectionRemoveArgs,
    current_dir: &Path,
) -> ParsedConnectionOptions {
    ParsedConnectionOptions {
        host_kind: args.host.map(host_kind),
        repo: args.repo,
        explicit_runtime_home: explicit_runtime_home(args.runtime_home, current_dir),
        shared: args.shared,
        dry_run: args.dry_run,
        output: output_format(args.output),
        ..ParsedConnectionOptions::default()
    }
}

fn explicit_runtime_home(args: RuntimeHomeArgs, current_dir: &Path) -> Option<PathBuf> {
    args.home.map(|path| absolute_path(current_dir, path))
}

pub(super) fn init_output_format(parsed: &ParsedInitOptions) -> OutputFormat {
    parsed.output
}

pub(super) fn connection_output_format(parsed: &ParsedConnectionOptions) -> OutputFormat {
    parsed.output
}

fn output_format(args: ConnectionReportOutputArgs) -> OutputFormat {
    if args.json {
        OutputFormat::Json
    } else if args.verbose {
        OutputFormat::Human(HumanOutputDetail::Verbose)
    } else {
        OutputFormat::Human(HumanOutputDetail::Concise)
    }
}

fn host_kind(_host: CodexHost) -> HostKind {
    HostKind::Codex
}

pub(super) fn absolute_path(current_dir: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        current_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_output_flags_map_to_one_typed_selection() {
        assert_eq!(
            output_format(ConnectionReportOutputArgs::default()),
            OutputFormat::Human(HumanOutputDetail::Concise)
        );
        assert_eq!(
            output_format(ConnectionReportOutputArgs {
                json: false,
                verbose: true,
            }),
            OutputFormat::Human(HumanOutputDetail::Verbose)
        );
        assert_eq!(
            output_format(ConnectionReportOutputArgs {
                json: true,
                verbose: false,
            }),
            OutputFormat::Json
        );
    }

    #[test]
    fn explicit_connection_runtime_home_is_made_absolute_from_current_dir() {
        let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let parsed = connection_list_options(
            ConnectionListArgs {
                repo: None,
                runtime_home: RuntimeHomeArgs {
                    home: Some(PathBuf::from("runtime-home")),
                },
                json: true,
            },
            &current_dir,
        );

        assert_eq!(
            parsed.explicit_runtime_home,
            Some(current_dir.join("runtime-home"))
        );
        assert_eq!(parsed.output, OutputFormat::Json);
    }
}
