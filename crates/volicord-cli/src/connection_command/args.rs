use std::path::{Path, PathBuf};

use volicord_types::IntegrationProfile;

use crate::{
    cli::{
        CodexHost, ConnectionAddArgs, ConnectionListArgs, ConnectionModeArgs, ConnectionRemoveArgs,
        ConnectionSelectArgs, InitArgs,
    },
    host_integration::HostKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedConnectionOptions {
    pub(super) host_kind: Option<HostKind>,
    pub(super) repo: Option<PathBuf>,
    pub(super) shared: bool,
    pub(super) read_only: bool,
    pub(super) dry_run: bool,
    pub(super) json: bool,
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
    pub(super) runtime_home: Option<PathBuf>,
    pub(super) mcp_command: Option<PathBuf>,
    pub(super) mode: InitMode,
    pub(super) shared: bool,
    pub(super) dry_run: bool,
    pub(super) json: bool,
}

pub(super) fn init_options(args: InitArgs, current_dir: &Path) -> ParsedInitOptions {
    ParsedInitOptions {
        host_kind: Some(host_kind(args.host)),
        repo: Some(absolute_path(current_dir, args.repo)),
        runtime_home: args.home.map(|path| absolute_path(current_dir, path)),
        mcp_command: args
            .mcp_command
            .map(|path| absolute_path(current_dir, path)),
        mode: InitMode::Record,
        shared: args.shared,
        dry_run: args.dry_run,
        json: args.json,
    }
}

impl From<ConnectionAddArgs> for ParsedConnectionOptions {
    fn from(args: ConnectionAddArgs) -> Self {
        Self {
            host_kind: args.host.map(host_kind),
            repo: args.repo,
            shared: args.shared,
            read_only: args.read_only,
            dry_run: args.dry_run,
            json: args.json,
        }
    }
}

impl From<ConnectionListArgs> for ParsedConnectionOptions {
    fn from(args: ConnectionListArgs) -> Self {
        Self {
            repo: args.repo,
            json: args.json,
            ..Self::default()
        }
    }
}

impl From<ConnectionSelectArgs> for ParsedConnectionOptions {
    fn from(args: ConnectionSelectArgs) -> Self {
        Self {
            host_kind: args.host.map(host_kind),
            repo: args.repo,
            shared: args.shared,
            json: args.json,
            ..Self::default()
        }
    }
}

impl From<ConnectionModeArgs> for ParsedConnectionOptions {
    fn from(args: ConnectionModeArgs) -> Self {
        Self {
            host_kind: args.host.map(host_kind),
            repo: args.repo,
            shared: args.shared,
            json: args.json,
            ..Self::default()
        }
    }
}

impl From<ConnectionRemoveArgs> for ParsedConnectionOptions {
    fn from(args: ConnectionRemoveArgs) -> Self {
        Self {
            host_kind: args.host.map(host_kind),
            repo: args.repo,
            shared: args.shared,
            dry_run: args.dry_run,
            json: args.json,
            ..Self::default()
        }
    }
}

pub(super) fn init_output_format(parsed: &ParsedInitOptions) -> OutputFormat {
    output_format(parsed.json)
}

pub(super) fn connection_output_format(parsed: &ParsedConnectionOptions) -> OutputFormat {
    output_format(parsed.json)
}

fn output_format(json: bool) -> OutputFormat {
    if json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
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
