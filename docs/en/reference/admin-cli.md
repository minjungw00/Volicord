# Administrative CLI reference

This document owns the local `harness` administrative and bootstrap CLI contract. The CLI initializes a `Harness Runtime Home`, registers local projects and surfaces, manages Agent Integration Profiles, installs host configuration for supported coding-agent hosts, and verifies host integration state. These commands are not public Harness API methods.

It does not define public API method behavior, API schemas, access-class value meanings, storage record layout, security guarantees, Core authority semantics, or MCP stdio transport behavior.

## Owns / does not own

This document owns:

- `harness` command names, command-line arguments, defaults, stdout/stderr routing, and process exit codes
- Runtime Home path selection for `harness` administrative commands
- administrative project and surface registration defaults
- Agent Integration Profile command behavior
- integration project membership command behavior
- host installation, status, verification, and uninstall command behavior for Codex, Claude Code, and generic export
- setup result states, dry-run behavior, machine-readable output, and noninteractive approval behavior
- optional repository-guidance apply, status, and remove command behavior
- `baseline-workflow` local registration profile expansion
- the boundary between administrative commands and public Harness API methods

This document does not own:

- public Harness API methods; see [API Methods](api/methods.md)
- API value meanings for `access_class` values; see [API Value Sets](api/schema-value-sets.md#access-class-values)
- Agent Integration Profile, Host Installation, verified surface context, actor provenance, and capability declaration meanings; see [Agent Integration](agent-integration.md)
- runtime data boundary meaning and `Product Repository` file-boundary exceptions; see [Runtime Boundaries](runtime-boundaries.md)
- MCP process startup, stdio framing, wire behavior, response wrapping, preflight internals, and shutdown; see [MCP Transport](mcp-transport.md)
- storage record layout, SQLite DDL, general storage migration definitions, Core authority semantics, and security guarantee meanings

## Command model

`harness` is a local administrative/bootstrap executable. It is not a long-running server and does not expose the public Harness API directly.

Supported baseline commands:

```text
harness --help
harness --version
harness init [--runtime-home-id ID]
harness project register --project-id ID --repo-root PATH [--status active]
harness project list
harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--interaction-role agent|user_interaction] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]
harness surface list --project-id ID
harness agent install --host codex|claude-code|claude_code|generic --scope user|project|local|export --project-id ID [--repo-root PATH] [--integration-id ID] [--default-project-id ID] [--server-name NAME] [--surface-id ID] [--surface-instance-id ID] [--mcp-command PATH] [--runtime-home PATH] [--export-path PATH|--export-dir PATH] [--guidance none|codex|claude-code|claude_code|both] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]
harness agent project add --integration-id ID --project-id ID [--repo-root PATH] [--default] [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project remove --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project default set --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent project default clear --integration-id ID [--runtime-home PATH] [--output text|json] [--dry-run]
harness agent status --integration-id ID [--runtime-home PATH] [--output text|json]
harness agent verify --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json]
harness agent uninstall --integration-id ID [--installation-id ID] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]
harness agent guidance apply --integration-id ID --project-id ID --host codex|claude-code|claude_code [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--replace-managed]
harness agent guidance status --integration-id ID --project-id ID [--runtime-home PATH] [--output text|json]
harness agent guidance remove --integration-id ID --project-id ID [--host codex|claude-code|claude_code] [--runtime-home PATH] [--output text|json] [--dry-run] [--allow-repository-write] [--remove-managed]
```

Exit and stream behavior:

- Successful commands write success output to stdout and exit with code `0`.
- `action_required` is a successful administrative result and exits `0`.
- `partial_failure`, `failed`, runtime errors, storage errors, preflight failures, verification failures, and conflicts exit `1`.
- Usage errors write diagnostics to stderr and exit with code `2`.
- `harness --version` writes `harness <version>` to stdout and does not require Runtime Home resolution.
- `--output json` writes exactly one JSON document to stdout and does not mix human explanation into stdout.
- Errors remain stderr diagnostics under the existing CLI exit-code model.

Not supported:

- The CLI has no `serve`, `server`, or `connect` command.
- The public `harness agent` contract has no broad automatic-confirmation flag. Use the explicit approval and replacement flags this contract requires.
- Administrative commands are not public Harness API methods and must not be added to the public method list.

## Runtime Home selection

The `harness` administrative CLI uses these Runtime Home path resolution rules. `harness-mcp` process environment and current MCP Runtime Home resolution are owned by [MCP Transport](mcp-transport.md#process-environment).

Resolution order:

1. Command-specific `--runtime-home` when the command defines it.
2. `HARNESS_HOME`.
3. The first non-empty home source in this order: `HOME`, `USERPROFILE`, then `HOMEDRIVE` plus `HOMEPATH`, with `.harness` appended.

Rules:

- A present but empty `HARNESS_HOME` is an error.
- A command-specific `--runtime-home` value must be absolute when the command performs setup, installation, verification, or migration planning.
- A relative `HARNESS_HOME` is resolved against the process current working directory without requiring the path to exist.
- `harness init` may create or validate the selected Runtime Home registry.
- Other administrative commands require the selected Runtime Home to contain the records needed for the requested operation.

## Host and scope support

Supported host and scope values:

| `--host` | Supported `--scope` values | Baseline target |
|---|---|---|
| `codex` | `user`, `project` | User config is Codex user `config.toml`. Project config is `.codex/config.toml` in the associated `Product Repository`. |
| `claude_code` | `local`, `project`, `user` | Local and user config are Claude Code user-owned configuration targets. Project config is `.mcp.json` in the associated `Product Repository`. The CLI may accept `claude-code` as an alias, but stored records use `claude_code`. |
| `generic` | `export` | Export an explicit MCP configuration object without claiming direct installation. |

Scope rules:

- `project` and `local` scopes permit exactly the associated `Product Repository`.
- `user` scope may permit multiple explicitly added projects, but `harness agent install` still requires at least one explicit `--project-id`.
- `generic export` writes or prints only an explicit configuration export and does not create a Host Installation that claims host loading.
- Unsupported host/scope combinations are usage errors.

Host configuration shape:

- Codex installation writes an MCP server table equivalent to `[mcp_servers.<server_name>]` with `command`, `args = ["--integration", "<integration_id>"]`, and optional `env.HARNESS_HOME`.
- Claude Code installation writes an MCP server entry under `mcpServers.<server_name>` with `command`, `args`, and optional `env.HARNESS_HOME`.
- Generic export emits the same command, args, and environment values in a host-neutral JSON object.
- User and local scopes may use a discovered canonical `harness-mcp` executable path or an explicit valid absolute path.
- Project-scoped shared configuration must use the portable command `harness-mcp` and rely on `PATH` in the host environment. It must not embed a personal build path, home-directory path, or personal `HARNESS_HOME`.
- Generic export may emit an explicitly selected absolute command path, but exported configuration is still `action_required` until a user-managed host loads and verifies it.
- New baseline host configuration must not require `HARNESS_PROJECT_ID`, `HARNESS_SURFACE_ID`, or `HARNESS_SURFACE_INSTANCE_ID`.

Host trust boundary:

- Installing configuration is distinct from the host loading and exposing the MCP server.
- Codex project-scoped configuration may require Codex project trust before it loads.
- Claude Code project-scoped MCP configuration may require project MCP approval before it loads.
- Harness must not claim that host trust, project trust, project MCP approval, OAuth, or comparable user-controlled host actions can be bypassed.

## Agent setup result states

The agent command family uses these setup result states:

| State | Meaning |
|---|---|
| `complete` | Durable integration state exists, managed host configuration exists and matches its expected fingerprint, the host-specific loadability gate is satisfied, no required trust or approval action remains, integration preflight succeeds, MCP initialization succeeds, and `tools/list` succeeds with the required tools. |
| `action_required` | Durable integration state and host configuration are present, but host trust, project approval, OAuth, reload, restart, or a comparable user-controlled host action remains. |
| `partial_failure` | Some durable administrative action succeeded, but a later installation, verification, host target, rollback, or cleanup step failed. The result must identify applied, rolled-back, and residual effects and be rerunnable. |
| `failed` | The requested installation or verification did not establish usable durable integration state or host configuration. |

`dry_run` is an output status, not a setup result state.

A successful `harness-mcp --check --integration <integration_id>` alone must not be described as `complete` host integration. It is only startup validation for the MCP process.

Host-specific state rules:

- Codex project scope remains `action_required` while Codex project trust cannot be confirmed.
- Claude Code project scope remains `action_required` while project MCP approval is pending.
- Rejected, missing, changed, unavailable, and unknown host states must not become `complete`.
- Generic export remains `action_required` because Harness cannot prove that an external host loaded the exported configuration.

## `harness agent install`

`harness agent install` creates or reuses an Agent Integration Profile, explicitly allows the requested project, installs or exports host configuration, and verifies the result where the host can be checked.

Required options:

- `--host`
- `--scope`
- `--project-id`

Optional behavior:

- `--integration-id` selects an existing integration or the desired id for a new integration.
- `--default-project-id` sets the default and must name an allowed project.
- `--server-name` selects the host MCP server name. When omitted, the CLI derives a stable default from `integration_id`, prefixed with `harness-`, sanitized to ASCII letters, numbers, hyphen, and underscore, and shortened with a hash when needed.
- `--repo-root` validates the associated `Product Repository` for project/local scope when a host target writes there.
- `--surface-id` and `--surface-instance-id` select the integration surface binding. When omitted, the CLI generates stable opaque ids and reports them.
- `--mcp-command` selects the `harness-mcp` executable for scopes that permit an explicit command path. User and local scopes require an existing absolute path when specified. Project scope uses `harness-mcp` from `PATH`; generic export requires an absolute command path when the command is explicit.
- `--runtime-home` selects the Runtime Home path to write into host configuration as `HARNESS_HOME`.
- `--guidance none|codex|claude-code|claude_code|both` previews and applies optional `Product Repository` guidance for the selected project. Omitted or `none` writes no guidance, and noninteractive guidance writes still require `--allow-repository-write`.

Installation rules:

- The command must not grant access to every project in the Runtime Home.
- The command must register, reuse, or validate the integration surface for each allowed project before verification can be `complete`.
- A default project must be allowed.
- Project/local scopes fail if more than one project would be allowed.
- User scope may add more projects later through `harness agent project add`.
- Host configuration writes use managed ownership markers or an equivalent managed fingerprint.
- Existing unmanaged configuration for the same host target and server name is a conflict unless `--replace-managed` applies to a previously managed block with a matching ownership marker.
- Project-scoped host configuration writes require `--allow-repository-write` in noninteractive execution.
- `--dry-run` previews every storage and file action under the zero-write contract in [Dry run and machine-readable output](#dry-run).

Verification:

- Verification must attempt MCP initialization and `tools/list` discovery when the host can be launched from the installed configuration.
- If configuration is installed but host trust or approval prevents loading, the result is `action_required`, not `failed`.
- If `harness-mcp --check` passes but MCP initialization or tool discovery has not succeeded, the result cannot be `complete`.
- A direct Harness-spawned MCP handshake does not prove that Codex or Claude Code loaded, trusted, approved, or exposed the server.

## Integration project membership commands

`harness agent project add` adds one allowed project to an existing integration.

Rules:

- `--integration-id` and `--project-id` are required.
- The project must already be registered in the selected Runtime Home.
- Adding a project does not make inactive, invalid, or execution-ineligible projects available at execution time.
- `--default` sets the integration default to the added project.
- Adding a second project to a `project` or `local` scoped integration is a conflict.
- The command does not rewrite host configuration; access revocation and addition are registry changes.

`harness agent project remove` removes one allowed project from an existing integration.

Rules:

- Removing a project that is still `default_project_id` must fail until the default is cleared or changed.
- Removing the only project from an installed integration is allowed only when the command reports the integration as not executable until a project is added again.
- Removing membership does not delete project state, surface records, Core records, host configuration, or guidance files.

`harness agent project default set` sets the default project for an existing integration.

Rules:

- `--integration-id` and `--project-id` are required.
- The project must already be allowed for that integration.
- Repeating a set operation for the current default is idempotent.
- Setting a different already allowed project changes the default without rewriting host configuration.

`harness agent project default clear` clears the default project for an existing integration.

Rules:

- `--integration-id` is required.
- Clearing an already absent default is idempotent.
- A current default project cannot be removed until the default is changed or cleared.
- After the default is cleared, the final project membership may be removed.
- An integration with no allowed projects may remain stored, but it is not executable until a project is added again.

## Status and verification commands

`harness agent status` reports registry and host-inventory state without launching the host unless a host owner defines a cheap status check.

It reports at least:

- `integration_id`
- enabled state
- `surface_id`
- `surface_instance_id`
- allowed projects with availability and default status
- Host Installation records
- `last_verified_status`
- guidance status

`harness agent verify` refreshes verification state for one integration or one installation.

Selection rules:

- `harness agent verify --installation-id <id>` verifies exactly that Host Installation and fails if it belongs to another integration.
- Without `--installation-id`, verification selects every Host Installation associated with `--integration-id`.
- Each selected installation uses its own `host_kind`, `host_scope`, `config_target`, repository root, command, arguments, environment, managed fingerprint, and host-specific status checks.
- One installation's result must not overwrite another installation's verification state. Per-installation output must identify the `installation_id` and resulting `last_verified_status`.

Verification must check:

- integration exists and is enabled
- allowed projects are readable and classified as available or unavailable
- default project, when present, is allowed and available
- host configuration target exists and still matches the managed fingerprint, when direct installation owns a target
- `harness-mcp --check --integration <integration_id>` succeeds
- MCP initialization succeeds
- `tools/list` exposes the nine public Harness tools and `harness.list_projects`

Verification records one of `complete`, `action_required`, `partial_failure`, or `failed` into each selected Host Installation's `last_verified_status`.

Aggregate result status:

| Selected installation results | Aggregate command status |
|---|---|
| Every selected installation is `complete` | `complete` |
| At least one selected installation is `action_required`, and none is `partial_failure` or `failed` | `action_required` |
| At least one selected installation is `partial_failure`, and none is `failed` | `partial_failure` |
| At least one selected installation is `failed` | `failed` |

The aggregate status is never `complete` while any selected installation is not `complete`.

## Uninstall

`harness agent uninstall` removes Harness-managed host configuration and optionally disables or removes registry inventory for the integration.

Rules:

- Uninstall must preview managed file edits before applying them.
- It must remove only blocks, files, or entries with matching Harness ownership markers or managed fingerprints.
- It must not delete a `Product Repository`, project state, Core records, Runtime Home, artifact store, or unrelated host configuration.
- Project-scoped file edits require `--allow-repository-write` in noninteractive execution.
- `--remove-managed` is required for noninteractive removal of managed `Product Repository` guidance.
- If host files were already changed by the user, uninstall must report the conflict rather than removing unrelated content.

## Repository guidance commands

Repository guidance is optional. It is installed only after explicit user authorization and is not an enforcement mechanism.

Supported guidance targets:

- Codex: a Harness-managed block in `AGENTS.md`.
- Claude Code: a Harness-managed Markdown rule file under `.claude/rules/`.

Rules:

- `harness agent guidance apply` requires `--integration-id`, `--project-id`, `--host`, and `--allow-repository-write` in noninteractive execution.
- The command must preview the exact file path and managed content.
- The command must detect unmanaged conflicts and require `--replace-managed` only for matching previously managed content.
- Managed guidance must include ownership markers that identify Harness management and the integration.
- `harness agent guidance status` reads managed marker state and reports whether guidance is absent, present, changed, or conflicted.
- `harness agent guidance remove` removes only matching managed content and requires `--remove-managed` in noninteractive execution.
- Guidance must state that Harness MCP server instructions and repository guidance can help tool selection but cannot guarantee model behavior.

Exact `Product Repository` write boundaries belong to [Runtime Boundaries](runtime-boundaries.md#explicit-integration-files-in-product-repositories).

<a id="dry-run"></a>
## Dry run and machine-readable output

`--dry-run` performs planning, validation, conflict detection, host target rendering, and output shaping without persistent changes.

Dry-run does not:

- create a `Harness Runtime Home`
- create or modify SQLite databases
- create SQLite WAL or SHM files
- apply registry or project-state migrations
- register or update projects, surfaces, integrations, memberships, installations, or verification status rows
- create, modify, or remove host configuration files
- create, modify, or remove `Product Repository` files or directories, including guidance files
- create, modify, or remove generic export files
- invoke `harness-mcp --check`
- perform MCP initialization or tool discovery

When a selected Runtime Home has a schema version 1 registry, dry-run may inspect it without migration and may report that migration would occur during apply. It must not migrate the registry, create new registry tables, create project-state databases, or write migration metadata.

Text output must be human-readable and identify each resource action using `created`, `reused`, `updated`, `removed`, `skipped`, `conflict`, or `planned`.

<a id="setup-output"></a>
JSON success output has these top-level keys:

```text
status
runtime
project
integration
allowed_projects
installations
guidance
host
verification
actions
effects
action_required
warnings
```

Required JSON values:

- `status`: `complete`, `action_required`, `partial_failure`, `failed`, or `dry_run`
- `host_kind`: `codex`, `claude_code`, or `generic`
- `host_scope`: `user`, `project`, `local`, or `export`
- `last_verified_status`: `not_verified`, `complete`, `action_required`, `partial_failure`, or `failed`

JSON output is administrative CLI output, not a public Harness API response schema.

Partial-failure output:

- Human-readable text output must identify each applied effect, rolled-back effect, and residual effect.
- JSON output must expose the same facts in machine-readable entries.
- Each effect entry must include the target location or record identity, the effect classification, and enough detail to rerun or inspect the target.
- Each residual effect must include why rollback was not performed or why rollback failed, plus the recommended operator action.
- A generic statement such as `registry changes may remain` is insufficient unless paired with exact residual-effect entries.

<a id="noninteractive-approval-behavior"></a>
## Noninteractive approval behavior

Noninteractive commands must fail instead of prompting when explicit user authorization is missing.

Rules:

- `--allow-repository-write` is required for any command that writes, replaces, or removes project-scoped host configuration or repository guidance.
- `--replace-managed` applies only to Harness-managed content with matching ownership markers or managed fingerprints.
- `--remove-managed` applies only to safe removal of Harness-managed content.
- A broad shell approval, write approval, host trust decision, or sensitive-action approval is not a `Write Authorization` and does not substitute for the explicit administrative flag required by this CLI contract.
- Host trust, project trust, project MCP approval, OAuth, restart, or reload actions remain user-controlled host actions and cannot be supplied by the CLI.

## Project registration

`harness project register --project-id ID --repo-root PATH [--status active]` registers a local `Product Repository` with the selected Runtime Home.

Rules:

- `--project-id` is required.
- `--repo-root` is required.
- `--status` defaults to `active`.
- Baseline registration accepts `status=active`.
- `--repo-root` identifies the local repository root for the project registration.
- The selected Runtime Home and `--repo-root` must satisfy the [Runtime Home/Product Repository separation contract](runtime-boundaries.md#runtime-home-product-repository-separation) before registration is recorded.

`harness project list` lists registered projects for the selected Runtime Home.

`harness project list` is registry-level inspection. It may show a legacy project record that violates the Runtime Home/Product Repository separation contract for diagnosis. Listing visibility does not make that record eligible for project-state database access, surface administration, Core execution, setup reuse, or MCP startup.

Runtime location boundaries, including the distinction between `Product Repository` and `Harness Runtime Home`, are owned by [Runtime Boundaries](runtime-boundaries.md#runtime-home-product-repository-separation).

## Surface registration

`harness surface register` records one local surface instance for a registered project.

Surface registration and listing require the project registration to remain eligible under the Runtime Home/Product Repository separation contract owned by [Runtime Boundaries](runtime-boundaries.md#runtime-home-product-repository-separation).

Defaults:

- `surface_kind` defaults to `cli`.
- `interaction_role` defaults to `agent`.
- Default access is only `read_status`.
- Generated Runtime Home IDs and generated `surface_instance_id` values are implementation-generated opaque values.

Registration profile:

- `--profile baseline-workflow` must be explicitly selected.
- `baseline-workflow` expands to `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, and `run_recording`.
- Explicit and profile-derived access classes form a deterministic de-duplicated union.
- The `baseline-workflow` profile does not include `artifact_read`.

`user_interaction` constraints:

- `user_interaction` requires `core_mutation`.
- `user_interaction` may have only `read_status` and `core_mutation`.
- `baseline-workflow` is therefore invalid for a `user_interaction` surface.

MCP registration guidance:

- For a coding-agent MCP integration, prefer `harness agent install` because it creates or validates the integration profile, project membership, host installation, and per-project surface binding together.
- Low-level `harness surface register --kind mcp` remains available for explicit administrative repair or custom automation.

Access-class value names and meanings are owned by [API Value Sets](api/schema-value-sets.md#access-class-values). Surface registration meaning and verified context boundaries are owned by [Agent Integration](agent-integration.md).

## Surface listing

`harness surface list --project-id ID` lists registered surfaces for one project in the selected Runtime Home.

Rules:

- `--project-id` is required.
- Listing output is diagnostic registration information.
- Listing output does not grant authority, prove local reachability, or replace owner-returned verified surface context.

<a id="local-mcp-setup-orchestration"></a>
## Compatibility: `harness setup local-mcp`

`harness setup local-mcp` is a non-baseline compatibility command for legacy fixed-project MCP configuration. New setup examples and Host Installation records must use `harness agent install`.

<a id="interactive-setup-frontend"></a>

Compatibility rules:

- The command remains administrative orchestration, not a public Harness API method.
- Any interactive frontend for this command is compatibility UI for the same non-baseline legacy setup path.
- It may generate legacy fixed-project configuration only when explicitly invoked for compatibility.
- It must identify the result as compatibility output.
- It must not be used as the baseline model for direct Codex or Claude Code installation.

<a id="host-neutral-configuration"></a>
### Compatibility host-neutral configuration

Legacy host-neutral configuration fragments such as `harness-agent.mcp.json` and server names such as `harness-agent` are compatibility material only. They are not baseline required names.

## Administrative boundary

The administrative CLI can initialize and register local resources. It does not create public Harness API methods and does not by itself create Core authority, `Write Authorization`, evidence sufficiency, close readiness, user-owned judgment, acceptance, residual-risk acceptance, artifact authority, or security guarantees.

Owner routes:

- Public method list and method routing: [API Methods](api/methods.md).
- Shared request and response schemas: [API Schema Core](api/schema-core.md).
- Access-class values: [API Value Sets](api/schema-value-sets.md#access-class-values).
- Agent Integration Profile, project membership, surface and actor context meaning: [Agent Integration](agent-integration.md).
- MCP process behavior: [MCP Transport](mcp-transport.md).
- Runtime location and repository write boundaries: [Runtime Boundaries](runtime-boundaries.md).
