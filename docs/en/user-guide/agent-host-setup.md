# Agent Host Setup

Use this guide to configure, verify, change, or remove a Codex or Claude Code
Agent Connection. For the shortest first setup, start with
[Quickstart](quickstart.md).

Exact command behavior belongs to
[Administrative CLI](../reference/admin-cli.md). Exact Agent Connection and
runtime boundaries belong to
[Agent Connection](../reference/agent-connection.md) and
[Runtime Boundaries](../reference/runtime-boundaries.md).

## Ordinary Setup

Install `volicord`, then initialize the Product Repository connection:

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
```

Use `--host claude-code` for Claude Code. `<repo>` is the Git repository where
the agent will work.

The command creates or reuses the Runtime Home and installation profile,
registers the repository, creates the Agent Connection, and writes
project-scoped MCP configuration and guidance. Generated configuration starts
`volicord mcp --stdio` for the selected connection.

This ordinary path uses shared repository configuration. Start the selected
host with the same nonempty, absolute `VOLICORD_HOME` selected by init. The
generated entry forwards that host value and does not embed a machine-local
Runtime Home path.

Setup also writes files inside the Product Repository. Review them under the
repository's normal configuration policy:

| Host | Typical project files |
|---|---|
| Codex | `.codex/config.toml`, `.volicord/policy.json`, and a managed Volicord block in `AGENTS.md` |
| Claude Code | `.mcp.json`; Detective setup may also add `.claude/settings.json`, `.claude/rules/volicord.md`, and `.claude/hooks/` |

Commit these files only when the setup should be shared with other contributors
or automation. Product Repository configuration is separate from operational
data stored in the Volicord Runtime Home.

After init, complete the host-owned step named in `Next`:

- restart or reload the host
- trust the Codex project when asked
- approve the Claude Code project MCP entry when asked
- repair `PATH` or command availability when named

Then verify the connection:

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

Use `claude-code` instead of `codex` for Claude Code.

## What Verification Can Show

Connection verification has several layers. Do not infer a later layer from an
earlier one.

| Layer | Question it answers |
|---|---|
| Managed configuration | Does the selected host configuration match the connection Volicord manages? |
| Host trust or approval | Has the host reported a trust, approval, pending, or rejection state that the user controls? |
| CLI MCP check | Can the verification process start and communicate with the MCP process? |
| Active host exposure | Can the current host session see and call Volicord tools? |
| Storage capability | Can that MCP process read the registry and project state, and write project state when workflow tools need it? |

Read `Status`, `Checks`, `Next`, and `Diagnostics` in the default text output.
Use `--json` for full diagnostics or automation. Scripts must not parse the
compact text.

CLI MCP success does not prove active host exposure. Confirm tool availability
inside the current Codex or Claude Code session. If tools are absent, follow
[Agent Host Troubleshooting](agent-host-troubleshooting.md) before rewriting
configuration by hand.

JSON connection status also includes `states.host_feature_support`, with all
six host features reported independently. Read `implemented_unverified` as
“the adapter path exists but exact current live-host evidence is absent,” not
as a failure of generated configuration. Read `unsupported_by_host` as a known
missing host-owned surface, and `temporarily_unavailable` only as a current
verified path whose runtime prerequisite is down. Only `verified` supports a
current feature claim. `configured` and `configuration_verified` in the
separate final-output detail do not change those states. See
[Host Feature Support Is Not Verified](agent-host-troubleshooting.md#host-feature-support-is-not-verified)
for recovery and the [Agent Connection](../reference/agent-connection.md#host-feature-support-state)
for the exact contract.

## Codex

The ordinary project-scoped path is:

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
volicord connection verify codex --shared --repo "<repo>"
```

After init:

1. Open or restart Codex in the Product Repository.
2. Complete any project trust prompt.
3. Confirm that the current session exposes `volicord.*` tools.
4. Ask the host to call `volicord.list_projects`, then `volicord.status`.

If the terminal-side checks pass but the tools are missing, check the
environment that launched Codex, not only a terminal opened later:

```sh
command -v volicord
```

