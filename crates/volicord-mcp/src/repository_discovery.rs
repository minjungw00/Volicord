use std::{collections::BTreeMap, error::Error, fmt};

/// Supported repository-visible MCP discovery host selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryDiscoveryHost {
    Codex,
}

impl RepositoryDiscoveryHost {
    /// Parses the public `--host` value used by repository discovery startup.
    pub fn parse(value: &str) -> Result<Self, RepositoryDiscoveryDescriptorError> {
        match value {
            "codex" => Ok(Self::Codex),
            _ => Err(RepositoryDiscoveryDescriptorError::new(
                "repository discovery host must be codex",
            )),
        }
    }

    /// Public CLI value stored in a portable repository projection.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
        }
    }

    /// Registry host-kind value resolved only from the local Runtime Home.
    pub const fn registry_host_kind(self) -> &'static str {
        match self {
            Self::Codex => "codex",
        }
    }
}

/// Exact clone-portable MCP process descriptor allowed in repository files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepositoryDiscoveryDescriptor {
    host: RepositoryDiscoveryHost,
}

impl RepositoryDiscoveryDescriptor {
    pub const COMMAND: &'static str = "volicord";
    pub const RUNTIME_HOME_ENV_VAR: &'static str = "VOLICORD_HOME";

    pub const fn new(host: RepositoryDiscoveryHost) -> Self {
        Self { host }
    }

    pub const fn host(self) -> RepositoryDiscoveryHost {
        self.host
    }

    pub fn args(self) -> Vec<String> {
        vec![
            "mcp".to_owned(),
            "--stdio".to_owned(),
            "--discover-repository".to_owned(),
            "--host".to_owned(),
            self.host.as_str().to_owned(),
        ]
    }

    /// Exact environment values stored in the host-native repository entry.
    pub fn env(self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Exact parent-environment names forwarded by the host-native entry.
    pub fn env_vars(self) -> Vec<String> {
        vec![Self::RUNTIME_HOME_ENV_VAR.to_owned()]
    }

    /// Validates the complete repository-visible process entry.
    ///
    /// The exact shape intentionally has no local Connection/project lifecycle coordinates,
    /// literal Runtime Home path, absolute executable, or unrelated environment
    /// fields. It carries only the host-native parent `VOLICORD_HOME` reference.
    pub fn validate_entry(
        self,
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
        env_vars: &[String],
    ) -> Result<(), RepositoryDiscoveryDescriptorError> {
        if command != Self::COMMAND {
            return Err(RepositoryDiscoveryDescriptorError::new(
                "repository discovery command must be the PATH-resolved volicord executable",
            ));
        }
        if args != self.args() {
            return Err(RepositoryDiscoveryDescriptorError::new(
                "repository discovery arguments must use the exact host-only discovery shape",
            ));
        }
        if env != &self.env() {
            return Err(RepositoryDiscoveryDescriptorError::new(
                "repository discovery environment values must use the exact host-native VOLICORD_HOME reference",
            ));
        }
        if env_vars != self.env_vars() {
            return Err(RepositoryDiscoveryDescriptorError::new(
                "repository discovery forwarded environment names must contain only VOLICORD_HOME in the host-native shape",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDiscoveryDescriptorError {
    message: String,
}

impl RepositoryDiscoveryDescriptorError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RepositoryDiscoveryDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RepositoryDiscoveryDescriptorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_descriptors_require_exact_host_native_runtime_home_forwarding() {
        let descriptor = RepositoryDiscoveryDescriptor::new(RepositoryDiscoveryHost::Codex);
        let command = RepositoryDiscoveryDescriptor::COMMAND;
        let args = descriptor.args();
        let env = descriptor.env();
        let env_vars = descriptor.env_vars();
        descriptor
            .validate_entry(command, &args, &env, &env_vars)
            .expect("canonical descriptor");

        let mut bound_args = args.clone();
        bound_args.extend(["--connection".to_owned(), "connection_local".to_owned()]);
        assert!(descriptor
            .validate_entry(command, &bound_args, &env, &env_vars)
            .is_err());

        let mut absolute_env = env.clone();
        absolute_env.insert("VOLICORD_HOME".to_owned(), "/local/runtime".to_owned());
        assert!(descriptor
            .validate_entry(command, &args, &absolute_env, &env_vars)
            .is_err());

        let mut secret_env = env.clone();
        secret_env.insert("API_TOKEN".to_owned(), "not-serialized".to_owned());
        assert!(descriptor
            .validate_entry(command, &args, &secret_env, &env_vars)
            .is_err());

        let mut injected_env_vars = env_vars.clone();
        injected_env_vars.push("API_TOKEN".to_owned());
        assert!(descriptor
            .validate_entry(command, &args, &env, &injected_env_vars)
            .is_err());

        assert!(descriptor
            .validate_entry("/absolute/volicord", &args, &env, &env_vars)
            .is_err());
    }

    #[test]
    fn portable_descriptor_environment_shapes_are_exact() {
        let codex = RepositoryDiscoveryDescriptor::new(RepositoryDiscoveryHost::Codex);
        assert!(codex.env().is_empty());
        assert_eq!(codex.env_vars(), ["VOLICORD_HOME"]);
    }
}
