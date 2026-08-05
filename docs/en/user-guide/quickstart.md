# Quickstart

This tutorial connects one Codex installation to one Product Repository with
the supported managed setup. Complete [Installation](installation.md) first so
`volicord` is available on `PATH`.

## 1. Initialize The Connection

For a repository-shared connection:

```sh cli-example
volicord init --shared --host codex --repo "<repo>" --profile record
```

Use the same command without `--shared` for a personal connection. `<repo>` is
the Git work tree where Codex will operate. The supported selector sets are
host `codex`, profile `record`, and connection intent `personal` or `shared`.

Review the reported file changes before committing project-owned configuration.
Volicord-managed project files may include `.codex/config.toml`,
`.volicord/policy.json`, and the managed block in `AGENTS.md`.

## 2. Complete The Codex Action

Follow the reported `activation_plan.required_steps` in order. Before its final
status-read step, the plan may include starting or reloading Codex in the
selected repository, completing any project-trust action, reviewing the current
project hooks, and requesting the reported in-chat integration-verification
step in a new conversation. Confirm that the active session can discover the
`volicord.*` tools. Configuration on disk does not by itself prove that an
already-running session loaded it.

## 3. Check Connection Readiness

Use the same intent selector that initialized the connection:

```sh cli-example
volicord connection status codex --shared --repo "<repo>"
```

A `complete` current-status result is a connection-readiness checkpoint. It
does not prove that Codex followed instructions, that repository writes are
sandboxed, or that a Task is ready to close. For `action_required`, complete
the returned `activation_plan.required_steps`, then read status again.

### Optional Active Diagnostics

```sh cli-example
volicord connection verify codex --shared --repo "<repo>"
```

`verify` is optional and is not needed to read or explain current state. Use it
only for fresh executable, Store, protocol, or host probe evidence; it does not
replace managed-host, session, hook, or in-chat Guard evidence. See
[Administrative CLI](../reference/admin-cli.md#agent-connection-result-states)
for its exact effects.

## 4. Start Work

Begin with `volicord.status`. Follow the returned tagged
the required entry in `workflow.transition_catalog` and its exact refs and state version. A work Task
records shaping before it advances explicitly to implementation; obtain a Write
Ticket before product-file writes, record work and evidence, and use close
readiness only during an intentional close review after the work is ready.

If an agent creates a pending `UserActionRequest`, resolution belongs only to
the local CLI User Channel:

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

The MCP agent must create a current request before presenting an actionable
user-owned choice and may later observe its status, but it cannot resolve the
request. Chat text is not a User Channel resolution.

## Next Routes

- [Agent Host Setup](agent-host-setup.md) for personal/shared choices, preview,
  verification, repair, and removal.
- [Agent Host Troubleshooting](agent-host-troubleshooting.md) for bounded
  recovery.
- [Agent Workflow](agent-workflow.md) for the normal Core workflow.
- [Scope](../reference/scope.md) for the exact supported boundary.