For an IDE, remote executor, or non-interactive run, inspect that host's MCP
startup environment and logs. See the
[Codex troubleshooting path](agent-host-troubleshooting.md#trusted-codex-project-but-host-runtime-is-not-observed).

Codex-owned tool approval settings can coexist with Volicord-managed MCP
configuration. Do not delete an approval overlay merely because it appears
under the `volicord` server entry. Use the
[configuration-drift troubleshooting path](agent-host-troubleshooting.md#codex-approval-overlay-reported-as-mcp-configuration-changed)
when verification reports a mismatch.

## Claude Code

The ordinary project-scoped path is:

```sh
volicord init --shared --host claude-code --repo "<repo>" --profile record
volicord connection verify claude-code --shared --repo "<repo>"
```

After init:

1. Open or restart Claude Code in the Product Repository.
2. Complete any project MCP approval.
3. Check the host's connection view:

   ```sh
   claude mcp list
   claude mcp get volicord
   ```

4. Check `/mcp` in the active Claude Code session.
5. Ask the host to call `volicord.list_projects`, then `volicord.status`.

Matching `.mcp.json` or `claude mcp get` output does not by itself prove active
tool exposure. Use the
[Claude Code troubleshooting path](agent-host-troubleshooting.md#claude-code-configuration-exists-but-tools-are-not-exposed)
when the current session still has no Volicord tools.

<a id="integration-profiles"></a>
## Integration Profiles

| Profile | Use it when | What it adds |
|---|---|---|
| Record profile (`record`) | You want the ordinary MCP workflow without host lifecycle hooks or a session watcher. | Cooperative workflow recording and project guidance. |
| Detective profile (`detective`) | The selected host, platform, and repository meet every owner-defined hook and watcher setup prerequisite. | Configured host-hook and watcher observation paths, including Unrecorded Change signals. |

Configuring or enabling these paths does not by itself establish current feature
support. Use `states.host_feature_support`; only `verified` supports a current
feature claim.

Neither profile provides an OS sandbox, network isolation, actor proof,
correctness proof, or full write prevention. Detective observations are signals;
they do not turn Write Tickets into filesystem enforcement.

On native Windows, use `--profile record`. Detective host-hook wrappers and the
session watcher are not supported there. Exact profile behavior and failure
conditions belong to
[Administrative CLI](../reference/admin-cli.md#agent-host-setup-and-init).

If Detective setup reports an unsafe or missing hook path, rerun Detective init
for the same host and repository rather than hand-editing generated wrappers:

```sh
volicord init --shared --host codex --repo "<repo>" --profile detective
```

Then complete any host restart, trust, or approval step and rerun verification.
For each diagnostic value, use
[Hook Path Or Wrapper Is Unsafe](agent-host-troubleshooting.md#guard-hook-path-or-wrapper-is-unsafe).

## Lower-Level Connection Choices

`volicord init` is the primary first-run path. Use `volicord connection add`
when you need to choose connection intent or mode directly.

### Connection Intent

| Intent | Command shape | Use |
|---|---|---|
| `personal` | `volicord connection add codex --repo "<repo>"` | Current-user host configuration. |
| `shared` | `volicord connection add codex --shared --repo "<repo>"` | Project-shared configuration in the Product Repository. |
| `global` | `volicord connection add claude-code --global --repo "<repo>"` | User-wide Claude Code configuration with explicit connected projects. |

Codex supports personal and shared intent. Claude Code supports personal,
shared, and global intent. `--shared` and `--global` cannot be combined.

For one host-level connection serving multiple repositories, use
[Multi-Repository Agent Setup](multi-repository-agent-setup.md).

### Connection Mode

Workflow mode is the default. Use read-only mode when the host should inspect
projects and status without workflow mutation tools:

```sh
volicord connection add codex --repo "<repo>" --read-only
volicord connection mode codex read-only --repo "<repo>"
volicord connection mode codex workflow --repo "<repo>"
```

A workflow connection can expose only read-compatible tools when the MCP host
can read project state but cannot write it. That is a storage-capability issue,
not a new connection mode. Use
[Read-Only Host Storage](agent-host-troubleshooting.md#read-only-host-storage)
for diagnosis.

## Preview And Inspect Changes

Preview managed configuration changes before applying them:

```sh
volicord connection add codex --repo "<repo>" --dry-run
volicord connection remove codex --repo "<repo>" --dry-run
```

Inspect existing connections with:

```sh
volicord connection list --repo "<repo>"
volicord connection status codex --repo "<repo>"
volicord connection verify codex --repo "<repo>"
```

If more than one connection matches the host and repository, add the intent
flag used when it was created, such as `--shared` or `--global`.

## Smoke Checks

Use a read-only smoke check before creating workflow state:

1. Run `volicord connection verify` for the selected host and repository.
2. In the active host, call `volicord.list_projects`.
3. Call `volicord.status` for the intended project.

This checks configuration, tool visibility, project selection, and readable
project state. It should not create a `Task`.

Use workflow mutation calls only when creating Volicord state is appropriate.
Exact public method sequences belong to the [API Methods](../reference/api/methods.md)
and their focused owners.

## Generic MCP Hosts

For a host Volicord does not manage, first create and enable an Agent Connection
for an accepted `HOST` value. Then configure the external host through its own
settings to start:

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

The external configuration remains user-managed, and the resulting process must
still pass MCP startup validation. Volicord does not claim that an arbitrary
host loaded or approved it. Exact process and project-selection
behavior belongs to [MCP Transport](../reference/mcp-transport.md).

## User Channel Boundary

An Agent Connection may create a focused user action but displays only its
request ID, `status=pending`, and `next_actor=user`. It neither receives the
canonical form nor records the user's resolution. Use the separately verified
User Channel host surface when one is shown; the stable CLI path lists the
stored form and then resolves it. For a choice form:

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

Evidence-observation forms instead use the displayed criterion or claim,
artifact IDs, summary, and optional contradicted flag. See
[User Workflow](user-workflow.md#use-evidence-without-replacing-judgment).

## Removal

Preview and remove the selected repository membership:

```sh
volicord connection remove codex --repo "<repo>" --dry-run
volicord connection remove codex --repo "<repo>"
```

Removal deletes only matching managed host configuration when ownership and
safety checks allow it. It does not delete the Product Repository, project
state, Volicord records, Evidence attachments, or unrelated host configuration.

## Troubleshooting Routes

| Symptom | Read |
|---|---|
| Executable or installation profile is missing | [Installation](installation.md) |
| Setup reports `action_required` or `failed` | [Agent Host Troubleshooting](agent-host-troubleshooting.md) |
| Exact command behavior is unclear | [Administrative CLI](../reference/admin-cli.md) |
| Runtime Home and Product Repository separation matters | [Runtime Boundaries](../reference/runtime-boundaries.md) |
