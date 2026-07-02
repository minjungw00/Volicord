#![forbid(unsafe_code)]

use std::{
    ffi::OsString,
    fmt,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use volicord_mcp::{
    generate_bearer_token, local_http_listen_is_container_wildcard, local_http_listen_is_loopback,
    LocalHttpListenScope, LocalHttpServerConfig, LocalHttpTokenSource,
};
use volicord_store::{
    agent_connections::{list_agent_connections, list_connection_projects},
    bootstrap::project_record_by_repo_root,
    runtime_home::{resolve_runtime_home, RuntimeHomeResolutionError},
    StoreError,
};
use volicord_types::ProjectId;

use crate::project_context::{resolve_repository_root, ProjectCommandError};

const DEFAULT_LOCAL_HTTP_LISTEN: &str = "127.0.0.1:8765";
const LOCAL_HTTP_TRANSPORT: &str = "local-http";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServeCommand {
    Help,
    Version,
    LocalHttp { config: LocalHttpServerConfig },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServeCommandError {
    Usage(String),
    Runtime(String),
}

impl ServeCommandError {
    fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

impl fmt::Display for ServeCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ServeCommandError {}

impl From<StoreError> for ServeCommandError {
    fn from(error: StoreError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<RuntimeHomeResolutionError> for ServeCommandError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::runtime(error.to_string())
    }
}

impl From<ProjectCommandError> for ServeCommandError {
    fn from(error: ProjectCommandError) -> Self {
        match error {
            ProjectCommandError::Usage(message) => Self::Usage(message),
            ProjectCommandError::Runtime(message) => Self::Runtime(message),
        }
    }
}

#[derive(Debug, Default)]
struct ServeOptions {
    transport: Option<String>,
    listen: Option<SocketAddr>,
    container_listen: Option<SocketAddr>,
    home: Option<PathBuf>,
    token: Option<String>,
    generate_token: bool,
    connection_id: Option<String>,
    project_paths: Vec<PathBuf>,
    allowed_origins: Vec<String>,
}

pub fn run_serve_command<F>(
    args: &[String],
    env_var: F,
    current_dir: &Path,
) -> Result<ServeCommand, ServeCommandError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match args {
        [] => return Err(ServeCommandError::usage(serve_usage())),
        [option] if option == "-h" || option == "--help" || option == "help" => {
            return Ok(ServeCommand::Help)
        }
        [option] if option == "-V" || option == "--version" => return Ok(ServeCommand::Version),
        _ => {}
    }

    let options = parse_serve_options(args)?;
    let transport = options
        .transport
        .as_deref()
        .ok_or_else(|| ServeCommandError::usage("--transport is required"))?;
    if transport != LOCAL_HTTP_TRANSPORT {
        return Err(ServeCommandError::usage(format!(
            "UNSUPPORTED_TRANSPORT: --transport must be {LOCAL_HTTP_TRANSPORT}"
        )));
    }

    let home_override = options.home.clone();
    let runtime_home = resolve_runtime_home(
        |name| {
            if name == "VOLICORD_HOME" {
                home_override
                    .as_ref()
                    .map(|path| path.as_os_str().to_owned())
                    .or_else(|| env_var(name))
            } else {
                env_var(name)
            }
        },
        current_dir,
    )?;
    let listen_scope = if options.container_listen.is_some() {
        LocalHttpListenScope::ContainerPublishedHostLoopback
    } else {
        LocalHttpListenScope::NativeLoopback
    };
    let listen_addr = options.listen.unwrap_or_else(|| {
        options.container_listen.unwrap_or_else(|| {
            DEFAULT_LOCAL_HTTP_LISTEN
                .parse()
                .expect("valid default listen")
        })
    });
    let project_allowlist = resolve_project_allowlist(&runtime_home, current_dir, &options)?;
    let connection_id = match options.connection_id {
        Some(connection_id) => connection_id,
        None => infer_connection_id(&runtime_home, &project_allowlist)?,
    };
    let (bearer_token, token_source) = match options.token {
        Some(token) => {
            if options.generate_token {
                return Err(ServeCommandError::usage(
                    "cannot combine --token and --generate-token",
                ));
            }
            (token, LocalHttpTokenSource::Supplied)
        }
        None => (
            generate_bearer_token()
                .map_err(|error| ServeCommandError::runtime(error.to_string()))?,
            LocalHttpTokenSource::Generated,
        ),
    };

    Ok(ServeCommand::LocalHttp {
        config: LocalHttpServerConfig {
            runtime_home,
            connection_id,
            listen_addr,
            listen_scope,
            bearer_token,
            token_source,
            project_allowlist,
            allowed_origins: options.allowed_origins,
        },
    })
}

pub fn serve_usage() -> String {
    "volicord serve --transport local-http [--listen 127.0.0.1:8765 | --container-listen 0.0.0.0:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token TOKEN | --generate-token] [--allow-origin ORIGIN]\nLocal HTTP transport is for Docker and localhost adapters only. It is not a public network API, SaaS endpoint, multi-user server, authentication service, remote API, or security boundary.\nUse --listen only with loopback addresses. Use --container-listen only inside a container with Docker host-loopback publishing.\n"
        .to_owned()
}

fn parse_serve_options(args: &[String]) -> Result<ServeOptions, ServeCommandError> {
    let mut options = ServeOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--transport" => {
                set_once_string(args, &mut index, &mut options.transport, "--transport")?;
            }
            "--listen" => {
                if options.container_listen.is_some() {
                    return Err(ServeCommandError::usage(
                        "cannot combine --listen and --container-listen",
                    ));
                }
                index += 1;
                let value = option_value(args, index, "--listen")?;
                let listen = value.parse::<SocketAddr>().map_err(|error| {
                    ServeCommandError::usage(format!("--listen must be host:port: {error}"))
                })?;
                if !local_http_listen_is_loopback(&listen) {
                    return Err(ServeCommandError::usage(format!(
                        "NONLOCAL_LISTEN_REJECTED: --listen {listen} is not allowed; local HTTP transport only supports 127.0.0.1 or [::1]"
                    )));
                }
                options.listen = Some(listen);
                index += 1;
            }
            "--container-listen" => {
                if options.listen.is_some() {
                    return Err(ServeCommandError::usage(
                        "cannot combine --listen and --container-listen",
                    ));
                }
                index += 1;
                let value = option_value(args, index, "--container-listen")?;
                let listen = value.parse::<SocketAddr>().map_err(|error| {
                    ServeCommandError::usage(format!(
                        "--container-listen must be host:port: {error}"
                    ))
                })?;
                if !local_http_listen_is_container_wildcard(&listen) || listen.port() == 0 {
                    return Err(ServeCommandError::usage(format!(
                        "CONTAINER_LISTEN_REJECTED: --container-listen {listen} is not allowed; use 0.0.0.0:<port> or [::]:<port> inside the container and publish only to host 127.0.0.1"
                    )));
                }
                options.container_listen = Some(listen);
                index += 1;
            }
            "--home" => {
                index += 1;
                let value = option_value(args, index, "--home")?;
                if options.home.is_some() {
                    return Err(ServeCommandError::usage(
                        "--home was supplied more than once",
                    ));
                }
                options.home = Some(PathBuf::from(value));
                index += 1;
            }
            "--token" => {
                set_once_string(args, &mut index, &mut options.token, "--token")?;
            }
            "--generate-token" => {
                if options.generate_token {
                    return Err(ServeCommandError::usage(
                        "--generate-token was supplied more than once",
                    ));
                }
                options.generate_token = true;
                index += 1;
            }
            "--connection" => {
                set_once_string(args, &mut index, &mut options.connection_id, "--connection")?;
            }
            "--project" => {
                index += 1;
                let value = option_value(args, index, "--project")?;
                options.project_paths.push(PathBuf::from(value));
                index += 1;
            }
            "--allow-origin" => {
                index += 1;
                let value = option_value(args, index, "--allow-origin")?;
                options.allowed_origins.push(value.to_owned());
                index += 1;
            }
            "-h" | "--help" | "help" | "-V" | "--version" => {
                return Err(ServeCommandError::usage(
                    "cannot combine volicord serve command-line modes",
                ));
            }
            option if option.starts_with('-') => {
                return Err(ServeCommandError::usage(format!(
                    "unknown option: {option}"
                )));
            }
            argument => {
                return Err(ServeCommandError::usage(format!(
                    "unexpected argument: {argument}"
                )));
            }
        }
    }

    Ok(options)
}

