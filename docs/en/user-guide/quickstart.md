# Quickstart

This tutorial connects one Codex installation to one Product Repository by
using the first-release managed setup. Complete [Installation](installation.md)
first so `volicord` is available on `PATH`.

## 1. Initialize The Connection

For a repository-shared connection:

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
```

Use the same command without `--shared` for a personal connection. `<repo>` is
the Git work tree where Codex will operate. The first release accepts only the
`codex` host, the `record` profile, and `personal` or `shared` scope.

Review the reported file changes before committing project-owned configuration.
Volicord-managed project files may include `.codex/config.toml`,
`.volicord/policy.json`, and the managed block in `AGENTS.md`.

## 2. Complete The Codex Action

Start or reload Codex in the selected repository. Complete any project-trust
step that Codex presents, then confirm that the active session can discover the
`volicord.*` tools. Configuration on disk does not by itself prove that an
already-running session loaded it.

## 3. Verify The Connection

Use the same intent selector that initialized the connection:

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

A `complete` verification result is a connection-readiness checkpoint. It does
not prove that Codex followed instructions, that repository writes are
sandboxed, or that a Task is ready to close. Follow any returned
`action_required` item and verify again.

## 4. Start Work

Begin with `volicord.status`. Follow the returned `next_action`; obtain a Write
Ticket before product-file writes, record work and evidence, and check close
readiness before completion.

If an agent creates a pending `UserActionRequest`, resolution belongs only to
the local CLI User Channel:

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

The MCP agent may create or resume the request and later observe its current
status, but it cannot resolve the request.

## Next Routes

- [Agent Host Setup](agent-host-setup.md) for personal/shared choices, preview,
  verification, repair, and removal.
- [Agent Host Troubleshooting](agent-host-troubleshooting.md) for bounded
  recovery.
- [Agent Workflow](agent-workflow.md) for the normal Core workflow.
- [Scope](../reference/scope.md) for the exact first-release boundary.
