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
```

Do not delete configuration, Runtime Home data, or the repository to clear a
diagnostic. Preserve JSON output when escalating a reproducible failure, but do
not include credentials or private payloads.

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

Rerun active verification with JSON output and find the `mcp_server` check:

```sh
volicord connection verify codex --repo "<repo>" --json
```

Inspect `details.self_test.failure.kind` and `.stage` before the bounded
`io_detail`, `protocol_detail`, or `stderr` context. An
`exited_before_response` failure also reports `exit_code`; `null` means no
numeric exit code was available. A timeout reports `timeout_ms`. When a text
capture has `truncated=true`, retain its deterministic truncation marker and
`omitted_bytes` count when sharing the diagnostic.

Treat stderr only as bounded context. Do not infer a machine reason from child
wording or copy credentials into a report. `missing_tools`, when present, came
from the structured tool-list response. The exact fields and stage-to-check-code
mapping are owned by [Administrative CLI](../reference/admin-cli.md).

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
