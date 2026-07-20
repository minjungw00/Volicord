use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use volicord_types::{HostKind, HostScope};

pub const VOLICORD_HOME_ENV: &str = "VOLICORD_HOME";
pub const VOLICORD_MCP_LAUNCH_ENV: &str = "VOLICORD_MCP_LAUNCH";
pub const VOLICORD_MCP_HOST_ENV: &str = "VOLICORD_MCP_HOST";
pub const VOLICORD_MCP_CONNECTION_ID_ENV: &str = "VOLICORD_MCP_CONNECTION_ID";
pub const VOLICORD_MCP_PROJECT_ID_ENV: &str = "VOLICORD_MCP_PROJECT_ID";
pub const VOLICORD_MCP_VERIFICATION_ENV: &str = "VOLICORD_MCP_VERIFICATION";
pub const MANAGED_MCP_LAUNCH_VALUE: &str = "managed_host";
pub const VOLICORD_MCP_VERIFICATION_VALUE: &str = "1";

pub const MANAGED_MCP_LAUNCH_ENVIRONMENT_NAMES: [&str; 5] = [
    VOLICORD_HOME_ENV,
    VOLICORD_MCP_LAUNCH_ENV,
    VOLICORD_MCP_HOST_ENV,
    VOLICORD_MCP_CONNECTION_ID_ENV,
    VOLICORD_MCP_PROJECT_ID_ENV,
];

pub const MANAGED_MCP_PROCESS_ENVIRONMENT_NAMES: [&str; 6] = [
    VOLICORD_HOME_ENV,
    VOLICORD_MCP_LAUNCH_ENV,
    VOLICORD_MCP_HOST_ENV,
    VOLICORD_MCP_CONNECTION_ID_ENV,
    VOLICORD_MCP_PROJECT_ID_ENV,
    VOLICORD_MCP_VERIFICATION_ENV,
];

const MCP_COMMAND: &str = "volicord";
const FINGERPRINT_DOMAIN: &[u8] = b"volicord.codex-managed-configuration\0";

