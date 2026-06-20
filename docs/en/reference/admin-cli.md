# Administrative CLI reference

This document owns the local `harness` administrative and bootstrap CLI contract. The CLI initializes a `Harness Runtime Home`, registers local projects, and registers local surfaces. These commands are not public Harness API methods.

It does not define public API method behavior, API schemas, access-class value meanings, storage record layout, security guarantees, Core authority semantics, or MCP stdio transport behavior.

## Owns / does not own

This document owns:

- `harness` command names, command-line arguments, defaults, stdout/stderr routing, and process exit codes
- Runtime Home path selection for `harness` administrative commands
- administrative project and surface registration defaults
- local MCP setup orchestration, optional interactive setup frontend, setup option defaults, conflict handling, dry-run behavior, preview and cancellation guarantees, storage preparation and migration sequencing, post-preparation revalidation, project-ID validation, generated-configuration path validation, partial-failure reporting, output formats, and host-neutral configuration generation
- local registration profile expansion for `baseline-workflow`
- the boundary between administrative commands and public Harness API methods

This document does not own:

- public Harness API methods; see [API Methods](api/methods.md)
- API value meanings for `access_class` values; see [API Value Sets](api/schema-value-sets.md#access-class-values)
- surface registration meaning, verified surface context, actor provenance, and capability declaration boundaries; see [Agent Integration](agent-integration.md)
- runtime data boundary meaning; see [Runtime Boundaries](runtime-boundaries.md)
- MCP process startup, stdio framing, wire behavior, response wrapping, preflight internals, and shutdown; see [MCP Transport](mcp-transport.md)
- external MCP host installation schemas or host-specific configuration locations
- storage record layout, SQLite DDL, general storage migration definitions, Core authority semantics, and security guarantee meanings

## Command model

`harness` is a local administrative/bootstrap executable. It is not a long-running server and does not expose the public Harness API directly.

Supported baseline commands:

```text
harness --help
harness --version
harness setup local-mcp [OPTIONS]
harness init [--runtime-home-id ID]
harness project register --project-id ID --repo-root PATH [--status active]
harness project list
harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--interaction-role agent|user_interaction] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]
harness surface list --project-id ID
```

Exit and stream behavior:

- Successful commands write success output to stdout and exit with code `0`.
- `harness --version` writes `harness <version>` to stdout and does not require Runtime Home resolution.
- Usage errors write diagnostics to stderr and exit with code `2`.
- Runtime, environment, or storage errors write diagnostics to stderr and exit with code `1`.

Not supported:

- The CLI has no `serve`, `server`, or `connect` command.
- Administrative commands are not public Harness API methods and must not be added to the public method list.

## Runtime Home selection

The `harness` administrative CLI uses these Runtime Home path resolution rules. `harness-mcp` process environment and current MCP Runtime Home resolution are owned by [MCP Transport](mcp-transport.md#process-environment).

Resolution order:

1. A present but empty `HARNESS_HOME` is an error.
2. An absolute `HARNESS_HOME` is used as supplied.
3. A relative `HARNESS_HOME` is resolved against the process current working directory without requiring the path to exist.
4. When `HARNESS_HOME` is absent, use the first non-empty home source in this order: `HOME`, `USERPROFILE`, then `HOMEDRIVE` plus `HOMEPATH`.
5. Append `.harness` to the selected user home.
6. Resolve a relative selected home against the process current working directory.
7. Do not require canonicalization before `harness init`.

`harness init` may create or validate the selected Runtime Home registry. Other administrative commands require the selected Runtime Home to contain the records needed for the requested operation.

`harness setup local-mcp` adds a command-specific `--runtime-home` override. Its full selection order is defined in [Local MCP setup orchestration](#local-mcp-setup-orchestration).

## Local MCP setup orchestration

`harness setup local-mcp [OPTIONS]` is a local administrative orchestration command for the common Product Repository-root local MCP setup path. It supports the non-interactive command path and an optional interactive frontend. It preserves the lower-level `harness init`, `harness project register`, and `harness surface register` commands.

Supported options:

```text
--interactive
--runtime-home PATH
--repo-root PATH
--project-id ID
--with-user-interaction
--mcp-command PATH
--config-dir PATH
--output text|json
--dry-run
--replace-conflicting-surfaces
--overwrite-config
```

Boolean options are presence flags. Forms such as `--dry-run=true` are usage errors.

Defaults:

- `--output` defaults to `text`.
- `--interactive` is disabled unless present.
- User-interaction setup is disabled unless `--with-user-interaction` is present.
- The agent MCP surface target is `surface_id=agent_mcp`, `surface_instance_id=agent_mcp_local`, `surface_kind=mcp`, `interaction_role=agent`, with the `baseline-workflow` access set.
- The optional user-interaction MCP surface target is `surface_id=user_ui`, `surface_instance_id=user_ui_local`, `surface_kind=mcp`, `interaction_role=user_interaction`, with `read_status` and `core_mutation`.

Selection rules:

- Non-interactive setup requires `--repo-root`. Use `--repo-root .` as the explicit form for selecting the current `Product Repository`.
- Interactive setup prompts for `Product Repository` when `--repo-root` is absent.
- An explicit `--runtime-home` value for setup must be an absolute path. Omitting `--runtime-home` still uses `HARNESS_HOME` or the shared user-home fallback described in [Runtime Home setup selection](#runtime-home-setup-selection).
- The selected `Harness Runtime Home` and `Product Repository` must satisfy the path-separation contract owned by [Runtime Boundaries](runtime-boundaries.md).

### Interactive setup frontend

`--interactive` starts a text-only wizard for the same setup command. The wizard gathers or confirms setup inputs, shows the planned bindings and access classes, asks for destructive decisions before final confirmation, and then invokes the same setup planning and application path used by non-interactive execution. It is optional and must not become the only supported onboarding path.

Interactive mode rules:

- `--interactive` uses text output only.
- `--interactive --output json` is a usage error.
- `--interactive` may be combined with `--dry-run`.
- Explicit setup options seed the wizard defaults.
- The final plan is always shown before setup application or dry-run output.
- Before final affirmative confirmation, the wizard uses the same read-only planning path as dry-run. It does not initialize or migrate storage, run preflight, create configuration directories or files, register projects or surfaces, create application records, or change project `state_version`.
- Cancellation exits `0`, writes `setup: cancelled` to stdout, and performs no persistent setup change. The same no-persistent-change guarantee applies when the user declines a conflicting-surface replacement, declines configuration overwrite, declines the final plan, cancels the wizard, or uses interactive dry-run. This guarantee does not claim that screen output or ordinary process-local prompt state is unchanged.

Terminal and stream behavior:

- Interactive mode requires usable interactive terminal input checked through standard terminal detection at the binary boundary.
- When interactive input is unavailable, the command writes a usage diagnostic to stderr, suggests non-interactive flags, exits `2`, does not wait for input, and does not mutate state.
- Prompts, access reviews, conflict confirmations, and final confirmation are written to stderr.
- Normal final setup output is written to stdout through the existing text renderer.
- Interactive prompts must not be mixed into JSON output, and the wizard must not print secrets or raw unrelated environment values.

Wizard prompt order:

1. Runtime Home
2. Product Repository
3. project ID
4. agent binding and access review
5. user-interaction connector choice
6. configuration output location
7. conflict decisions when applicable
8. complete plan review
9. final confirmation

Prompt behavior:

- The Runtime Home prompt shows the path selected by setup precedence as the default. Empty input accepts the default. Entered setup override values must be absolute paths, and prompting does not create the path.
- The repository prompt shows an explicit `--repo-root` value as the default. Empty input accepts that default only when `--repo-root` was supplied. When `--repo-root` is absent, the prompt requires input and does not silently preselect the process current working directory. Entering `.` is the explicit current `Product Repository` form. Entered paths are validated and canonicalized with a retryable prompt when inaccessible or not a directory, without mutating state.
- After repository selection, the project prompt uses the setup planner to suggest one exact matching project ID when available, otherwise the final directory name when valid. It requires explicit input when no valid suggestion exists, surfaces ambiguity instead of choosing among several matches, and shows project-ID-to-repository conflicts clearly. Project rebinding remains unsupported.
- The agent binding review shows `surface_id=agent_mcp`, `surface_instance_id=agent_mcp_local`, `interaction_role=agent`, and the access classes `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, and `run_recording`. This list is registration input, not user identity, trust, or Core authority.
- The user-interaction connector prompt defaults to no. When selected, it shows a separate `surface_id=user_ui`, `surface_instance_id=user_ui_local`, `interaction_role=user_interaction` binding with `read_status` and `core_mutation`. The prompt explains that this is a separate connector binding, not an extension of the agent role; it is needed only when a real user-facing UI or connector submits the user action; `actor_kind=user` alone does not establish user authority; and its configuration remains separate from the agent configuration.
- The configuration prompt defaults to stdout-only unless `--config-dir` supplies a default. It may accept a configuration directory. It does not ask for or infer a third-party host settings path.

Conflict and final-confirmation behavior:

- Surface replacement confirmations use the structured conflicts produced by setup planning. For each incompatible target surface, the wizard shows the current and desired role, kind, and normalized access classes, then asks separately whether to replace that exact target surface. The default is no unless an explicit destructive flag seeds the proposed answer.
- Existing generated configuration files are shown by exact path and require a separate overwrite confirmation. The default is no unless `--overwrite-config` seeds the proposed answer.
- A general final confirmation is not sufficient authorization for a destructive surface replacement or configuration overwrite.
- Declining a required destructive action cancels setup before storage preparation, registration, preflight, or configuration-file creation.
- The final plan shows the Runtime Home, repository, project ID and action, each surface and action, MCP executable, preflight bindings, configuration destinations, dry-run status, and destructive updates. Final confirmation defaults to no.

With `--interactive --dry-run`, the wizard gathers and confirms the same inputs, shows the plan, performs no persistent setup change or migration, does not run preflight, and emits the normal dry-run output after final confirmation.

### Runtime Home setup selection

For `harness setup local-mcp`, Runtime Home selection order is:

1. absolute `--runtime-home`
2. `HARNESS_HOME`
3. the shared user-home fallback defined in [Runtime Home selection](#runtime-home-selection)

An explicit `--runtime-home` value is invalid when it is empty or relative. When `--runtime-home` is absent, `HARNESS_HOME` and the shared user-home fallback follow the shared Runtime Home resolution rules.

The final selected path is absolute and need not exist before setup.

The setup command initializes the Runtime Home when it is not already initialized. It preserves an existing valid Runtime Home registration.

### Project selection

Non-interactive setup requires `--repo-root`; the command does not infer the process current working directory. Interactive setup prompts for `Product Repository` when `--repo-root` is absent. `--repo-root .` is valid and explicitly selects the current `Product Repository`.

The selected `repo_root` must be an existing accessible directory and must be canonicalized before comparison.

Explicit and derived project IDs are validated before Runtime Home initialization, storage migration, project registration, surface registration, MCP preflight, or configuration-file creation. Invalid derived path-component IDs require an explicit valid `--project-id`.

Project ID validation rules:

- empty and whitespace-only values are invalid
- `.` and `..` are invalid
- `/`, `\`, and NUL are invalid
- setup does not silently trim, slugify, rewrite, or replace an invalid ID
- the same validation is shared by `harness setup local-mcp` and the lower-level project registration path
- setup does not define a broader character whitelist beyond the implemented path-component exclusions

When `--project-id` is present:

- use that project ID
- if that ID is unregistered, create it for the selected repository
- if that ID already points to the same canonical repository and is `active`, reuse it
- if that ID points to another repository, fail without changing the registration
- if that project is `inactive`, fail rather than silently activating it

When `--project-id` is absent:

1. Find projects whose canonical `repo_root` exactly matches the selected repository.
2. Reuse the project when exactly one match exists.
3. Fail as ambiguous when more than one match exists.
4. When no match exists, derive the project ID from the final repository directory name.
5. Require `--project-id` when no valid UTF-8 directory name can be obtained, including filesystem-root cases.
6. Fail when the derived ID is already registered to another repository.

No setup option may forcibly rebind an existing project ID to another repository.

Before setup reuses an existing project registration, the stored `Product Repository` must still satisfy the Runtime Home/Product Repository separation contract with the selected `Harness Runtime Home`. A legacy registration that violates that relationship fails before storage preparation, surface registration, MCP preflight, or configuration output. Setup does not repair, update, delete, or replace that registry row; supported recovery is to select a separate `Harness Runtime Home` and set up the `Product Repository` there.

### Surface compatibility and conflicts

For each target surface instance, setup:

- creates it when absent
- reuses it without writing when the existing registration is compatible
- reports a conflict when the existing registration differs
- replaces it only when `--replace-conflicting-surfaces` is present

Compatibility compares normalized meaning rather than raw JSON byte equality:

- exact target project, surface, and instance IDs
- `surface_kind`
- `interaction_role`
- normalized registered access-class set
- valid JSON object metadata required by MCP startup validation

Differences in unrelated display text or pre-existing non-authoritative setup metadata must not cause an authority change. A read-only existing agent surface must not be upgraded to `baseline-workflow` without `--replace-conflicting-surfaces`.

`--replace-conflicting-surfaces` applies only to the fixed target surface instances. It does not permit project rebinding or changes to public Harness authority rules.

### Preview inspection and historical schemas

Setup planning and dry-run inspect existing Runtime Home databases through a read-only, no-migration path. Supported historical schemas may be inspected without migration and classified internally as requiring migration for a later real setup path. This inspection does not create authority, change registration meaning, or apply storage repair.

Unsupported, inconsistent, corrupt, or unsafe-to-inspect schemas fail planning without repair or modification. Read-only inspection normalizes historical registration meaning only where an existing migration defines an unambiguous default; for example, older surface rows without stored `interaction_role` are inspected as `agent` for compatibility analysis without changing the database.

Detailed storage migration semantics belong to [Storage Versioning](storage-versioning.md). Storage record families and record-layout ownership belong to [Storage Records](storage-records.md).

### Real approved setup sequence

A non-dry-run setup path reaches mutation only after non-interactive execution or final affirmative interactive confirmation. The sequence is:

```text
read-only planning
-> execution approval
-> required recognized storage initialization or migration
-> refreshed planning and conflict validation
-> project and surface registration
-> MCP preflight
-> generated configuration output
```

Rules:

- migrations occur only on a real approved setup path
- a pre-migration plan is not applied blindly
- the plan is rebuilt after storage preparation or migration
- newly observed conflicts stop project and surface registration
- a completed migration is not rolled back merely because later revalidation, registration, preflight, or configuration output fails
- diagnostics identify completed storage preparation or migration where relevant
- the command remains rerunnable after partial failure
- there is no cross-database, cross-file, or cross-system rollback guarantee
- migration itself does not create Core authority, user identity, surface trust, or user-owned judgment

### Idempotency and partial failure

An exact repeated setup:

- does not duplicate Runtime Home, project, or surface records
- reports compatible records as `reused`
- does not rewrite reused project or surface metadata
- does not modify existing `Task` or Core workflow records
- does not increment project `state_version`
- generates deterministic host configuration

Before registration changes, the command detects currently observable deterministic input errors, registration conflicts, executable-discovery failures, configuration-rendering failures, and output-path structure conflicts. Missing non-interactive `--repo-root` and invalid explicit `--runtime-home` values fail before executable discovery. Project-ID validation and generated-configuration path structure validation occur before storage initialization or migration.

This pre-mutation validation is not a race-free or failure-free guarantee. External filesystem changes, permission changes, storage exhaustion, operating-system errors, MCP preflight failures, and other runtime failures may still occur after storage preparation or registration begins. File checks cannot eliminate time-of-check/time-of-use races. A later failure reports completed actions where relevant, generated destination files continue to use same-directory temporary-file replacement behavior, and no cross-system transaction is promised.

New records created by setup may use non-authoritative diagnostic metadata equivalent to:

```json
{
  "created_by": "harness_cli_setup",
  "setup_profile": "local_mcp_v1"
}
```

This metadata is preserved as ordinary registration metadata. It must not be interpreted as Core authority, user identity, surface trust, access grant, or proof of security properties. Compatible reused records keep their existing metadata.

### MCP executable and preflight

MCP executable discovery priority is:

1. `--mcp-command PATH`
2. a `harness-mcp` executable beside the running `harness` executable
3. `harness-mcp` discovered through `PATH`

The selected executable path written to generated host configuration must be absolute. Executable discovery and basic path validation must happen before registration writes.

The setup command must not add a dependency from the administrative CLI to the MCP adapter implementation. It invokes the selected executable as a separate process.

After applying registration, setup invokes the equivalent of `harness-mcp --check` with explicit environment bindings. The agent preflight must confirm:

```text
configuration: valid
interaction_role: agent
baseline_workflow_access: full
```

When `--with-user-interaction` is present, setup runs a separate preflight that confirms:

```text
configuration: valid
interaction_role: user_interaction
baseline_workflow_access: not_applicable
```

A failed preflight makes setup fail with exit code `1`. Setup must not write host configuration files after a failed preflight. Exact MCP preflight behavior remains owned by [MCP Transport](mcp-transport.md#configuration-preflight).

### Host-neutral configuration

Generated MCP configuration is host-neutral. The setup command must not guess or edit an unknown external host's configuration file.

Without `--config-dir`, text output must include a copyable host-neutral agent configuration equivalent to:

```json
{
  "mcpServers": {
    "harness-agent": {
      "command": "/absolute/path/to/harness-mcp",
      "env": {
        "HARNESS_HOME": "/absolute/path/to/runtime-home",
        "HARNESS_PROJECT_ID": "project-id",
        "HARNESS_SURFACE_ID": "agent_mcp",
        "HARNESS_SURFACE_INSTANCE_ID": "agent_mcp_local"
      }
    }
  }
}
```

When user interaction is requested, output or generate a separate configuration containing only the `harness-user-interaction` binding. Do not combine agent and user-interaction bindings into one generated file, and do not imply that an ordinary agent host should receive the user-interaction binding.

With `--config-dir PATH`, setup generates:

```text
harness-agent.mcp.json
harness-user-interaction.mcp.json
```

`harness-user-interaction.mcp.json` is generated only when `--with-user-interaction` is present.

Configuration directory rules:

- before storage preparation or registration, validate whether `--config-dir` is an existing directory or can be created beneath existing directory ancestors
- before storage preparation or registration, reject a non-directory or unsupported existing ancestor
- before storage preparation or registration, reject target paths that are directories or unsupported filesystem objects
- before storage preparation or registration, check whether target files already exist
- before storage preparation or registration, require explicit overwrite authorization for existing regular target files
- validate all requested agent and user-interaction targets before writing any target
- create the destination directory when needed
- write valid deterministic JSON
- use a same-directory replacement file rather than truncating a destination in place
- do not overwrite existing files by default
- require `--overwrite-config` to replace existing generated files
- treat `--overwrite-config` without `--config-dir` as a usage error
- dry-run performs the same structural checks without creating directories or files

The command must not claim atomic behavior stronger than the implementation can provide across supported platforms. At minimum, a partially written destination file must never be exposed as a completed configuration.

### Dry run

`--dry-run` performs:

- path resolution
- repository canonicalization
- project selection
- surface compatibility and conflict analysis
- read-only inspection of existing Runtime Home databases through the no-migration inspection path
- identification of supported historical schemas that may require migration on a later real setup path
- MCP executable discovery
- configuration rendering
- configuration-output structure and target-file conflict analysis

It does not:

- create a Runtime Home
- create or modify SQLite databases
- apply migrations
- change `schema_migrations`
- register or update a project
- register or update a surface
- create a configuration directory or file
- invoke `harness-mcp --check`
- create `Task` or application records
- change `state_version`

Dry-run output reports preflight as `planned`, not `passed`.

### Setup output

Text output must be human-readable and include at least:

```text
setup: complete|dry_run
runtime_home: ...
project_id: ...
repo_root: ...
agent_surface_id: agent_mcp
agent_surface_instance_id: agent_mcp_local
mcp_command: ...
preflight: passed|planned
```

It must identify each resource action using `created`, `reused`, `updated`, or `skipped`.

`--output json` writes exactly one valid JSON document to stdout and does not mix human explanation into stdout. Errors remain stderr diagnostics under the existing CLI exit-code model. JSON output is administrative CLI output, not a public Harness API response schema.

JSON success output has these top-level keys:

```text
status
runtime_home
project
surfaces
mcp_command
preflight
generated_configs
actions
warnings
```

Required JSON values:

- `status`: `complete` or `dry_run`
- project action: `created` or `reused`
- surface action: `created`, `reused`, or `updated`
- preflight status: `passed` or `planned`
- binding name: `agent` or `user_interaction`

Each `generated_configs` entry includes:

- binding name
- output path, or `null`
- the parsed JSON configuration object

### Usage errors and exit codes

Usage errors include:

- unknown option
- duplicate non-repeatable option
- missing option value
- unsupported output format
- `--overwrite-config` without `--config-dir`
- empty explicit path or ID
- incompatible boolean-value syntax

Exit behavior:

- success exits `0`
- runtime, storage, preflight, or conflict failure exits `1`
- usage failure exits `2`

### Setup administrative boundary

`harness setup local-mcp` is local administrative orchestration. It is not a public Harness API method and must not be added to the public method list.

The setup command:

- does not edit `Product Repository` files
- does not create a `Task`
- does not grant Core authority
- does not merge agent and user-interaction provenance
- does not install an unknown external MCP host
- does not change access-class meanings, public request or response schemas, storage DDL, security guarantees, or user-judgment authority rules

## Project registration

`harness project register --project-id ID --repo-root PATH [--status active]` registers a local `Product Repository` with the selected Runtime Home.

Rules:

- `--project-id` is required.
- `--repo-root` is required.
- `--status` defaults to `active`.
- Baseline registration accepts `status=active`.
- `--repo-root` identifies the local repository root for the project registration.

`harness project list` lists registered projects for the selected Runtime Home.

Runtime location boundaries, including the distinction between `Product Repository` and `Harness Runtime Home`, are owned by [Runtime Boundaries](runtime-boundaries.md).

## Surface registration

`harness surface register` records one local surface instance for a registered project.

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

- For an MCP process registration, use explicit `--kind mcp`.
- Use explicit `--surface-instance-id` when the registered surface instance will be referenced by `HARNESS_SURFACE_INSTANCE_ID`.

Access-class value names and meanings are owned by [API Value Sets](api/schema-value-sets.md#access-class-values). Surface registration meaning and verified context boundaries are owned by [Agent Integration](agent-integration.md).

## Surface listing

`harness surface list --project-id ID` lists registered surfaces for one project in the selected Runtime Home.

Rules:

- `--project-id` is required.
- Listing output is diagnostic registration information.
- Listing output does not grant authority, prove local reachability, or replace owner-returned verified surface context.

## Administrative boundary

The administrative CLI can initialize and register local resources. It does not create public Harness API methods and does not by itself create Core authority, `Write Authorization`, evidence sufficiency, close readiness, user-owned judgment, acceptance, residual-risk acceptance, artifact authority, or security guarantees.

Owner routes:

- Public method list and method routing: [API Methods](api/methods.md).
- Shared request and response schemas: [API Schema Core](api/schema-core.md).
- Access-class values: [API Value Sets](api/schema-value-sets.md#access-class-values).
- Surface and actor context meaning: [Agent Integration](agent-integration.md).
- Runtime location boundaries: [Runtime Boundaries](runtime-boundaries.md).
