use std::{collections::BTreeMap, path::Path};

use serde_json::{json, Value};
use volicord_types::IntegrationProfile;

use crate::{
    guard_integration::{
        files::VOLICORD_POLICY_SCHEMA, hooks::GuardCommandSpec, public_host_label,
    },
    host_integration::{HostKind, HostLifecyclePhase, ManagedServerEntry, REQUIRED_GUARD_PHASES},
};

pub(crate) fn policy_json(
    host_kind: HostKind,
    profile: IntegrationProfile,
    repo_root: &Path,
    connection_id: &str,
    guard_installation_id: &str,
    mcp_entry: &ManagedServerEntry,
    guard_commands: &BTreeMap<String, GuardCommandSpec>,
) -> Value {
    let commands = guard_commands
        .iter()
        .map(|(phase, spec)| {
            (
                phase.clone(),
                json!({
                    "command": &spec.command,
                    "args": &spec.args,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "schema": VOLICORD_POLICY_SCHEMA,
        "managed_by": "volicord",
        "host": public_host_label(host_kind),
        "repo_root": path_text(repo_root),
        "connection_id": connection_id,
        "guard_installation_id": guard_installation_id,
        "selected_profile": profile.as_str(),
        "mcp": {
            "command": &mcp_entry.command,
            "args": &mcp_entry.args,
            "env": &mcp_entry.env,
        },
        "host_hook": {
            "enabled": profile != IntegrationProfile::Record,
            "commands": commands,
        },
    })
}

pub(crate) fn required_guard_phase_names() -> Vec<&'static str> {
    REQUIRED_GUARD_PHASES
        .iter()
        .map(|phase| phase.capability_name())
        .collect()
}

pub(crate) fn lifecycle_phase_names(phases: &[HostLifecyclePhase]) -> Vec<&'static str> {
    phases.iter().map(|phase| phase.capability_name()).collect()
}

pub(crate) fn guard_has_prompt_capture_commands(policy: &Value) -> bool {
    policy
        .get("host_hook")
        .and_then(|guard| guard.get("commands"))
        .and_then(|commands| commands.get("prompt_capture"))
        .is_some()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
