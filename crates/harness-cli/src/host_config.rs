use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::setup::SetupSurfaceBinding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedConfig {
    pub binding: SetupSurfaceBinding,
    pub output_path: Option<PathBuf>,
    pub value: Value,
}

pub fn setup_bindings(include_user_interaction: bool) -> Vec<SetupSurfaceBinding> {
    let mut bindings = vec![SetupSurfaceBinding::Agent];
    if include_user_interaction {
        bindings.push(SetupSurfaceBinding::UserInteraction);
    }
    bindings
}

pub fn binding_name(binding: SetupSurfaceBinding) -> &'static str {
    match binding {
        SetupSurfaceBinding::Agent => "agent",
        SetupSurfaceBinding::UserInteraction => "user_interaction",
    }
}

pub fn entry_name(binding: SetupSurfaceBinding) -> &'static str {
    match binding {
        SetupSurfaceBinding::Agent => "harness-agent",
        SetupSurfaceBinding::UserInteraction => "harness-user-interaction",
    }
}

pub fn config_file_name(binding: SetupSurfaceBinding) -> &'static str {
    match binding {
        SetupSurfaceBinding::Agent => "harness-agent.mcp.json",
        SetupSurfaceBinding::UserInteraction => "harness-user-interaction.mcp.json",
    }
}

pub fn render_config(
    binding: SetupSurfaceBinding,
    runtime_home: &Path,
    project_id: &str,
    mcp_command: &Path,
    output_path: Option<PathBuf>,
) -> GeneratedConfig {
    let mut env = Map::new();
    env.insert(
        "HARNESS_HOME".to_owned(),
        Value::String(path_text(runtime_home)),
    );
    env.insert(
        "HARNESS_PROJECT_ID".to_owned(),
        Value::String(project_id.to_owned()),
    );
    env.insert(
        "HARNESS_SURFACE_ID".to_owned(),
        Value::String(binding.surface_id().to_owned()),
    );
    env.insert(
        "HARNESS_SURFACE_INSTANCE_ID".to_owned(),
        Value::String(binding.surface_instance_id().to_owned()),
    );

    let mut server = Map::new();
    server.insert("command".to_owned(), Value::String(path_text(mcp_command)));
    server.insert("env".to_owned(), Value::Object(env));

    let mut servers = Map::new();
    servers.insert(entry_name(binding).to_owned(), Value::Object(server));

    let mut root = Map::new();
    root.insert("mcpServers".to_owned(), Value::Object(servers));

    GeneratedConfig {
        binding,
        output_path,
        value: Value::Object(root),
    }
}

pub fn render_configs(
    include_user_interaction: bool,
    runtime_home: &Path,
    project_id: &str,
    mcp_command: &Path,
    config_dir: Option<&Path>,
) -> Vec<GeneratedConfig> {
    setup_bindings(include_user_interaction)
        .into_iter()
        .map(|binding| {
            let output_path = config_dir.map(|dir| dir.join(config_file_name(binding)));
            render_config(binding, runtime_home, project_id, mcp_command, output_path)
        })
        .collect()
}

pub fn pretty_json(value: &Value) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}

pub fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::setup::{
        AGENT_SURFACE_ID, AGENT_SURFACE_INSTANCE_ID, USER_INTERACTION_SURFACE_ID,
        USER_INTERACTION_SURFACE_INSTANCE_ID,
    };

    #[test]
    fn agent_only_rendering_has_one_named_server() {
        let configs = render_configs(
            false,
            Path::new("/runtime"),
            "project_alpha",
            Path::new("/bin/harness-mcp"),
            None,
        );

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].binding, SetupSurfaceBinding::Agent);
        assert_eq!(
            configs[0].value,
            json!({
                "mcpServers": {
                    "harness-agent": {
                        "command": "/bin/harness-mcp",
                        "env": {
                            "HARNESS_HOME": "/runtime",
                            "HARNESS_PROJECT_ID": "project_alpha",
                            "HARNESS_SURFACE_ID": AGENT_SURFACE_ID,
                            "HARNESS_SURFACE_INSTANCE_ID": AGENT_SURFACE_INSTANCE_ID
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn optional_user_interaction_rendering_stays_separate() {
        let configs = render_configs(
            true,
            Path::new("/runtime"),
            "project_alpha",
            Path::new("/bin/harness-mcp"),
            Some(Path::new("/configs")),
        );

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].binding, SetupSurfaceBinding::Agent);
        assert_eq!(configs[1].binding, SetupSurfaceBinding::UserInteraction);
        assert_eq!(
            configs[0].output_path.as_deref(),
            Some(Path::new("/configs/harness-agent.mcp.json"))
        );
        assert_eq!(
            configs[1].output_path.as_deref(),
            Some(Path::new("/configs/harness-user-interaction.mcp.json"))
        );
        assert!(configs[0].value["mcpServers"]
            .as_object()
            .expect("servers object")
            .contains_key("harness-agent"));
        assert!(!configs[0].value["mcpServers"]
            .as_object()
            .expect("servers object")
            .contains_key("harness-user-interaction"));
        assert_eq!(
            configs[1].value["mcpServers"]["harness-user-interaction"]["env"]["HARNESS_SURFACE_ID"],
            USER_INTERACTION_SURFACE_ID
        );
        assert_eq!(
            configs[1].value["mcpServers"]["harness-user-interaction"]["env"]
                ["HARNESS_SURFACE_INSTANCE_ID"],
            USER_INTERACTION_SURFACE_INSTANCE_ID
        );
    }

    #[test]
    fn pretty_json_is_deterministic_and_parseable() {
        let config = render_config(
            SetupSurfaceBinding::Agent,
            Path::new("/runtime"),
            "project_alpha",
            Path::new("/bin/harness-mcp"),
            None,
        );

        let first = pretty_json(&config.value).expect("json should render");
        let second = pretty_json(&config.value).expect("json should render");

        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
        let parsed: Value = serde_json::from_str(&first).expect("pretty JSON should parse");
        assert_eq!(parsed, config.value);
    }
}
