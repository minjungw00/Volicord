use std::path::{Path, PathBuf};

use volicord_types::host_configuration::{ConnectionIntent, HostScope};
use volicord_types::values::{HostKind, IntegrationProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiagnosticOperation {
    Status,
    Verify,
}

impl DiagnosticOperation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Verify => "verify",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConnectionUserInvocation {
    Diagnostic {
        operation: DiagnosticOperation,
        host: HostKind,
        repository: PathBuf,
        runtime_home: PathBuf,
        scope: HostScope,
    },
    SelectionRepair {
        host: HostKind,
        repository: PathBuf,
        runtime_home: PathBuf,
        intent: Option<ConnectionIntent>,
    },
    OwningInitRepair {
        host: HostKind,
        repository: PathBuf,
        runtime_home: PathBuf,
        intent: ConnectionIntent,
        profile: IntegrationProfile,
    },
}

impl ConnectionUserInvocation {
    pub(super) fn diagnostic(
        operation: DiagnosticOperation,
        host: HostKind,
        repository: &Path,
        runtime_home: &Path,
        scope: HostScope,
    ) -> Self {
        Self::Diagnostic {
            operation,
            host,
            repository: repository.to_path_buf(),
            runtime_home: runtime_home.to_path_buf(),
            scope,
        }
    }

    pub(super) fn selection_repair(
        host: HostKind,
        repository: &Path,
        runtime_home: &Path,
        intent: Option<ConnectionIntent>,
    ) -> Self {
        Self::SelectionRepair {
            host,
            repository: repository.to_path_buf(),
            runtime_home: runtime_home.to_path_buf(),
            intent,
        }
    }

    pub(super) fn owning_init_repair(
        host: HostKind,
        repository: &Path,
        runtime_home: &Path,
        intent: ConnectionIntent,
        profile: IntegrationProfile,
    ) -> Self {
        Self::OwningInitRepair {
            host,
            repository: repository.to_path_buf(),
            runtime_home: runtime_home.to_path_buf(),
            intent,
            profile,
        }
    }

    pub(super) fn arguments(&self) -> Vec<String> {
        match self {
            Self::Diagnostic {
                operation,
                host,
                repository,
                runtime_home,
                scope,
            } => {
                let mut arguments = vec![
                    "volicord".to_owned(),
                    "connection".to_owned(),
                    operation.as_str().to_owned(),
                    host.as_str().to_owned(),
                    "--repo".to_owned(),
                    path_value(repository),
                    "--home".to_owned(),
                    path_value(runtime_home),
                ];
                if *scope == HostScope::Project {
                    arguments.push("--shared".to_owned());
                }
                arguments.push("--verbose".to_owned());
                arguments
            }
            Self::SelectionRepair {
                host,
                repository,
                runtime_home,
                intent,
            } => match intent {
                Some(ConnectionIntent::Personal) => vec![
                    "volicord".to_owned(),
                    "connection".to_owned(),
                    "add".to_owned(),
                    host.as_str().to_owned(),
                    "--repo".to_owned(),
                    path_value(repository),
                    "--home".to_owned(),
                    path_value(runtime_home),
                ],
                Some(ConnectionIntent::Shared) => vec![
                    "volicord".to_owned(),
                    "init".to_owned(),
                    "--host".to_owned(),
                    host.as_str().to_owned(),
                    "--shared".to_owned(),
                    "--repo".to_owned(),
                    path_value(repository),
                    "--home".to_owned(),
                    path_value(runtime_home),
                ],
                None => vec![
                    "volicord".to_owned(),
                    "init".to_owned(),
                    "--host".to_owned(),
                    host.as_str().to_owned(),
                    "--repo".to_owned(),
                    path_value(repository),
                    "--home".to_owned(),
                    path_value(runtime_home),
                ],
            },
            Self::OwningInitRepair {
                host,
                repository,
                runtime_home,
                intent,
                profile,
            } => {
                let mut arguments = vec!["volicord".to_owned(), "init".to_owned()];
                if *intent == ConnectionIntent::Shared {
                    arguments.push("--shared".to_owned());
                }
                arguments.extend([
                    "--host".to_owned(),
                    host.as_str().to_owned(),
                    "--repo".to_owned(),
                    path_value(repository),
                    "--profile".to_owned(),
                    profile.as_str().to_owned(),
                    "--home".to_owned(),
                    path_value(runtime_home),
                ]);
                arguments
            }
        }
    }

    pub(super) fn render_guidance(&self) -> String {
        let arguments = self.arguments();
        if arguments
            .iter()
            .all(|argument| is_portable_inline_token(argument))
        {
            return self.render_inline_guidance(&arguments.join(" "));
        }
        self.render_structured_guidance()
    }

    fn render_inline_guidance(&self, command: &str) -> String {
        match self {
            Self::Diagnostic {
                operation: DiagnosticOperation::Status,
                ..
            } => format!("Run `{command}` for detailed current Connection diagnostics."),
            Self::Diagnostic {
                operation: DiagnosticOperation::Verify,
                ..
            } => format!("Rerun active verification with `{command}` for detailed diagnostics."),
            Self::SelectionRepair { .. } => format!("Run `{command}` first."),
            Self::OwningInitRepair { .. } => {
                format!("Repair the Guard Installation by rerunning `{command}`.")
            }
        }
    }

    fn render_structured_guidance(&self) -> String {
        match self {
            Self::Diagnostic {
                operation,
                host,
                repository,
                runtime_home,
                scope,
            } => {
                let introduction = match operation {
                    DiagnosticOperation::Status => {
                        "For detailed current Connection diagnostics, run the verbose status command with:"
                    }
                    DiagnosticOperation::Verify => {
                        "For detailed diagnostics, rerun active verification with:"
                    }
                };
                let mut lines =
                    structured_start(introduction, None, *host, repository, runtime_home);
                if *scope == HostScope::Project {
                    lines.push("  Scope: shared".to_owned());
                }
                append_control_notation(&mut lines, repository, runtime_home);
                lines.push("  Verbose output: required.".to_owned());
                lines.join("\n")
            }
            Self::SelectionRepair {
                host,
                repository,
                runtime_home,
                intent,
            } => {
                let (introduction, operation) = match intent {
                    Some(ConnectionIntent::Personal) => (
                        "Add or repair the selected personal Connection with:",
                        "connection add",
                    ),
                    Some(ConnectionIntent::Shared) => (
                        "Initialize or repair the selected shared Connection with:",
                        "init",
                    ),
                    None => (
                        "Initialize Volicord for the selected Connection with:",
                        "init",
                    ),
                };
                let mut lines = structured_start(
                    introduction,
                    Some(operation),
                    *host,
                    repository,
                    runtime_home,
                );
                append_control_notation(&mut lines, repository, runtime_home);
                match intent {
                    Some(ConnectionIntent::Personal) => {
                        lines.push("  Connection intent: personal.".to_owned());
                    }
                    Some(ConnectionIntent::Shared) => {
                        lines.push("  Connection intent: shared".to_owned());
                        lines.push("  Shared scope: required.".to_owned());
                    }
                    None => lines.push("  Connection intent: not specified.".to_owned()),
                }
                lines.join("\n")
            }
            Self::OwningInitRepair {
                host,
                repository,
                runtime_home,
                intent,
                profile,
            } => {
                let mut lines = structured_start(
                    "Repair the Guard Installation by rerunning the owning init operation with:",
                    Some("init"),
                    *host,
                    repository,
                    runtime_home,
                );
                append_control_notation(&mut lines, repository, runtime_home);
                lines.push(format!("  Connection intent: {}", intent.as_str()));
                if *intent == ConnectionIntent::Shared {
                    lines.push("  Shared scope: required".to_owned());
                }
                lines.push(format!("  Profile: {}.", profile.as_str()));
                lines.join("\n")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeHomeSetupState {
    Missing,
    InstallationProfileMissing,
}

pub(super) fn render_runtime_home_setup_guidance(
    runtime_home: &Path,
    state: RuntimeHomeSetupState,
) -> String {
    let introduction = match state {
        RuntimeHomeSetupState::Missing => {
            "RUNTIME_HOME_MISSING: the selected Runtime Home is missing."
        }
        RuntimeHomeSetupState::InstallationProfileMissing => {
            "SETUP_REQUIRED: the selected Runtime Home does not have an Installation Profile."
        }
    };
    let runtime_home_value = path_value(runtime_home);
    let mut lines = vec![
        format!(
            "{introduction} Initialize Volicord from the Product Repository with `volicord init` using:"
        ),
        String::new(),
        render_structured_path("Runtime home", &runtime_home_value),
    ];
    if contains_control(&runtime_home_value) {
        lines.push(control_notation_explanation());
    }
    lines.extend([
        String::new(),
        "Select the host and repository when running `volicord init`.".to_owned(),
    ]);
    lines.join("\n")
}

fn structured_start(
    introduction: &str,
    operation: Option<&str>,
    host: HostKind,
    repository: &Path,
    runtime_home: &Path,
) -> Vec<String> {
    let mut lines = vec![introduction.to_owned(), String::new()];
    if let Some(operation) = operation {
        lines.push(format!("  Operation: {operation}"));
    }
    lines.extend([
        format!("  Host: {}", host.as_str()),
        render_structured_path("Repository", &path_value(repository)),
        render_structured_path("Runtime home", &path_value(runtime_home)),
    ]);
    lines
}

fn append_control_notation(lines: &mut Vec<String>, repository: &Path, runtime_home: &Path) {
    if contains_control(&path_value(repository)) || contains_control(&path_value(runtime_home)) {
        lines.push(control_notation_explanation());
    }
}

fn control_notation_explanation() -> String {
    "  Control-character values use JSON string notation; its quotation marks and escape sequences are not part of the value."
        .to_owned()
}

fn path_value(path: &Path) -> String {
    path.display().to_string()
}

fn is_portable_inline_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'=')
        })
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn render_structured_path(label: &str, value: &str) -> String {
    if contains_control(value) {
        format!(
            "  {label} (JSON string): {}",
            serde_json::to_string(value).expect("a path display value serializes as JSON")
        )
    } else {
        format!("  {label}: {value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPOSITORY: &str = "/workspace/product";
    const RUNTIME_HOME: &str = "/home/user/.volicord";

    #[test]
    fn connection_user_guidance_logical_argument_vectors_are_exact() {
        let cases = [
            (
                ConnectionUserInvocation::diagnostic(
                    DiagnosticOperation::Status,
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    HostScope::User,
                ),
                vec![
                    "volicord",
                    "connection",
                    "status",
                    "codex",
                    "--repo",
                    REPOSITORY,
                    "--home",
                    RUNTIME_HOME,
                    "--verbose",
                ],
            ),
            (
                ConnectionUserInvocation::diagnostic(
                    DiagnosticOperation::Verify,
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    HostScope::Project,
                ),
                vec![
                    "volicord",
                    "connection",
                    "verify",
                    "codex",
                    "--repo",
                    REPOSITORY,
                    "--home",
                    RUNTIME_HOME,
                    "--shared",
                    "--verbose",
                ],
            ),
            (
                ConnectionUserInvocation::selection_repair(
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    Some(ConnectionIntent::Personal),
                ),
                vec![
                    "volicord",
                    "connection",
                    "add",
                    "codex",
                    "--repo",
                    REPOSITORY,
                    "--home",
                    RUNTIME_HOME,
                ],
            ),
            (
                ConnectionUserInvocation::selection_repair(
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    Some(ConnectionIntent::Shared),
                ),
                vec![
                    "volicord",
                    "init",
                    "--host",
                    "codex",
                    "--shared",
                    "--repo",
                    REPOSITORY,
                    "--home",
                    RUNTIME_HOME,
                ],
            ),
            (
                ConnectionUserInvocation::selection_repair(
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    None,
                ),
                vec![
                    "volicord",
                    "init",
                    "--host",
                    "codex",
                    "--repo",
                    REPOSITORY,
                    "--home",
                    RUNTIME_HOME,
                ],
            ),
            (
                ConnectionUserInvocation::owning_init_repair(
                    HostKind::Codex,
                    Path::new(REPOSITORY),
                    Path::new(RUNTIME_HOME),
                    ConnectionIntent::Shared,
                    IntegrationProfile::Record,
                ),
                vec![
                    "volicord",
                    "init",
                    "--shared",
                    "--host",
                    "codex",
                    "--repo",
                    REPOSITORY,
                    "--profile",
                    "record",
                    "--home",
                    RUNTIME_HOME,
                ],
            ),
        ];

        for (invocation, expected) in cases {
            assert_eq!(invocation.arguments(), expected);
            assert_eq!(
                invocation
                    .arguments()
                    .windows(2)
                    .find_map(|pair| (pair[0] == "--home").then_some(pair[1].as_str())),
                Some(RUNTIME_HOME)
            );
        }
    }

    #[test]
    fn connection_user_guidance_portable_token_policy_is_platform_independent() {
        for value in [
            "volicord",
            "connection",
            "status",
            "ABCxyz019_-./:=",
            "C:/Users/Example/.volicord",
        ] {
            assert!(is_portable_inline_token(value), "portable token: {value:?}");
        }
        for value in [
            "",
            "with space",
            "with\ttab",
            "with\nnewline",
            "single'quote",
            "double\"quote",
            r"back\slash",
            "$HOME",
            "%USERPROFILE%",
            "bang!",
            "a&b",
            "a|b",
            "a>b",
            "a<b",
            "(value)",
            "a;b",
            "a*b",
            "a?b",
            "a[b]",
            "@arguments",
            "제품",
        ] {
            assert!(!is_portable_inline_token(value), "unsafe token: {value:?}");
        }
    }

    #[test]
    fn connection_user_guidance_portable_invocations_render_inline_without_quotes() {
        let personal = ConnectionUserInvocation::selection_repair(
            HostKind::Codex,
            Path::new(REPOSITORY),
            Path::new(RUNTIME_HOME),
            Some(ConnectionIntent::Personal),
        );
        assert_eq!(
            personal.render_guidance(),
            "Run `volicord connection add codex --repo /workspace/product --home /home/user/.volicord` first."
        );

        let shared = ConnectionUserInvocation::selection_repair(
            HostKind::Codex,
            Path::new(REPOSITORY),
            Path::new(RUNTIME_HOME),
            Some(ConnectionIntent::Shared),
        );
        assert_eq!(
            shared.render_guidance(),
            "Run `volicord init --host codex --shared --repo /workspace/product --home /home/user/.volicord` first."
        );

        let owning = ConnectionUserInvocation::owning_init_repair(
            HostKind::Codex,
            Path::new(REPOSITORY),
            Path::new(RUNTIME_HOME),
            ConnectionIntent::Personal,
            IntegrationProfile::Record,
        );
        assert_eq!(
            owning.render_guidance(),
            "Repair the Guard Installation by rerunning `volicord init --host codex --repo /workspace/product --profile record --home /home/user/.volicord`."
        );
        for output in [
            personal.render_guidance(),
            shared.render_guidance(),
            owning.render_guidance(),
        ] {
            assert!(!output.contains("'"));
        }
    }

    #[test]
    fn connection_user_guidance_unsafe_values_use_exact_structured_fields() {
        for (repository, runtime_home) in [
            ("/workspace/product repo", RUNTIME_HOME),
            ("/workspace/product's", RUNTIME_HOME),
            (REPOSITORY, "/runtime home"),
            (REPOSITORY, r"C:\Users\Example User\.volicord"),
            ("%USERPROFILE%/product", RUNTIME_HOME),
            ("$HOME/product", RUNTIME_HOME),
            ("/workspace/a&b", RUNTIME_HOME),
            ("/workspace/a|b", RUNTIME_HOME),
            ("/workspace/(product)", RUNTIME_HOME),
            ("/workspace/product;next", RUNTIME_HOME),
            ("/workspace/product*", RUNTIME_HOME),
            ("/workspace/product?", RUNTIME_HOME),
            ("/workspace/control\npath", RUNTIME_HOME),
            ("/workspace/제품", RUNTIME_HOME),
        ] {
            let invocations = [
                ConnectionUserInvocation::diagnostic(
                    DiagnosticOperation::Status,
                    HostKind::Codex,
                    Path::new(repository),
                    Path::new(runtime_home),
                    HostScope::Project,
                ),
                ConnectionUserInvocation::selection_repair(
                    HostKind::Codex,
                    Path::new(repository),
                    Path::new(runtime_home),
                    Some(ConnectionIntent::Personal),
                ),
                ConnectionUserInvocation::owning_init_repair(
                    HostKind::Codex,
                    Path::new(repository),
                    Path::new(runtime_home),
                    ConnectionIntent::Shared,
                    IntegrationProfile::Record,
                ),
            ];
            for invocation in invocations {
                let output = invocation.render_guidance();
                assert!(output.contains(&structured_path_expectation("Repository", repository)));
                assert!(output.contains(&structured_path_expectation("Runtime home", runtime_home)));
                assert!(output.contains("codex"));
                assert!(!output.contains("'\\''"));
                assert!(!output.contains(&format!("'{runtime_home}'")));
                assert!(!output.contains("copy-and-paste"));
                assert!(!output.contains("copyable command"));
                assert!(!output.contains("`volicord"));
                assert!(output.ends_with('.'));
            }
        }
    }

    #[test]
    fn connection_user_guidance_structured_variants_preserve_scope_profile_and_output() {
        let repository = Path::new("/workspace/product repo");
        let runtime_home = Path::new(r"C:\Users\Example User\.volicord");
        let diagnostic = ConnectionUserInvocation::diagnostic(
            DiagnosticOperation::Verify,
            HostKind::Codex,
            repository,
            runtime_home,
            HostScope::Project,
        )
        .render_guidance();
        assert!(diagnostic.contains("active verification"));
        assert!(diagnostic.contains("  Scope: shared\n"));
        assert!(diagnostic.ends_with("  Verbose output: required."));

        let shared = ConnectionUserInvocation::selection_repair(
            HostKind::Codex,
            repository,
            runtime_home,
            Some(ConnectionIntent::Shared),
        )
        .render_guidance();
        assert!(shared.contains("  Operation: init\n"));
        assert!(shared.contains("  Connection intent: shared\n"));
        assert!(shared.ends_with("  Shared scope: required."));

        let unspecified = ConnectionUserInvocation::selection_repair(
            HostKind::Codex,
            repository,
            runtime_home,
            None,
        )
        .render_guidance();
        assert!(unspecified.ends_with("  Connection intent: not specified."));

        let owning = ConnectionUserInvocation::owning_init_repair(
            HostKind::Codex,
            repository,
            runtime_home,
            ConnectionIntent::Shared,
            IntegrationProfile::Record,
        )
        .render_guidance();
        assert!(owning.contains("  Operation: init\n"));
        assert!(owning.contains("  Connection intent: shared\n"));
        assert!(owning.contains("  Shared scope: required\n"));
        assert!(owning.ends_with("  Profile: record."));
    }

    #[test]
    fn connection_user_guidance_setup_messages_are_structured_without_placeholders() {
        for (state, code, state_text) in [
            (
                RuntimeHomeSetupState::Missing,
                "RUNTIME_HOME_MISSING",
                "is missing",
            ),
            (
                RuntimeHomeSetupState::InstallationProfileMissing,
                "SETUP_REQUIRED",
                "does not have an Installation Profile",
            ),
        ] {
            for runtime_home in [
                RUNTIME_HOME,
                "/runtime home",
                r"C:\Users\Example User\.volicord",
                "/runtime/control\npath",
            ] {
                let output = render_runtime_home_setup_guidance(Path::new(runtime_home), state);
                assert!(output.starts_with(code));
                assert!(output.contains(state_text));
                assert!(output.contains(&structured_path_expectation("Runtime home", runtime_home)));
                assert!(output.contains("with `volicord init` using:"));
                assert!(output
                    .ends_with("Select the host and repository when running `volicord init`."));
                assert!(!output.contains("<host>"));
                assert!(!output.contains("<path>"));
                assert!(!output.contains("--home '"));
                assert!(!output.contains("'\\''"));
            }
        }
    }

    fn structured_path_expectation(label: &str, value: &str) -> String {
        if contains_control(value) {
            format!(
                "  {label} (JSON string): {}",
                serde_json::to_string(value).unwrap()
            )
        } else {
            format!("  {label}: {value}")
        }
    }
}
