# Administrative CLI reference

This document owns the local `harness` administrative and bootstrap CLI contract. The CLI initializes a `Harness Runtime Home`, registers local projects, and registers local surfaces. These commands are not public Harness API methods.

It does not define public API method behavior, API schemas, access-class value meanings, storage record layout, security guarantees, Core authority semantics, or MCP stdio transport behavior.

## Owns / does not own

This document owns:

- `harness` command names, command-line arguments, defaults, stdout/stderr routing, and process exit codes
- Runtime Home path selection for `harness` administrative commands
- administrative project and surface registration defaults
- local registration profile expansion for `baseline-workflow`
- the boundary between administrative commands and public Harness API methods

This document does not own:

- public Harness API methods; see [API Methods](api/methods.md)
- API value meanings for `access_class` values; see [API Value Sets](api/schema-value-sets.md#access-class-values)
- surface registration meaning, verified surface context, actor provenance, and capability declaration boundaries; see [Agent Integration](agent-integration.md)
- runtime data boundary meaning; see [Runtime Boundaries](runtime-boundaries.md)
- MCP process startup, stdio framing, response wrapping, and shutdown; see [MCP Transport](mcp-transport.md)

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
