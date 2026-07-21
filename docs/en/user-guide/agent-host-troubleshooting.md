# Agent Host Troubleshooting

Use this guide for bounded recovery of the first-release Codex `record`
connection. Keep the selected Product Repository, connection scope, and Runtime
Home explicit throughout recovery.

## Before Changing Anything

Collect read-only diagnostics first:

```sh
volicord doctor
volicord project current
volicord connection list
volicord connection status codex --repo "<repo>"
volicord connection status codex --repo "<repo>" --json
```

Do not delete configuration, Runtime Home data, or the repository to clear a
diagnostic. Preserve JSON output when escalating a reproducible failure, but do
not include credentials or private payloads.

## Use The Stable Finding Code

In `volicord doctor --json`, read `findings[].code` and
`findings[].actions[].code`. In Connection status or verification JSON, read
`root_cause_ids`, match those IDs in `findings`, and use the top-level
`actions[].code` and `actions[].root_cause_ids`. Retain the persisted finding ID,
runtime-session ID when present, and namespaced diagnostic code. Do not
classify a failure from the English summary, SQLite message, path wording, or
stderr excerpt.

Use the code family to choose the focused recovery:

| Code family | Recovery boundary |
|---|---|
| `platform.*` | Move to or restore observation of the supported platform cell. |
| `runtime_home.*` | Correct the absolute Runtime Home, initialize a missing Registry, repair permissions, or separate path boundaries as named by the action. |
| `installation.*` | Restore a runnable current Volicord build; use `action.installation.reinstall_current_build` when present. |
| `managed_config.*` | Run the same supported `init` repair. The finding never exposes static environment values or arguments. |
| `store.sqlite.busy`, `store.sqlite.locked` | Finish or stop the process holding the database transaction, then retry. |
| `store.schema.mismatch`, `store.integrity.corruption_failure` | Use a compatible build and an explicit owner-approved restore or reinitialization path. Do not edit schema tables in place. |
| `guard.*` | Repair the Guard installation, or trigger the exact unobserved phase named by the typed action. |
| `trust.repository.not_trusted` | Approve the exact Product Repository in Codex. |
| `revision.integration.stale` | Reload Codex after the already-applied configuration change. |
| `revision.observation.mismatch` | Run verification again against the current revision. |

Do not restart Codex merely for deterministic TOML drift, schema mismatch,
read-only storage, or Runtime Home permission failure. Repair that cause first.
`internal.unexpected_failure` means no narrower owner mapping was available; it
does not authorize guessing from prose.

## Inspect One Finding Or Runtime Session

Use the exact identifier from concise, verbose, or JSON output:

```sh
volicord diagnostics show "<finding-id>"
volicord diagnostics show "<finding-id>" --json
volicord diagnostics session "<runtime-session-id>"
volicord diagnostics session "<runtime-session-id>" --json
```

These are bounded Registry lookups. The human and JSON forms contain the same
root IDs and typed facts. A missing identifier returns a typed failed report
with `observation_state=absent`; it is not evidence that an empty finding or
empty session was observed. Check the selected Runtime Home and exact ID rather
than scanning SQLite or inventing an alternate identifier.

## Command Is Not Available

Confirm that the exact `volicord` binary is on the environment used to start
Codex. An already-running Codex process may retain an older `PATH`; restart it
after correcting the launch environment. Then rerun:

```sh
volicord doctor
volicord connection verify codex --repo "<repo>"
```

## Repository Or Connection Is Ambiguous

Run commands from the intended Git work tree or pass `--repo` explicitly. Use
`volicord project current` and `volicord connection list` to find the stored
identifiers. Never select a repository by scanning neighboring directories.

Keep the scope selector consistent. A shared connection must use `--shared` on
`init`, `status`, `verify`, and `remove`; a personal connection omits it.

## `action_required`

`action_required` is a structured next step, not an unexplained success or
fatal failure. Complete only the named Codex trust, reload, configuration, or
storage action, then rerun the same verification command.

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

Do not edit a verification result or infer readiness from configuration files.

## MCP Preflight Fails

Use the exact stored connection and project identifiers:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

A failure must identify the structural, binding, executable, storage, or
external-contract problem. Fix that problem and rerun preflight. Do not start a
different transport or bypass connection binding.

## MCP Self-Test Fails

Rerun active verification with JSON output and start with the root IDs:

```sh
volicord connection verify codex --repo "<repo>" --json
```

Match `root_cause_ids` to `findings[].id`, then inspect that finding's `code`,
typed `facts.data`, `causes`, correlations, and actions. The failed check still
retains stage-specific detail, including `details.self_test.diagnostic_code`,
`failure_stage`, and `finding_id` where applicable. Retain the finding ID when
inspecting or sharing bounded Registry facts such as exit code, timeout,
missing tools, or stderr excerpt.

For `mcp.protocol.unsupported_revision`, compare
`attempted_client_name`/`attempted_client_version`, `requested_revision`, and
`production_supported_revisions`. The ordinary concise output already shows
these values and the blocked `required_tools` and `tool_round_trip` checks. Use
`action.mcp.use_supported_protocol_revision`; do not replace it with a generic
inspection step. In verbose output, keep the requested, selected, and
negotiated revisions distinct, and keep actual MCP peer `clientInfo` distinct
from the PATH executable probe.

Treat stderr only as bounded context. Do not infer a machine reason from child
wording or copy credentials into a report. The stable `process.*`, `mcp.*`, and
`host.codex.*` code identifies the cause without downstream prose parsing. The
exact diagnostic-reference fields and process limits are owned by
[Administrative CLI](../reference/admin-cli.md); MCP code meanings and safe
negotiation facts are owned by [MCP Transport](../reference/mcp-transport.md).

If `actual_mcp_peer_client_info.version` differs from
`path_executable_probe.version`, first confirm which Codex process and PATH are
active. This warning is useful evidence but is not by itself a fatal result;
do not replace one version with the other when reporting it.

## Codex Loaded No Tools

Confirm that Codex trusts the exact project and has reloaded the current
`.codex/config.toml`. Check that the managed command points to the intended
`volicord` binary and Runtime Home. Then run read-only `volicord.status` from
the Codex tool list and perform administrative verification again.

Configuration presence does not prove active tool discovery. If the current
session still has no tools, preserve the diagnostics and start a fresh Codex
session on the same managed connection.

## Pending UserAction

The agent may create or resume a pending request but cannot answer it. Resolve
it only through the local CLI User Channel:

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

If the CLI rejects a stored request or resolution as corrupt, do not edit the
database or substitute a guessed answer. Preserve the machine-readable failure
and rebuild disposable development state when appropriate.

## Unrecorded Changes

An Unrecorded Change is a bounded observation, not actor attribution. Follow
the returned reconciliation action. Guard suppression may remove only
owner-defined matching paths; an `Unavailable` suppression outcome must remain
visible and must not be treated as an empty successful suppression.

## Repair Managed Configuration

Rerun the same supported setup intent:

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
volicord connection verify codex --shared --repo "<repo>"
```

Review every changed file. Repair must preserve unrelated Codex settings and
repository content.

## Partial Removal

Preview the exact intent and inspect the result:

```sh
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

Removal is successful only for the Volicord-managed paths named by the result.
Retained authority records or unrelated configuration are not partial failure
unless the command contract says they should have been removed.

## Security Boundary

Volicord is cooperative local authority state. A Write Ticket is not filesystem
permission, connection verification is not model-compliance proof, and Close
Status is not correctness, deployment, or human-review proof. See
[Security](../reference/security.md).
