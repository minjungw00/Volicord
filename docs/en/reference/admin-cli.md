# Administrative CLI reference

This document owns the supported local administrative command surface. Public
Core API method behavior remains with [API Methods](api/methods.md); managed
stdio behavior remains with [MCP Transport](mcp-transport.md).

<a id="surface-stability"></a>
## Surface Stability

| Surface | Stability |
|---|---|
| Commands and selectors listed here | `stable` |
| Pre-1.0 additions that are not listed as stable commands | `beta` |
| Human-readable formatting and diagnostics prose | `diagnostic` |
| Generated Codex configuration details | `internal` |

Labels use [Documentation Policy](../maintain/documentation-policy.md#surface-stability-labels).

## Command Model

`volicord` is a local administrative/bootstrap executable, not a long-running
network service. The first release accepts only `host=codex`, `profile=record`,
and `personal` or `shared` connection scope.

Supported command groups are:

```text
volicord init
volicord status
volicord doctor
volicord diagnostics
volicord policy
volicord connection
volicord project
volicord mcp
volicord export authority-bundle
volicord changes reconcile
volicord inbox
```

Unknown commands, removed selectors, extra positionals, and conflicting options
are usage errors. Administrative command names are not public API method names.

<a id="doctor-diagnostic-states"></a>
## Doctor Diagnostic States

`volicord doctor --json` keeps missing, invalid, unavailable, corrupt, and stale
observations distinct. Its `states.installation_profile` value is `present`,
`missing`, `invalid`, `unavailable`, `corrupt`, `unsupported_contract`,
`not_checked`, or `unknown`; it never combines those conditions. A
`project_policy_authority` finding uses
`authority_missing`, `authority_corrupt`, `authority_unavailable`,
`authority_unsupported_contract`, `managed_file_missing`,
`managed_file_invalid`, `managed_file_unavailable`, or `managed_file_stale`.
`managed_file_stale` means both copies were individually valid but their
canonical fingerprints differed. Repair actions may be offered, but doctor
does not substitute a default policy or rewrite either authority copy. The
bounded project-policy audit reports `scan_state: complete` or
`scan_state: bounded_incomplete`; a bounded-incomplete audit is a warning and
can never be reported as passed even when its inspected page has no finding.

<a id="runtime-home-selection"></a>
## Runtime Home Selection

`--home PATH`, when accepted by a command, selects the exact Runtime Home.
Otherwise selection uses `VOLICORD_HOME` and then the platform default. Empty,
relative, malformed, or conflicting values fail before storage access. A
Product Repository is never used as a Runtime Home.

<a id="volicord-agent-install"></a>
<a id="agent-host-setup-and-init"></a>
## Codex Setup

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord init --shared --host codex --repo "<repo>" --profile record
```

The first command selects a personal connection; `--shared` selects the
project-owned shared connection. `init` plans, validates, and applies the exact
managed binding and reports any remaining Codex trust, reload, or verification
action. `--dry-run` performs no filesystem or storage mutation.

Setup preserves unrelated Codex and repository content. Repair reruns the same
intent from current canonical inputs. Removal deletes only matching
Volicord-managed content.

<a id="project-commands"></a>
## Project Commands

```text
volicord project use [PATH]
volicord project current
volicord project list
volicord project rename NAME [--repo PATH]
volicord project forget [PATH|NAME]
```

Project selection resolves registered canonical Git work trees. Ambiguous
selection fails; cwd and a display name do not silently create identity.

<a id="project-workflow-policy-commands"></a>
## Policy Commands

```text
volicord policy show --repo PATH
volicord policy validate --file PATH
volicord policy apply --repo PATH --file PATH
```

Validation has no effect. Apply uses the owner-defined plan and atomic commit
boundary; unknown fields and invalid values fail before commit.

## Agent Connection Commands

```text
volicord connection add [codex] [--repo PATH] [--shared] [--read-only] [--dry-run]
volicord connection list [--repo PATH]
volicord connection status [codex] [--repo PATH] [--shared]
volicord connection verify [codex] [--repo PATH] [--shared]
volicord connection mode [codex] workflow|read-only [--repo PATH] [--shared]
volicord connection remove [codex] [--repo PATH] [--shared] [--dry-run]
```

When the host is omitted, the command uses it only if the current context is
unambiguous. The only accepted explicit value is `codex`.

<a id="agent-connection-result-states"></a>
### Connection Result States

| State | Meaning |
|---|---|
| `complete` | The selected operation completed and every owner-required current check passed. |
| `action_required` | Durable setup may exist, but a named user or Codex action remains. |
| `failed` | The operation failed and reports a machine-readable cause. |

`complete` is not a release-cell pass, Core invocation authorization, host
attestation, or proof that an active Codex session exposed tools.

Connection verification serializes the canonical
[`ConnectionVerificationReport`](agent-connection.md#connection-verification-report).
Its check and action arrays are not accompanied by an independent connection
status or setup-action state. `--dry-run` is reported as operation mode or plan
context; it never adds `dry_run` to either closed status set.

<a id="external-host-configuration"></a>
## Managed Codex Configuration

A personal connection writes only user-owned managed Codex configuration. A
shared connection writes the supported project-owned Codex entry and forwards
`VOLICORD_HOME` without embedding a machine-local path. The exact managed-entry
markers, drift rules, repair, launch context, and uninstall boundary belong to
[Agent Connection](agent-connection.md). Configuration markers select the
cooperative launch path; they are not credentials or identity evidence.

## MCP Commands

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

These commands expose only managed stdio. Exact framing, lifecycle, tool lists,
and response projection belong to [MCP Transport](mcp-transport.md).

<a id="diagnostics"></a>
## Diagnostics

```text
volicord diagnostics session [--session SESSION_ID] [--json]
volicord diagnostics workflow-metrics --repo PATH --json
```

Diagnostics output is bounded, non-authority operability data. JSON reports
identify their local storage with `contract_id=volicord.sqlite.diagnostics`
and the exact `canonical_schema_digest` derived from the current diagnostics
SQL. They do not expose or dispatch on a numeric schema version. A diagnostics
read does not create storage, open project authority state, advance state
version, change evidence or assurance, change close readiness, or resolve a
UserAction.

<a id="authority-bundle-export"></a>
## Authority Bundle Export

```text
volicord export authority-bundle --output "<path>" --repo "<repo>"
```

Export writes the owner-defined authority bundle to a new or explicitly allowed
output path. It does not alter project authority state or create release
evidence.

## Change Reconciliation

```text
volicord changes reconcile --repo "<repo>"
```

Reconciliation projects the public `volicord.reconcile_changes` behavior into a
local administrative flow. Guard suppression failures remain explicit; an
`Unavailable` outcome is not rendered as an empty successful suppression.

<a id="user-channel-commands"></a>
## User Channel Commands

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

`inbox` is the only first-release UserAction resolution channel. It renders the
stored typed form on a local user-owned surface and submits one explicit choice
or evidence observation to `volicord.resolve_user_action`. An MCP agent may
create or resume a pending request but cannot run this resolution path.

Stored requests and resolutions are strict typed records. Corrupt, unknown,
mixed, or invalid stored values fail with the persisted-data failure taxonomy;
the CLI does not default, repair, or guess an answer.

## Output And Exit Status

`--json` writes exactly one JSON document to stdout. Default prose is for
humans and must not be parsed for automation. Success and `action_required`
exit `0`; runtime, storage, verification, and contract failures exit `1`;
usage errors exit `2`.

<a id="noninteractive-approval-behavior"></a>
## Noninteractive Behavior

Noninteractive execution never accepts project trust, resolves UserAction,
approves a sensitive operation, or answers a host-controlled prompt. It returns
the structured next action and leaves the decision to the user.

## Related Owners

- [Agent Connection](agent-connection.md)
- [MCP Transport](mcp-transport.md)
- [Runtime Boundaries](runtime-boundaries.md)
- [API User-Action Schemas](api/schema-user-action.md)
- [Storage Effects](storage-effects.md)
