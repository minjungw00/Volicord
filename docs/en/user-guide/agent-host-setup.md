# Agent Host Setup

Use this guide to install, verify, repair, or remove the managed Codex Agent
Connection. The exact command contract belongs to
[Administrative CLI](../reference/admin-cli.md), and the managed operational
session boundary belongs to [Agent Connection](../reference/agent-connection.md).

## Supported Setup

The first release has one managed host and one profile:

- `host_kind=codex`
- `profile=record`
- `scope=personal` or `scope=shared`
- new connections start in `workflow`; later setup preserves an established
  `workflow` or `read_only` mode
- managed `volicord mcp --stdio` transport

Create or repair a shared setup:

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
```

For a personal setup, omit `--shared`. Keep the same selector on later status,
verify, repair, and removal commands.

## Review Managed Changes

Before accepting setup, review the structured result and every managed file.
Project-owned configuration may include `.codex/config.toml`,
`.volicord/policy.json`, and a Volicord-managed `AGENTS.md` block. Setup must
not overwrite unrelated user content.

Use dry run for lower-level connection changes:

```sh
volicord connection add codex --repo "<repo>" --dry-run
volicord connection remove codex --repo "<repo>" --dry-run
```

## Verify

After Codex has loaded the configuration and completed any trust action:

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

`verify` inspects the selected managed configuration and returns its current
three-state report. It does not issue runtime authorization. Authorization is
validated per MCP project call from the current Connection, membership, mode,
and authoritative managed runtime/project sessions.

For a direct process preflight, use the exact stored identifiers:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

Normal managed operation starts `volicord mcp --stdio` with the launch context
supplied by the generated Codex configuration. Its markers are cooperative
routing inputs, not credentials. Do not infer a personal Connection from cwd or
scan for a nearby repository.

## UserAction Boundary

An MCP agent may call `volicord.request_user_action` to create a pending request
or use its read-only resume operation. The human resolves it only through the
CLI inbox:

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

Guard prompt-related observations are diagnostic input only. They do not create
a UserAction resolution and never substitute for the explicit CLI command.

## Repair

Run `volicord doctor`, then rerun the same `init` command for the exact
connection intent. Review the diff again and restart or reload Codex when
reported. This repair works in both `workflow` and `read_only` mode and keeps
the established mode; use `volicord connection mode` when a mode transition is
intended. Repair must preserve unrelated configuration and product data.

## Remove

Preview first, then remove the same intent:

```sh
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
```

Removal deletes only Volicord-managed integration material named by the result.
That includes the selected membership's Registry bindings and Guard
Installation. The Agent Connection and its connection-wide runtime sessions
remain while another repository membership exists; the last membership removes
them after matching host configuration is removed. Project-local Agent
Sessions, Guard and workflow history, evidence, and other authority records are
retained. The Product Repository, other repositories, and unrelated Codex
configuration are preserved.

## Related Guides

- [Quickstart](quickstart.md)
- [Agent Host Troubleshooting](agent-host-troubleshooting.md)
- [Multi-Repository Agent Setup](multi-repository-agent-setup.md)
- [System Requirements](../reference/system-requirements.md)
