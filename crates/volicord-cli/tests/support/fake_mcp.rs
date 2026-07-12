use std::{error::Error, fs, path::Path, path::PathBuf};

use serde_json::json;
use volicord_types::{
    ADAPTER_UTILITY_TOOL_NAMES, READ_ONLY_METHOD_TOOL_NAMES, RECONCILE_CHANGES_TOOL_NAME,
    WORKFLOW_METHOD_TOOL_NAMES,
};

use super::fake_hosts::{make_executable, shell_single_quoted};

#[cfg(unix)]
pub(crate) fn write_fake_mcp(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let workflow_tools = workflow_mcp_tool_names().collect::<Vec<_>>();
    write_fake_mcp_with_workflow_tools(dir, &workflow_tools)
}

#[cfg(unix)]
pub(crate) fn write_fake_mcp_missing_workflow_reconcile(
    dir: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let workflow_tools = workflow_mcp_tool_names()
        .filter(|tool| *tool != RECONCILE_CHANGES_TOOL_NAME)
        .collect::<Vec<_>>();
    write_fake_mcp_with_workflow_tools(dir, &workflow_tools)
}

#[cfg(unix)]
fn write_fake_mcp_with_workflow_tools(
    dir: &Path,
    workflow_tools: &[&str],
) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("volicord");
    let read_only_tools = read_only_mcp_tool_names().collect::<Vec<_>>();
    let workflow_response = shell_single_quoted(&fake_tools_list_response(workflow_tools));
    let read_only_response = shell_single_quoted(&fake_tools_list_response(&read_only_tools));
    let mut script = "#!/bin/sh\n\
         mode=\"${VOLICORD_TEST_CONNECTION_MODE:-read_only}\"\n\
         storage_read=\"${VOLICORD_TEST_STORAGE_READ:-passed}\"\n\
         storage_write=\"${VOLICORD_TEST_STORAGE_WRITE:-passed}\"\n\
         effective_tool_mode=\"${VOLICORD_TEST_EFFECTIVE_TOOL_MODE:-}\"\n\
         if [ -z \"$effective_tool_mode\" ]; then\n\
         if [ \"$storage_read\" != \"passed\" ]; then effective_tool_mode=\"unavailable\";\n\
         elif [ \"$mode\" = \"read_only\" ]; then effective_tool_mode=\"read_only\";\n\
         elif [ \"$storage_write\" = \"passed\" ]; then effective_tool_mode=\"workflow\";\n\
         elif [ \"$storage_write\" = \"readonly\" ]; then effective_tool_mode=\"read_only_degraded\";\n\
         else effective_tool_mode=\"unavailable\"; fi\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--check\" ]; then\n\
         shift 2\n\
         if [ \"$1\" != \"--connection\" ]; then printf 'missing connection\\n' >&2; exit 2; fi\n\
         connection=\"$2\"\n\
         printf 'configuration: valid\\n'\n\
         printf 'transport: stdio\\n'\n\
         printf 'runtime_home: %s\\n' \"$VOLICORD_HOME\"\n\
         printf 'connection_id: %s\\n' \"$connection\"\n\
         printf 'mode: %s\\n' \"$mode\"\n\
         printf 'enabled: true\\n'\n\
         printf 'project_state_read: %s\\n' \"$storage_read\"\n\
         printf 'project_state_write: %s\\n' \"$storage_write\"\n\
         printf 'effective_tool_mode: %s\\n' \"$effective_tool_mode\"\n\
         printf 'allowed_projects: 1\\n'\n\
         printf 'available_projects: 1\\n'\n\
         printf 'verification_scope: startup_check_only\\n'\n\
         exit 0\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--stdio\" ] && { [ \"$3\" = \"--connection\" ] || [ \"$3\" = \"--discover-repository\" ]; }; then\n\
         while IFS= read -r line; do\n\
         case \"$line\" in\n\
         *'\"method\":\"initialize\"'*) printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{\"tools\":{}},\"serverInfo\":{\"name\":\"volicord-mcp\",\"version\":\"test\"},\"instructions\":\"Use Volicord.\"}}' ;;\n\
         *'\"method\":\"tools/list\"'*)\n\
         if [ \"$mode\" = \"workflow\" ]; then\n"
        .to_owned();
    script.push_str("         printf '%s\\n' ");
    script.push_str(&workflow_response);
    script.push_str(
        "\n\
         else\n",
    );
    script.push_str("         printf '%s\\n' ");
    script.push_str(&read_only_response);
    script.push_str(
        "\n\
         fi\n\
         exit 0 ;;\n\
         esac\n\
         done\n\
         exit 0\n\
         fi\n\
         printf 'unexpected invocation\\n' >&2\n\
         exit 2\n",
    );
    fs::write(&path, script)?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