fn set_once_string(
    args: &[String],
    index: &mut usize,
    target: &mut Option<String>,
    option: &'static str,
) -> Result<(), ServeCommandError> {
    if target.is_some() {
        return Err(ServeCommandError::usage(format!(
            "{option} was supplied more than once"
        )));
    }
    *index += 1;
    let value = option_value(args, *index, option)?;
    *target = Some(value.to_owned());
    *index += 1;
    Ok(())
}

fn option_value<'a>(
    args: &'a [String],
    index: usize,
    option: &'static str,
) -> Result<&'a str, ServeCommandError> {
    let value = args
        .get(index)
        .ok_or_else(|| ServeCommandError::usage(format!("{option} requires a value")))?;
    if value.starts_with('-') {
        return Err(ServeCommandError::usage(format!(
            "{option} requires a value"
        )));
    }
    Ok(value)
}

fn resolve_project_allowlist(
    runtime_home: &Path,
    current_dir: &Path,
    options: &ServeOptions,
) -> Result<Vec<ProjectId>, ServeCommandError> {
    let mut project_ids = Vec::new();
    for project_path in &options.project_paths {
        let repo_root = resolve_repository_root(current_dir, Some(project_path.as_path()))?;
        let project = project_record_by_repo_root(runtime_home, &repo_root)?.ok_or_else(|| {
            ServeCommandError::runtime(format!(
                "PROJECT_NOT_REGISTERED: repository {} is not registered; run `volicord project use {}` first",
                repo_root.display(),
                repo_root.display()
            ))
        })?;
        let project_id = ProjectId::new(project.project_id);
        if !project_ids
            .iter()
            .any(|existing: &ProjectId| existing.as_str() == project_id.as_str())
        {
            project_ids.push(project_id);
        }
    }
    Ok(project_ids)
}