pub fn is_managed_mcp_launch_environment_name(name: &str) -> bool {
    matches!(
        name,
        VOLICORD_HOME_ENV
            | VOLICORD_MCP_LAUNCH_ENV
            | VOLICORD_MCP_HOST_ENV
            | VOLICORD_MCP_CONNECTION_ID_ENV
            | VOLICORD_MCP_PROJECT_ID_ENV
    )
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
        project_id: Option<String>,
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

impl ManagedMcpLaunchSpec {
    pub const PATH_COMMAND: &'static str = MCP_COMMAND;

    pub fn personal(
        command: &Path,
        runtime_home: &Path,
        connection_id: impl Into<String>,
        project_id: Option<&str>,
    ) -> Result<Self, ManagedMcpLaunchError> {
        validate_personal_command(command)?;
        let command = command.to_str().ok_or_else(|| {
            ManagedMcpLaunchError::new(
                "personal managed MCP launch requires a UTF-8 executable path",
            )
        })?;
        let runtime_home = RuntimeHomeBinding::try_new(runtime_home)?;
        let connection_id = nonblank(connection_id.into(), "connection ID")?;
        let project_id = project_id
            .map(|value| nonblank(value.to_owned(), "project ID"))
            .transpose()?;

        let mut args = vec![
            "mcp".to_owned(),
            "--stdio".to_owned(),
            "--connection".to_owned(),
            connection_id.clone(),
        ];
        if let Some(project_id) = &project_id {
            args.extend(["--project".to_owned(), project_id.clone()]);
        }

        let mut static_values = BTreeMap::from([
            (
                VOLICORD_HOME_ENV.to_owned(),
                runtime_home.as_str().to_owned(),
            ),
            (
                VOLICORD_MCP_CONNECTION_ID_ENV.to_owned(),
                connection_id.clone(),
            ),
            (
                VOLICORD_MCP_HOST_ENV.to_owned(),
                HostKind::Codex.as_str().to_owned(),
            ),
            (
                VOLICORD_MCP_LAUNCH_ENV.to_owned(),
                MANAGED_MCP_LAUNCH_VALUE.to_owned(),
            ),
        ]);
        if let Some(project_id) = &project_id {
            static_values.insert(VOLICORD_MCP_PROJECT_ID_ENV.to_owned(), project_id.clone());
        }

        Ok(Self {
            command: command.to_owned(),
            args,
            environment: LaunchEnvironment::try_new(static_values, Vec::new())?,
            binding: ManagedMcpBinding::Personal {
                runtime_home,
                connection_id,
                project_id,
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
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--discover-repository".to_owned(),
                "--host".to_owned(),
                host_kind.as_str().to_owned(),
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

        let expected = if let [mcp, stdio, discover, host_flag, host] = candidate.args.as_slice() {
            if mcp != "mcp"
                || stdio != "--stdio"
                || discover != "--discover-repository"
                || host_flag != "--host"
            {
                return Err(ManagedMcpLaunchError::invalid_shape());
            }
            let host_kind = HostKind::from_str(host)
                .map_err(|_| ManagedMcpLaunchError::new("managed MCP launch host must be codex"))?;
            Self::shared_repository(host_kind)?
        } else if matches!(candidate.args.len(), 4 | 6)
            && candidate.args[0] == "mcp"
            && candidate.args[1] == "--stdio"
            && candidate.args[2] == "--connection"
        {
            let connection_id = candidate.args[3].as_str();
            let project_id = if candidate.args.len() == 6 && candidate.args[4] == "--project" {
                Some(candidate.args[5].as_str())
            } else if candidate.args.len() == 4 {
                None
            } else {
                return Err(ManagedMcpLaunchError::invalid_shape());
            };
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
                project_id,
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
    use super::*;

    fn personal() -> ManagedMcpLaunchSpec {
        ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
            None,
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
                "args": ["mcp", "--stdio", "--connection", "connection_alpha"],
                "env": {
                    "VOLICORD_HOME": "/srv/volicord/runtime",
                    "VOLICORD_MCP_CONNECTION_ID": "connection_alpha",
                    "VOLICORD_MCP_HOST": "codex",
                    "VOLICORD_MCP_LAUNCH": "managed_host"
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
    fn project_bound_personal_launch_extends_only_the_owned_binding() {
        let spec = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
            Some("project_alpha"),
        )
        .expect("project-bound personal launch");
        assert_eq!(
            spec.args(),
            [
                "mcp",
                "--stdio",
                "--connection",
                "connection_alpha",
                "--project",
                "project_alpha"
            ]
        );
        assert_eq!(
            spec.environment()
                .static_values()
                .get(VOLICORD_MCP_PROJECT_ID_ENV)
                .map(String::as_str),
            Some("project_alpha")
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
                "args": ["mcp", "--stdio", "--discover-repository", "--host", "codex"],
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

        let project = ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord/bin/volicord"),
            Path::new("/srv/volicord/runtime"),
            "connection_alpha",
            Some("project_alpha"),
        )
        .expect("project-bound launch");
        assert_ne!(
            first.managed_fingerprint("volicord"),
            project.managed_fingerprint("volicord")
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
            None,
        )
        .is_err());
        assert!(ManagedMcpLaunchSpec::personal(
            Path::new("/opt/volicord"),
            Path::new("relative/runtime"),
            "connection_alpha",
            None,
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
                None,
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
    fn strict_projection_rejects_mixed_personal_and_shared_shapes() {
        let shared = ManagedMcpLaunchSpec::shared_repository(HostKind::Codex)
            .expect("shared repository launch");
        let mut shared_env = shared.environment().static_values().clone();
        shared_env.insert(
            VOLICORD_MCP_CONNECTION_ID_ENV.to_owned(),
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
                "mcp".to_owned(),
                "--stdio".to_owned(),
                "--discover-repository".to_owned(),
                "--host".to_owned(),
                "codex".to_owned(),
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
}