pub(crate) fn write_basic_fake_mcp(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let path = dir.join("volicord");
    fs::write(
        &path,
        "#!/bin/sh\n\
         mode=\"${VOLICORD_TEST_CONNECTION_MODE:-read_only}\"\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--check\" ]; then\n\
         shift 2\n\
         if [ \"$1\" != \"--connection\" ]; then printf 'missing connection\\n' >&2; exit 2; fi\n\
         connection=\"$2\"\n\
         printf 'configuration: valid\\n'\n\
         printf 'transport: stdio\\n'\n\
         printf 'runtime_home: %s\\n' \"$VOLICORD_HOME\"\n\
         printf 'connection_id: %s\\n' \"$connection\"\n\
         printf 'mode: %s\\n' \"$mode\"\n\
         printf 'enabled: true\\n'\n\
         printf 'allowed_projects: 1\\n'\n\
         printf 'available_projects: 1\\n'\n\
         printf 'verification_scope: startup_check_only\\n'\n\
         exit 0\n\
         fi\n\
         if [ \"$1\" = \"mcp\" ] && [ \"$2\" = \"--stdio\" ] && { [ \"$3\" = \"--connection\" ] || [ \"$3\" = \"--discover-repository\" ]; }; then\n\
         while IFS= read -r line; do\n\
         case \"$line\" in\n\
         *'\"method\":\"initialize\"'*) printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{\"tools\":{}},\"serverInfo\":{\"name\":\"volicord-mcp\",\"version\":\"test\"},\"instructions\":\"Use Volicord.\"}}' ;;\n\
         *'\"method\":\"tools/list\"'*)\n\
         if [ \"$mode\" = \"workflow\" ]; then\n\
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.intake\"},{\"name\":\"volicord.update_scope\"},{\"name\":\"volicord.status\"},{\"name\":\"volicord.prepare_write\"},{\"name\":\"volicord.stage_artifact\"},{\"name\":\"volicord.record_run\"},{\"name\":\"volicord.request_user_judgment\"},{\"name\":\"volicord.check_close\"},{\"name\":\"volicord.close_task\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
         else\n\
         printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"volicord.status\"},{\"name\":\"volicord.check_close\"},{\"name\":\"volicord.list_projects\"}]}}'\n\
         fi\n\
         exit 0 ;;\n\
         esac\n\
         done\n\
         exit 0\n\
         fi\n\
         printf 'unexpected invocation\\n' >&2\n\
         exit 2\n",
    )?;
    make_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn workflow_mcp_tool_names() -> impl Iterator<Item = &'static str> {
    WORKFLOW_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

#[cfg(unix)]
fn read_only_mcp_tool_names() -> impl Iterator<Item = &'static str> {
    READ_ONLY_METHOD_TOOL_NAMES
        .iter()
        .chain(ADAPTER_UTILITY_TOOL_NAMES.iter())
        .copied()
}

#[cfg(unix)]
fn fake_tools_list_response(tool_names: &[&str]) -> String {
    let tools = tool_names
        .iter()
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": { "tools": tools },
    })
    .to_string()
}