fn infer_connection_id(
    runtime_home: &Path,
    project_allowlist: &[ProjectId],
) -> Result<String, ServeCommandError> {
    let mut candidates = Vec::new();
    for connection in list_agent_connections(runtime_home)? {
        if !connection.enabled {
            continue;
        }
        let projects = list_connection_projects(runtime_home, &connection.connection_internal_id)?;
        if projects.is_empty() {
            continue;
        }
        let project_matches = project_allowlist.iter().all(|project_id| {
            projects
                .iter()
                .any(|project| project.project_id == project_id.as_str())
        });
        if project_matches {
            candidates.push(connection.connection_internal_id);
        }
    }

    match candidates.as_slice() {
        [connection_id] => Ok(connection_id.clone()),
        [] => Err(ServeCommandError::runtime(
            "CONNECTION_REQUIRED: no enabled Agent Connection matches the serve project allowlist; pass --connection",
        )),
        _ => Err(ServeCommandError::runtime(format!(
            "CONNECTION_AMBIGUOUS: multiple enabled Agent Connections match; pass --connection ({})",
            candidates.join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use volicord_test_support::core_fixtures::CoreFixture;

    use super::*;

    #[test]
    fn serve_local_http_generates_token_and_defaults_to_localhost(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("serve-command-generated-token")?;

        let command = run_serve_command(
            &["--transport".to_owned(), "local-http".to_owned()],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(fixture.runtime_home_path().as_os_str().to_owned())
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )?;

        let ServeCommand::LocalHttp { config } = command else {
            panic!("serve command should build HTTP server config");
        };
        assert_eq!(config.connection_id, fixture.connection_id());
        assert_eq!(config.listen_addr, "127.0.0.1:8765".parse()?);
        assert_eq!(config.listen_scope, LocalHttpListenScope::NativeLoopback);
        assert_eq!(config.token_source, LocalHttpTokenSource::Generated);
        assert!(!config.bearer_token.is_empty());
        Ok(())
    }

    #[test]
    fn serve_home_option_overrides_environment_home() -> Result<(), Box<dyn std::error::Error>> {
        let env_fixture = CoreFixture::new("serve-command-env-home")?;
        let home_fixture = CoreFixture::new("serve-command-option-home")?;

        let command = run_serve_command(
            &[
                "--transport".to_owned(),
                "local-http".to_owned(),
                "--home".to_owned(),
                home_fixture.runtime_home_path().display().to_string(),
                "--connection".to_owned(),
                home_fixture.connection_id().to_owned(),
                "--token".to_owned(),
                "token".to_owned(),
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(env_fixture.runtime_home_path().as_os_str().to_owned())
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )?;

        let ServeCommand::LocalHttp { config } = command else {
            panic!("serve command should build HTTP server config");
        };
        assert_eq!(config.runtime_home, home_fixture.runtime_home_path());
        assert_eq!(config.connection_id, home_fixture.connection_id());
        assert_eq!(config.token_source, LocalHttpTokenSource::Supplied);
        Ok(())
    }

    #[test]
    fn serve_help_describes_local_http_boundary() {
        let usage = serve_usage();

        assert!(usage.contains("volicord serve --transport local-http"));
        assert!(usage.contains("--container-listen 0.0.0.0:8765"));
        assert!(usage.contains("Docker and localhost adapters only"));
        assert!(usage.contains("not a public network API"));
        assert!(usage.contains("not a public network API, SaaS endpoint, multi-user server, authentication service, remote API, or security boundary"));
        assert!(usage.contains("Docker host-loopback publishing"));
        assert!(!usage.contains("-p 127.0.0.1:8765:8765"));
    }

    #[test]
    fn serve_rejects_unsupported_transport() {
        let error = run_serve_command(
            &["--transport".to_owned(), "stdio".to_owned()],
            |_| None,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect_err("unsupported transport should be a usage error");

        assert_eq!(
            error,
            ServeCommandError::Usage(
                "UNSUPPORTED_TRANSPORT: --transport must be local-http".to_owned()
            )
        );
    }

    #[test]
    fn serve_rejects_nonloopback_listen_addresses() {
        for listen_addr in ["0.0.0.0:8765", "[::]:8765", "192.0.2.10:8765"] {
            let error = run_serve_command(
                &[
                    "--transport".to_owned(),
                    "local-http".to_owned(),
                    "--listen".to_owned(),
                    listen_addr.to_owned(),
                ],
                |_| None,
                Path::new(env!("CARGO_MANIFEST_DIR")),
            )
            .expect_err("non-loopback listen address should be a usage error");

            assert!(
                error.to_string().contains("NONLOCAL_LISTEN_REJECTED"),
                "unexpected error for {listen_addr}: {error}"
            );
        }
    }

    #[test]
    fn serve_container_listen_requires_explicit_container_scope(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = CoreFixture::new("serve-command-container-listen")?;

        let command = run_serve_command(
            &[
                "--transport".to_owned(),
                "local-http".to_owned(),
                "--container-listen".to_owned(),
                "0.0.0.0:8765".to_owned(),
            ],
            |name| {
                if name == "VOLICORD_HOME" {
                    Some(fixture.runtime_home_path().as_os_str().to_owned())
                } else {
                    None
                }
            },
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )?;

        let ServeCommand::LocalHttp { config } = command else {
            panic!("serve command should build HTTP server config");
        };
        assert_eq!(config.listen_addr, "0.0.0.0:8765".parse()?);
        assert_eq!(
            config.listen_scope,
            LocalHttpListenScope::ContainerPublishedHostLoopback
        );
        Ok(())
    }

    #[test]
    fn serve_container_listen_rejects_arbitrary_or_ephemeral_addresses() {
        for listen_addr in ["127.0.0.1:8765", "192.0.2.10:8765", "0.0.0.0:0"] {
            let error = run_serve_command(
                &[
                    "--transport".to_owned(),
                    "local-http".to_owned(),
                    "--container-listen".to_owned(),
                    listen_addr.to_owned(),
                ],
                |_| None,
                Path::new(env!("CARGO_MANIFEST_DIR")),
            )
            .expect_err("unsupported container listen address should be a usage error");

            assert!(
                error.to_string().contains("CONTAINER_LISTEN_REJECTED"),
                "unexpected error for {listen_addr}: {error}"
            );
        }
    }

    #[test]
    fn serve_rejects_options_outside_public_local_http_surface() {
        for option in ["--allow-nonlocal-listen", "--host"] {
            let error = run_serve_command(
                &[
                    "--transport".to_owned(),
                    "local-http".to_owned(),
                    option.to_owned(),
                ],
                |_| None,
                Path::new(env!("CARGO_MANIFEST_DIR")),
            )
            .expect_err("unsupported serve option should be a usage error");

            assert!(
                error.to_string().contains("unknown option"),
                "unexpected error for {option}: {error}"
            );
        }
    }
}
