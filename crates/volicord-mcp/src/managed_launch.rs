use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fmt,
    path::{Component, Path, PathBuf},
    process::Command,
    str::FromStr,
};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::host_configuration::HostScope;
use volicord_types::values::HostKind;

pub const VOLICORD_HOME_ENV: &str = "VOLICORD_HOME";
pub const MANAGED_MCP_LAUNCH_ENVIRONMENT_NAMES: [&str; 1] = [VOLICORD_HOME_ENV];

const OBSOLETE_MCP_PROVENANCE_ENVIRONMENT_NAMES: [&str; 4] = [
    "VOLICORD_MCP_LAUNCH",
    "VOLICORD_MCP_HOST",
    "VOLICORD_MCP_CONNECTION_ID",
    "VOLICORD_MCP_VERIFICATION",
];

const MCP_COMMAND: &str = "volicord";
const FINGERPRINT_DOMAIN: &[u8] = b"volicord.codex-managed-configuration\0";

pub fn is_managed_mcp_launch_environment_name(name: &str) -> bool {
    name == VOLICORD_HOME_ENV
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHomeBinding {
    path: String,
}

impl RuntimeHomeBinding {
    pub fn try_new(path: &Path) -> Result<Self, ManagedMcpLaunchError> {
        let has_noncanonical_component = path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir));
        let lexical_normalization = path.components().collect::<PathBuf>();
        if !path.is_absolute()
            || has_noncanonical_component
            || lexical_normalization.as_os_str() != path.as_os_str()
        {
            return Err(ManagedMcpLaunchError::new(
                "personal managed MCP launch requires a canonical absolute Runtime Home",
            ));
        }
        let path = path.to_str().ok_or_else(|| {
            ManagedMcpLaunchError::new(
                "personal managed MCP launch requires a UTF-8 Runtime Home path",
            )
        })?;
        if path.is_empty() {
            return Err(ManagedMcpLaunchError::new(
                "personal managed MCP launch requires a non-empty Runtime Home",
            ));
        }
        Ok(Self {
            path: path.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchEnvironment {
    static_values: BTreeMap<String, String>,
    forwarded_names: BTreeSet<String>,
}

impl LaunchEnvironment {
    pub fn try_new(
        static_values: BTreeMap<String, String>,
        forwarded_names: Vec<String>,
    ) -> Result<Self, ManagedMcpLaunchError> {
        let mut forwarded = BTreeSet::new();
        for name in forwarded_names {
            if name.trim().is_empty() {
                return Err(ManagedMcpLaunchError::new(
                    "managed MCP forwarded environment names must not be blank",
                ));
            }
            if !forwarded.insert(name.clone()) {
                return Err(ManagedMcpLaunchError::new(format!(
                    "managed MCP forwarded environment name is duplicated: {name}"
                )));
            }
            if static_values.contains_key(&name) {
                return Err(ManagedMcpLaunchError::new(format!(
                    "managed MCP environment name cannot be both static and forwarded: {name}"
                )));
            }
        }
        Ok(Self {
            static_values,
            forwarded_names: forwarded,
        })
    }

    pub fn static_values(&self) -> &BTreeMap<String, String> {
        &self.static_values
    }

    pub fn forwarded_names(&self) -> &BTreeSet<String> {
        &self.forwarded_names
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedMcpBinding {
    Personal {
        runtime_home: RuntimeHomeBinding,
        connection_id: String,
    },
    SharedRepository {
        host_kind: HostKind,
    },
}

impl ManagedMcpBinding {
    pub const fn host_kind(&self) -> HostKind {
        match self {
            Self::Personal { .. } => HostKind::Codex,
            Self::SharedRepository { host_kind } => *host_kind,
        }
    }

    pub const fn host_scope(&self) -> HostScope {
        match self {
            Self::Personal { .. } => HostScope::User,
            Self::SharedRepository { .. } => HostScope::Project,
        }
    }

    pub fn runtime_home(&self) -> Option<&RuntimeHomeBinding> {
        match self {
            Self::Personal { runtime_home, .. } => Some(runtime_home),
            Self::SharedRepository { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpLaunchSpec {
    command: String,
    args: Vec<String>,
    environment: LaunchEnvironment,
    binding: ManagedMcpBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedMcpInvocationPurpose {
    ManagedStdio,
    CliStdioHandshake,
    CliPreflightCheck {
        connection_id: String,
        project_id: Option<String>,
    },
}

impl ManagedMcpInvocationPurpose {
    pub fn cli_preflight_check(
        connection_id: impl Into<String>,
        project_id: Option<&str>,
    ) -> Result<Self, ManagedMcpLaunchError> {
        Ok(Self::CliPreflightCheck {
            connection_id: nonblank(connection_id.into(), "preflight connection ID")?,
            project_id: project_id
                .map(|value| nonblank(value.to_owned(), "preflight project ID"))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedMcpWorkingDirectory {
    Inherited,
    ProductRepository(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpMaterializationInput {
    purpose: ManagedMcpInvocationPurpose,
    forwarded_environment: BTreeMap<String, OsString>,
    working_directory: ManagedMcpWorkingDirectory,
}

impl ManagedMcpMaterializationInput {
    pub fn new(
        purpose: ManagedMcpInvocationPurpose,
        forwarded_environment: BTreeMap<String, OsString>,
        working_directory: ManagedMcpWorkingDirectory,
    ) -> Self {
        Self {
            purpose,
            forwarded_environment,
            working_directory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedManagedMcpLaunch {
    command: String,
    args: Vec<String>,
    environment: BTreeMap<String, OsString>,
    working_directory: ManagedMcpWorkingDirectory,
    purpose: ManagedMcpInvocationPurpose,
}

impl MaterializedManagedMcpLaunch {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn environment(&self) -> &BTreeMap<String, OsString> {
        &self.environment
    }

    pub fn working_directory(&self) -> &ManagedMcpWorkingDirectory {
        &self.working_directory
    }

    pub fn purpose(&self) -> &ManagedMcpInvocationPurpose {
        &self.purpose
    }

    pub fn process_command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command.args(&self.args);
        self.apply_process_context(&mut command);
        command
    }

    fn apply_process_context(&self, command: &mut Command) {
        command.env_remove(VOLICORD_HOME_ENV);
        for name in OBSOLETE_MCP_PROVENANCE_ENVIRONMENT_NAMES {
            command.env_remove(name);
        }
        command.env_remove("VOLICORD_MCP_PROJECT_ID");
        for (name, value) in &self.environment {
            command.env(name, value);
        }
        if let ManagedMcpWorkingDirectory::ProductRepository(path) = &self.working_directory {
            command.current_dir(path);
        }
    }
}

impl ManagedMcpLaunchSpec {
    pub const PATH_COMMAND: &'static str = MCP_COMMAND;

    pub fn personal(
        command: &Path,
        runtime_home: &Path,
        connection_id: impl Into<String>,
    ) -> Result<Self, ManagedMcpLaunchError> {
        validate_personal_command(command)?;
        let command = command.to_str().ok_or_else(|| {
            ManagedMcpLaunchError::new(
                "personal managed MCP launch requires a UTF-8 executable path",
            )
        })?;
        let runtime_home = RuntimeHomeBinding::try_new(runtime_home)?;
        let connection_id = nonblank(connection_id.into(), "connection ID")?;

        let args = vec![
            "_host-launch".to_owned(),
            HostKind::Codex.as_str().to_owned(),
            "--connection".to_owned(),
            connection_id.clone(),
        ];

        let static_values = BTreeMap::from([(
            VOLICORD_HOME_ENV.to_owned(),
            runtime_home.as_str().to_owned(),
        )]);

        Ok(Self {
            command: command.to_owned(),
            args,
            environment: LaunchEnvironment::try_new(static_values, Vec::new())?,
            binding: ManagedMcpBinding::Personal {
                runtime_home,
                connection_id,
            },
        })
    }

    pub fn shared_repository(host_kind: HostKind) -> Result<Self, ManagedMcpLaunchError> {
        if host_kind != HostKind::Codex {
            return Err(ManagedMcpLaunchError::new(
                "shared managed MCP launch host must be codex",
            ));
        }
        Ok(Self {
            command: MCP_COMMAND.to_owned(),
            args: vec![
                "_host-launch".to_owned(),
                host_kind.as_str().to_owned(),
                "--discover-repository".to_owned(),
            ],
            environment: LaunchEnvironment::try_new(
                BTreeMap::new(),
                vec![VOLICORD_HOME_ENV.to_owned()],
            )?,
            binding: ManagedMcpBinding::SharedRepository { host_kind },
        })
    }

    pub fn try_from_host_projection(
        command: String,
        args: Vec<String>,
        static_environment: BTreeMap<String, String>,
        forwarded_environment: Vec<String>,
    ) -> Result<Self, ManagedMcpLaunchError> {
        let environment = LaunchEnvironment::try_new(static_environment, forwarded_environment)?;
        let candidate = Self {
            command,
            args,
            environment,
            binding: ManagedMcpBinding::SharedRepository {
                host_kind: HostKind::Codex,
            },
        };

        let expected = if let [launcher, host, discover] = candidate.args.as_slice() {
            if launcher != "_host-launch" || discover != "--discover-repository" {
                return Err(ManagedMcpLaunchError::invalid_shape());
            }
            let host_kind = HostKind::from_str(host)
                .map_err(|_| ManagedMcpLaunchError::new("managed MCP launch host must be codex"))?;
            Self::shared_repository(host_kind)?
        } else if candidate.args.len() == 4
            && candidate.args[0] == "_host-launch"
            && candidate.args[1] == HostKind::Codex.as_str()
            && candidate.args[2] == "--connection"
        {
            let connection_id = candidate.args[3].as_str();
            let runtime_home = candidate
                .environment
                .static_values
                .get(VOLICORD_HOME_ENV)
                .ok_or_else(|| {
                    ManagedMcpLaunchError::new(
                        "personal managed MCP launch requires a static VOLICORD_HOME",
                    )
                })?;
            Self::personal(
                Path::new(&candidate.command),
                Path::new(runtime_home),
                connection_id,
            )?
        } else {
            return Err(ManagedMcpLaunchError::invalid_shape());
        };

        if candidate.command != expected.command
            || candidate.args != expected.args
            || candidate.environment != expected.environment
        {
            return Err(ManagedMcpLaunchError::invalid_shape());
        }
        Ok(expected)
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn environment(&self) -> &LaunchEnvironment {
        &self.environment
    }

    pub fn binding(&self) -> &ManagedMcpBinding {
        &self.binding
    }

    pub fn host_kind(&self) -> HostKind {
        self.binding.host_kind()
    }

    pub fn host_scope(&self) -> HostScope {
        self.binding.host_scope()
    }

    pub fn canonical_projection(&self) -> Value {
        serde_json::to_value(CanonicalLaunchProjection::from(self))
            .expect("managed MCP launch projection should serialize")
    }

    pub fn managed_fingerprint(&self, server_name: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(FINGERPRINT_DOMAIN);
        digest.update(self.host_kind().as_str().as_bytes());
        digest.update([0]);
        digest.update(self.host_scope().as_str().as_bytes());
        digest.update([0]);
        digest.update(server_name.as_bytes());
        digest.update([0]);
        digest.update(
            serde_json::to_vec(&CanonicalLaunchProjection::from(self))
                .expect("managed MCP launch projection should serialize"),
        );
        format!("sha256:{:x}", digest.finalize())
    }

    pub fn materialize(
        &self,
        input: ManagedMcpMaterializationInput,
    ) -> Result<MaterializedManagedMcpLaunch, ManagedMcpLaunchError> {
        validate_working_directory(&self.binding, &input.working_directory)?;
        for name in input.forwarded_environment.keys() {
            if !self.environment.forwarded_names.contains(name) {
                return Err(ManagedMcpLaunchError::new(format!(
                    "managed MCP materialization received an undeclared forwarded environment value: {name}"
                )));
            }
        }

        let mut environment = self
            .environment
            .static_values
            .iter()
            .map(|(name, value)| (name.clone(), OsString::from(value)))
            .collect::<BTreeMap<_, _>>();
        for name in &self.environment.forwarded_names {
            let value = input.forwarded_environment.get(name).ok_or_else(|| {
                ManagedMcpLaunchError::new(format!(
                    "managed MCP materialization is missing forwarded environment value: {name}"
                ))
            })?;
            environment.insert(name.clone(), value.clone());
        }
        let args = invocation_args(&self.binding, &self.args, &input.purpose)?;
        Ok(MaterializedManagedMcpLaunch {
            command: self.command.clone(),
            args,
            environment,
            working_directory: input.working_directory,
            purpose: input.purpose,
        })
    }
}

fn invocation_args(
    binding: &ManagedMcpBinding,
    stdio_args: &[String],
    purpose: &ManagedMcpInvocationPurpose,
) -> Result<Vec<String>, ManagedMcpLaunchError> {
    match purpose {
        ManagedMcpInvocationPurpose::ManagedStdio => Ok(stdio_args.to_vec()),
        ManagedMcpInvocationPurpose::CliStdioHandshake => Ok(manual_stdio_args(binding)),
        ManagedMcpInvocationPurpose::CliPreflightCheck {
            connection_id,
            project_id,
        } => {
            if let ManagedMcpBinding::Personal {
                connection_id: bound_connection_id,
                ..
            } = binding
            {
                if connection_id != bound_connection_id {
                    return Err(ManagedMcpLaunchError::new(
                        "personal managed MCP preflight Connection must match the launch contract",
                    ));
                }
            }
            let mut args = vec![
                "mcp".to_owned(),
                "preflight".to_owned(),
                "--connection".to_owned(),
                connection_id.clone(),
            ];
            if let Some(project_id) = project_id {
                args.extend(["--project".to_owned(), project_id.clone()]);
            }
            args.push("--json".to_owned());
            Ok(args)
        }
    }
}

fn manual_stdio_args(binding: &ManagedMcpBinding) -> Vec<String> {
    match binding {
        ManagedMcpBinding::Personal { connection_id, .. } => vec![
            "mcp".to_owned(),
            "serve".to_owned(),
            "--connection".to_owned(),
            connection_id.clone(),
        ],
        ManagedMcpBinding::SharedRepository { host_kind } => vec![
            "mcp".to_owned(),
            "serve".to_owned(),
            "--discover-repository".to_owned(),
            "--host".to_owned(),
            host_kind.as_str().to_owned(),
        ],
    }
}

fn validate_working_directory(
    binding: &ManagedMcpBinding,
    working_directory: &ManagedMcpWorkingDirectory,
) -> Result<(), ManagedMcpLaunchError> {
    match (binding, working_directory) {
        (ManagedMcpBinding::Personal { .. }, ManagedMcpWorkingDirectory::Inherited) => Ok(()),
        (
            ManagedMcpBinding::SharedRepository { .. },
            ManagedMcpWorkingDirectory::ProductRepository(path),
        ) if is_canonical_absolute_path(path) => Ok(()),
        (ManagedMcpBinding::Personal { .. }, _) => Err(ManagedMcpLaunchError::new(
            "personal managed MCP launch must use repository-independent working-directory policy",
        )),
        (ManagedMcpBinding::SharedRepository { .. }, _) => Err(ManagedMcpLaunchError::new(
            "shared managed MCP launch requires a canonical absolute Product Repository working directory",
        )),
    }
}

fn is_canonical_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        && path.components().collect::<PathBuf>().as_os_str() == path.as_os_str()
}

#[derive(Serialize)]
struct CanonicalLaunchProjection<'a> {
    command: &'a str,
    args: &'a [String],
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    env: &'a BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    env_vars: Vec<&'a str>,
}

impl<'a> From<&'a ManagedMcpLaunchSpec> for CanonicalLaunchProjection<'a> {
    fn from(spec: &'a ManagedMcpLaunchSpec) -> Self {
        Self {
            command: &spec.command,
            args: &spec.args,
            env: &spec.environment.static_values,
            env_vars: spec
                .environment
                .forwarded_names
                .iter()
                .map(String::as_str)
                .collect(),
        }
    }
}

fn validate_personal_command(command: &Path) -> Result<(), ManagedMcpLaunchError> {
    let valid_name = command
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == MCP_COMMAND)
        && command
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| extension.eq_ignore_ascii_case("exe"));
    if command.is_absolute() && valid_name {
        Ok(())
    } else {
        Err(ManagedMcpLaunchError::new(
            "personal managed MCP launch requires the selected absolute volicord executable",
        ))
    }
}

fn nonblank(value: String, label: &str) -> Result<String, ManagedMcpLaunchError> {
    if value.trim().is_empty() {
        Err(ManagedMcpLaunchError::new(format!(
            "managed MCP launch {label} must not be blank"
        )))
    } else {
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpLaunchError {
    message: String,
}

impl ManagedMcpLaunchError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn invalid_shape() -> Self {
        Self::new("managed MCP launch projection is not the canonical personal or shared shape")
    }
}

impl fmt::Display for ManagedMcpLaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ManagedMcpLaunchError {}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn personal() -> ManagedMcpLaunchSpec {
        ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
        )
        .expect("personal launch")
    }

    #[test]
    fn personal_launch_has_exact_static_runtime_binding() {
        let spec = personal();
        assert_eq!(
            spec.canonical_projection(),
            serde_json::json!({
                "command": "/opt/volicord/bin/volicord",
                "args": ["_host-launch", "codex", "--connection", "connection_alpha"],
                "env": {
                    "VOLICORD_HOME": "/srv/volicord/runtime"
                }
            })
        );
        assert!(spec.environment().forwarded_names().is_empty());
        assert_eq!(
            spec.binding()
                .runtime_home()
                .map(RuntimeHomeBinding::as_str),
            Some("/srv/volicord/runtime")
        );
    }

    #[test]
    fn shared_launch_has_exact_clone_portable_shape() {
        let spec = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        assert_eq!(
            spec.canonical_projection(),
            serde_json::json!({
                "command": "volicord",
                "args": ["_host-launch", "codex", "--discover-repository"],
                "env_vars": ["VOLICORD_HOME"]
            })
        );
        assert!(spec.environment().static_values().is_empty());
    }

    #[test]
    fn canonical_projection_and_fingerprint_are_deterministic() {
        let first = personal();
        let reparsed = ManagedMcpLaunchSpec::try_from_host_projection(
            first.command().to_owned(),
            first.args().to_vec(),
            first.environment().static_values().clone(),
            first
                .environment()
                .forwarded_names()
                .iter()
                .cloned()
                .collect(),
        )
        .expect("strict round trip");
        assert_eq!(
            first.canonical_projection(),
            reparsed.canonical_projection()
        );
        assert_eq!(
            first.managed_fingerprint("volicord"),
            reparsed.managed_fingerprint("volicord")
        );

        let other_connection = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_beta",
        )
        .expect("other personal launch");
        assert_ne!(
            first.managed_fingerprint("volicord"),
            other_connection.managed_fingerprint("volicord")
        );
        assert_ne!(
            first.managed_fingerprint("volicord"),
            first.managed_fingerprint("volicord-other")
        );
    }

    #[test]
    fn environment_rejects_collisions_blank_names_and_duplicates() {
        assert!(LaunchEnvironment::try_new(
            BTreeMap::from([(VOLICORD_HOME_ENV.to_owned(), "/runtime".to_owned())]),
            vec![VOLICORD_HOME_ENV.to_owned()],
        )
        .is_err());
        assert!(LaunchEnvironment::try_new(BTreeMap::new(), vec![" ".to_owned()]).is_err());
        assert!(LaunchEnvironment::try_new(
            BTreeMap::new(),
            vec![VOLICORD_HOME_ENV.to_owned(), VOLICORD_HOME_ENV.to_owned()],
        )
        .is_err());
    }

    #[test]
    fn personal_launch_rejects_missing_local_coordinates_and_forwarding() {
        assert!(ManagedMcpLaunchSpec::personal(
            Path::new("volicord"),
            Path::new("/runtime"),
            "connection_alpha",
        )
        .is_err());
        assert!(ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord"),
            Path::new("relative/runtime"),
            "connection_alpha",
        )
        .is_err());
        for runtime_home in [
            Path::new("/runtime/../other"),
            Path::new("/runtime/./home"),
            Path::new("/runtime/home/"),
        ] {
            assert!(ManagedMcpLaunchSpec::personal(
                Path::new("/opt/volicord"),
                runtime_home,
                "connection_alpha",
            )
            .is_err());
        }

        let spec = personal();
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            spec.command().to_owned(),
            spec.args().to_vec(),
            spec.environment().static_values().clone(),
            vec![VOLICORD_HOME_ENV.to_owned()],
        )
        .is_err());
    }

    #[test]
    fn personal_projection_rejects_project_argument_and_environment_marker() {
        let spec = personal();
        let mut project_args = spec.args().to_vec();
        project_args.extend(["--project".to_owned(), "project_alpha".to_owned()]);
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            spec.command().to_owned(),
            project_args,
            spec.environment().static_values().clone(),
            Vec::new(),
        )
        .is_err());

        let mut project_environment = spec.environment().static_values().clone();
        project_environment.insert(
            "VOLICORD_MCP_PROJECT_ID".to_owned(),
            "project_alpha".to_owned(),
        );
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            spec.command().to_owned(),
            spec.args().to_vec(),
            project_environment,
            Vec::new(),
        )
        .is_err());
    }

    #[test]
    fn strict_projection_rejects_mixed_personal_and_shared_shapes() {
        let shared = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        let mut shared_env = shared.environment().static_values().clone();
        shared_env.insert(
            "VOLICORD_MCP_CONNECTION_ID".to_owned(),
            "connection_local".to_owned(),
        );
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            shared.command().to_owned(),
            shared.args().to_vec(),
            shared_env,
            vec![VOLICORD_HOME_ENV.to_owned()],
        )
        .is_err());

        let personal = personal();
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            personal.command().to_owned(),
            vec![
                "_host-launch".to_owned(),
                "codex".to_owned(),
                "--discover-repository".to_owned(),
            ],
            personal.environment().static_values().clone(),
            Vec::new(),
        )
        .is_err());
        assert!(ManagedMcpLaunchSpec::try_from_host_projection(
            "/absolute/volicord".to_owned(),
            shared.args().to_vec(),
            BTreeMap::new(),
            vec![VOLICORD_HOME_ENV.to_owned()],
        )
        .is_err());
    }

    #[test]
    fn personal_materialization_uses_static_runtime_home_and_cleans_ambient_managed_names() {
        let spec = personal();
        let materialized = spec
            .materialize(ManagedMcpMaterializationInput::new(
                ManagedMcpInvocationPurpose::ManagedStdio,
                BTreeMap::new(),
                ManagedMcpWorkingDirectory::Inherited,
            ))
            .expect("personal materialization");
        assert_eq!(
            materialized.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/srv/volicord/runtime"))
        );

        let mut command = Command::new(materialized.command());
        for name in OBSOLETE_MCP_PROVENANCE_ENVIRONMENT_NAMES
            .into_iter()
            .chain([VOLICORD_HOME_ENV, "VOLICORD_MCP_PROJECT_ID"])
        {
            command.env(name, "ambient-decoy");
        }
        materialized.apply_process_context(&mut command);
        for (name, expected) in spec.environment().static_values() {
            assert_eq!(
                command
                    .get_envs()
                    .find(|(candidate, _)| *candidate == OsStr::new(name))
                    .and_then(|(_, value)| value),
                Some(OsStr::new(expected)),
                "static contract value must replace an ambient {name}"
            );
        }
        for name in [
            "VOLICORD_MCP_PROJECT_ID",
            "VOLICORD_MCP_LAUNCH",
            "VOLICORD_MCP_HOST",
            "VOLICORD_MCP_CONNECTION_ID",
            "VOLICORD_MCP_VERIFICATION",
        ] {
            assert_eq!(
                command
                    .get_envs()
                    .find(|(candidate, _)| *candidate == OsStr::new(name))
                    .map(|(_, value)| value),
                Some(None),
                "unbound managed name must be removed: {name}"
            );
        }
    }

    #[test]
    fn shared_materialization_uses_only_explicit_forwarded_runtime_home() {
        let spec = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        let input = ManagedMcpMaterializationInput::new(
            ManagedMcpInvocationPurpose::CliStdioHandshake,
            BTreeMap::from([(
                VOLICORD_HOME_ENV.to_owned(),
                OsString::from("/selected/runtime-home"),
            )]),
            ManagedMcpWorkingDirectory::ProductRepository(PathBuf::from("/workspace/product")),
        );
        let first = spec
            .materialize(input.clone())
            .expect("first materialization");
        let second = spec.materialize(input).expect("second materialization");

        assert_eq!(first, second);
        assert_eq!(
            first.environment().get(VOLICORD_HOME_ENV),
            Some(&OsString::from("/selected/runtime-home"))
        );
        assert_eq!(
            first.args(),
            ["mcp", "serve", "--discover-repository", "--host", "codex"]
        );
        let mut command = Command::new(first.command());
        for name in OBSOLETE_MCP_PROVENANCE_ENVIRONMENT_NAMES
            .into_iter()
            .chain([VOLICORD_HOME_ENV, "VOLICORD_MCP_PROJECT_ID"])
        {
            command.env(name, "ambient-decoy");
        }
        first.apply_process_context(&mut command);
        assert_eq!(
            command
                .get_envs()
                .find(|(candidate, _)| *candidate == OsStr::new(VOLICORD_HOME_ENV))
                .and_then(|(_, value)| value),
            Some(OsStr::new("/selected/runtime-home"))
        );
        for name in [
            "VOLICORD_MCP_LAUNCH",
            "VOLICORD_MCP_HOST",
            "VOLICORD_MCP_CONNECTION_ID",
            "VOLICORD_MCP_VERIFICATION",
            "VOLICORD_MCP_PROJECT_ID",
        ] {
            assert_eq!(
                command
                    .get_envs()
                    .find(|(candidate, _)| *candidate == OsStr::new(name))
                    .map(|(_, value)| value),
                Some(None),
                "decoy managed marker must be removed: {name}"
            );
        }
    }

    #[test]
    fn shared_materialization_rejects_missing_or_undeclared_forwarded_values() {
        let spec = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        let missing = spec
            .materialize(ManagedMcpMaterializationInput::new(
                ManagedMcpInvocationPurpose::ManagedStdio,
                BTreeMap::new(),
                ManagedMcpWorkingDirectory::ProductRepository(PathBuf::from("/workspace/product")),
            ))
            .expect_err("missing forwarded Runtime Home must fail");
        assert_eq!(
            missing.to_string(),
            "managed MCP materialization is missing forwarded environment value: VOLICORD_HOME"
        );

        let undeclared = personal()
            .materialize(ManagedMcpMaterializationInput::new(
                ManagedMcpInvocationPurpose::ManagedStdio,
                BTreeMap::from([(
                    VOLICORD_HOME_ENV.to_owned(),
                    OsString::from("/decoy/runtime-home"),
                )]),
                ManagedMcpWorkingDirectory::Inherited,
            ))
            .expect_err("personal materialization must reject a forwarded override");
        assert!(undeclared
            .to_string()
            .contains("undeclared forwarded environment"));
    }

    #[test]
    fn cli_invocations_use_public_manual_commands_without_provenance_environment() {
        let spec = personal();
        let materialize = |purpose| {
            spec.materialize(ManagedMcpMaterializationInput::new(
                purpose,
                BTreeMap::new(),
                ManagedMcpWorkingDirectory::Inherited,
            ))
            .expect("materialized invocation")
        };

        let managed = materialize(ManagedMcpInvocationPurpose::ManagedStdio);
        assert_eq!(
            managed.args(),
            ["_host-launch", "codex", "--connection", "connection_alpha"]
        );
        let handshake = materialize(ManagedMcpInvocationPurpose::CliStdioHandshake);
        assert_eq!(
            handshake.args(),
            ["mcp", "serve", "--connection", "connection_alpha"]
        );
        let preflight = materialize(
            ManagedMcpInvocationPurpose::cli_preflight_check("connection_alpha", None)
                .expect("preflight purpose"),
        );
        assert_eq!(
            preflight.args(),
            [
                "mcp",
                "preflight",
                "--connection",
                "connection_alpha",
                "--json"
            ]
        );
        for invocation in [managed, handshake, preflight] {
            for name in OBSOLETE_MCP_PROVENANCE_ENVIRONMENT_NAMES {
                assert!(!invocation.environment().contains_key(name));
            }
        }
    }

    #[test]
    fn preflight_arguments_use_selected_coordinates_without_changing_launch_identity() {
        let spec = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
        )
        .expect("personal launch");
        let projection = spec.canonical_projection();
        let fingerprint = spec.managed_fingerprint("volicord");
        let preflight = spec
            .materialize(ManagedMcpMaterializationInput::new(
                ManagedMcpInvocationPurpose::cli_preflight_check(
                    "connection_alpha",
                    Some("project_alpha"),
                )
                .expect("preflight purpose"),
                BTreeMap::new(),
                ManagedMcpWorkingDirectory::Inherited,
            ))
            .expect("preflight materialization");
        assert_eq!(
            preflight.args(),
            [
                "mcp",
                "preflight",
                "--connection",
                "connection_alpha",
                "--project",
                "project_alpha",
                "--json"
            ]
        );
        assert_eq!(spec.canonical_projection(), projection);
        assert_eq!(spec.managed_fingerprint("volicord"), fingerprint);

        let mismatch = spec.materialize(ManagedMcpMaterializationInput::new(
            ManagedMcpInvocationPurpose::cli_preflight_check(
                "connection_decoy",
                Some("project_alpha"),
            )
            .expect("preflight purpose"),
            BTreeMap::new(),
            ManagedMcpWorkingDirectory::Inherited,
        ));
        assert!(mismatch.is_err());
    }

    #[test]
    fn working_directory_materialization_does_not_change_managed_fingerprint() {
        let spec = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        let fingerprint = spec.managed_fingerprint("volicord");
        for repo_root in ["/workspace/alpha", "/workspace/beta"] {
            spec.materialize(ManagedMcpMaterializationInput::new(
                ManagedMcpInvocationPurpose::ManagedStdio,
                BTreeMap::from([(
                    VOLICORD_HOME_ENV.to_owned(),
                    OsString::from("/selected/runtime-home"),
                )]),
                ManagedMcpWorkingDirectory::ProductRepository(PathBuf::from(repo_root)),
            ))
            .expect("shared materialization");
            assert_eq!(spec.managed_fingerprint("volicord"), fingerprint);
        }
    }
}
