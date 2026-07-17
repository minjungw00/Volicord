# Agent Host Setup

Use this guide to install, verify, repair, or remove the managed Codex Agent
Connection. The exact command contract belongs to
[Administrative CLI](../reference/admin-cli.md), and exact binding and receipt
shapes belong to [Agent Connection](../reference/agent-connection.md).

## Supported Setup

The first release has one managed host and one profile:

- `host_kind=codex`
- `profile=record`
- `scope=personal` or `scope=shared`
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

`verify` validates the selected managed binding and records the owner-defined
receipt only when all required facts are current. `status` is diagnostic and
does not upgrade missing evidence.

For a direct process preflight, use the exact stored identifiers:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

Normal managed operation starts `volicord mcp --stdio` with the binding supplied
by the generated Codex configuration. Do not infer a binding from cwd or scan
for a nearby repository.

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
reported. Repair must preserve unrelated configuration and product data.

## Remove

Preview first, then remove the same intent:

```sh
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
```

Removal deletes only Volicord-managed integration material named by the result.
It does not delete the Product Repository, Runtime Home authority records, or
unrelated Codex configuration.

## Related Guides

- [Quickstart](quickstart.md)
- [Agent Host Troubleshooting](agent-host-troubleshooting.md)
- [Multi-Repository Agent Setup](multi-repository-agent-setup.md)
- [System Requirements](../reference/system-requirements.md)
