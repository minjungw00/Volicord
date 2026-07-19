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
`missing`, `invalid`, `unavailable`, `corrupt`, `not_checked`, or `unknown`; it
never combines those conditions. A
`project_policy_authority` finding uses
`authority_missing`, `authority_corrupt`, `authority_unavailable`,
`managed_file_missing`, `managed_file_invalid`, `managed_file_unavailable`, or
`managed_file_stale`.
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

When no matching current Agent Connection exists, `init` creates one in
`workflow` mode. When a matching current Agent Connection already exists,
`init` replay and repair preserve its exact `workflow` or `read_only` mode in
the host plan, verification expectations, and registration. They do not change
the integration generation. Only `volicord connection mode` performs a mode
transition and increments that generation.

Setup preserves unrelated Codex and repository content. Repair reruns the same
intent from current canonical inputs. The same `init` repair command repairs
owned Guard and Codex configuration in both modes. Removal deletes only
matching Volicord-managed content.

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

`volicord connection mode` treats Connection mode and every owned
project-scoped Guard manifest as one revision transition. Before mutation, the
CLI requires exactly one current, strictly valid Guard Installation for every
Connection Project and prepares each candidate manifest by replacing only its
Connection integration revision. Missing, duplicate, stale, malformed, or
owner-mismatched inventory fails with a repair instruction to rerun the owning
`volicord init` command. The Store then updates the Connection mode, increments
its integration generation once, clears the stored verification report, and
rebinds all candidate manifests in one Registry transaction. Any conflict or
write failure rolls back the whole transition, including multi-project
personal Connections.

Selecting the current mode is an exact no-op: it changes no Registry row,
timestamp, report, generation, manifest, host configuration, or Product
Repository file, and emits no reload action. A successful real transition also
does not rewrite host configuration or Product Repository files; it emits
exactly one `reload_host` action because the existing managed host must be
reloaded against the new revision. Prior runtime sessions, project Agent
Sessions, and Guard events remain historical and cannot satisfy current checks,
including when a later transition returns to a previously used mode.

When `volicord init` replaces the selected project's Connection, migration
retires that project's Registry project-session bindings and Guard Installation
before its superseded membership. For a superseded multi-project Connection,
that ordered retirement, the replacement membership and Guard Installation,
and replacement activation commit in one Registry transaction; the old
Connection, its other memberships and child rows, and connection-wide runtime
sessions remain. For a superseded last-project Connection, migration instead
disables the old Connection and retains its membership, bindings, Guard
Installation, and exact pending-host-cleanup marker until host cleanup succeeds.
Cleanup failure reports `partial_application` with that complete retry inventory
unchanged. After host cleanup, a final Registry transaction revalidates the
replacement, marker, and membership inventory, retires the old project-owned
rows and membership, and clears the marker. An already absent owned host entry
is a no-op, so replay can finish Registry cleanup without duplicating either
Connection's current rows.

`volicord connection remove` removes the selected Connection Project
membership and its project-scoped Registry bindings and Guard Installation in
one Store transaction. If other memberships remain, the Agent Connection,
connection-wide runtime sessions, other memberships, and their Registry rows
remain, and shared host configuration is not changed. For the last membership,
the CLI validates the plan, removes the matching managed host entry first, and
then commits the Registry transaction that removes the remaining bindings,
Guard Installations, runtime sessions, membership, and Agent Connection. An
absent owned host entry is a no-op so a retry can finish Registry cleanup.

Host-removal failure occurs before Registry mutation. A later Registry failure
rolls back the complete Registry transaction, leaving the membership and Agent
Connection selectable for retry even though host removal may already have
succeeded. `--dry-run` changes neither Registry state, host configuration, nor
Product Repository content. Removal output includes `membership_removed`,
`connection_removed`, and `remaining_project_count` so membership-only removal
is distinct from complete Agent Connection removal.

<a id="agent-connection-result-states"></a>
### Connection Result States

| State | Meaning |
|---|---|
| `complete` | The selected operation completed and every owner-required current check passed. |
| `action_required` | Durable setup may exist, but a named user or Codex action remains. |
| `failed` | The operation failed and reports a machine-readable cause. |

`complete` is not Core invocation authorization, executable attestation, or a
claim about behavior beyond the checks and observations in the report.

Every selected-Connection setup and lifecycle command serializes one
`ConnectionCommandReport`. This includes `volicord init` and the `add`,
`status`, `verify`, `mode`, and `remove` Connection commands:

```yaml
ConnectionCommandReport:
  operation: init | add | status | verify | mode | remove
  dry_run: bool
  status: complete | action_required | failed
  runtime_home: string
  connection:
    id: string
    host: codex
    scope: user | project
    profile: record
    mode: read_only | workflow
    repository: string
    config_target: string
  checks: ConnectionCheck[]
  actions: ConnectionAction[]
  result?: SetupResult | ModeTransitionResult | RemovalResult
  planned_changes?: PlannedConnectionChange[] # dry-run only
  limits: string[]

SetupResult:
  kind: setup
  applied: bool

ModeTransitionResult:
  kind: mode_transition
  changed: bool
  previous_mode: read_only | workflow
  current_mode: read_only | workflow
  previous_integration_revision: string
  current_integration_revision: string
  rebound_guard_installation_ids: string[]

RemovalResult:
  kind: removal
  membership_removed: bool
  connection_removed: bool
  remaining_project_count: integer

PlannedConnectionChange:
  kind: runtime_home_initialization | project_registration | managed_host_configuration | guard_managed_file | guard_registry_setup | mode_transition | connection_membership
  operation: create | update | remove | register | rebind | execute
  target: string
```

The report contains one aggregate status and one check/action tree. The optional
tagged `result` contains only operation-specific facts; it does not introduce a
second status. Setup reports use `kind=setup`, mode reports use
`kind=mode_transition`, and successful applied removal reports use
`kind=removal`. Status and verify normally omit `result`, and removal dry runs
omit an outcome that has not happened. The report does not
add `states`, a nested verification report or status, a Guard health tree,
host-gate or approval fields, a summary card, a primary action, or a second
disclosure tree. JSON omits optional fields when they do not apply; it does not
emit them as null-filled placeholders. `limits` carries the cooperative
assurance limitation once.

`SetupResult.applied` separates setup mutation from operational verification.
A successful init or add apply reports `applied=true` even when a later local
or operational check makes `status=failed`. A dry run reports `applied=false`,
includes `planned_changes`, and never serializes
`status=dry_run`. Its status is `action_required` when a planned change or host
action remains and otherwise `complete`.

If setup fails after only part of a migration was applied, it reports
`status=failed` and `applied=false`; the failed `setup_plan` check details state
the observed Registry transition, cleanup, prior-Connection dispositions, and
retry arguments. It does not report a second migration status or imply that an
unobserved step succeeded.

The command-report aggregate is `failed` when any required check failed,
`action_required` when no check failed and a required check is pending or a
typed action remains, and `complete` otherwise. This command aggregation lets a
completed mode transition report its passed transition check together with the
one current reload/use action without inventing a second transition status.

Planning assigns every planned change's closed ownership `kind`, typed
`operation`, and canonical target. It emits no no-op entry and sorts and
deduplicates entries by the stable spelling of `kind`, then `operation`, then
`target`. Managed-configuration and Guard checks use `kind`; changing a target
path cannot change a planned change's meaning. JSON includes the three fields
shown above. Human output renders each entry as
`kind=<kind> operation=<operation> target=<target>`.

`checks` and `actions` use the canonical
[`ConnectionVerificationReport`](agent-connection.md#connection-verification-report)
member types and ordering. JSON and human output render the same typed command
report. Human output may group checks but does not recompute status or actions.

Mode no-op reports `changed=false`, equal previous/current modes and revisions,
no rebound Guard Installation IDs, a passed `mode_transition` check, no action,
and `status=complete`. A real transition reports `changed=true`, the exact
previous/current modes and revisions, the rebound Guard Installation IDs, one
passed `mode_transition` check, exactly one current `reload_host` action, and
`status=action_required`.

Successful applied removal reports a passed `connection_removal` check,
`status=complete`, and exact `membership_removed`, `connection_removed`, and
`remaining_project_count` facts inside `RemovalResult`. A removal dry run has a
pending removal check and `apply_removal` action only when an actual removal is
planned; it reports that plan through typed `planned_changes` and performs no
mutation.

`volicord connection status` is read-only. It projects current managed
configuration, trust, Guard audit, integration revision, and managed-host
session observations together with the last active executable and MCP-server
probe. It neither launches a process nor changes files, timestamps, reports,
actions, observations, or database rows.

`volicord connection verify` actively discovers `codex`, runs the version
command, runs `volicord mcp --check`, and starts a CLI-only MCP self-test that
performs `initialize`, `tools/list`, required-tool validation, and a safe
read-only `volicord.list_projects` call. It then reads current managed-host
observations and persists exactly one canonical report. Before planning, it
captures the selected Connection's exact typed integration revision. Report
persistence compares that revision and replaces only the report in one
immediate Registry transaction. If the Connection changed while verification
was running, no stale report is stored and the command requires a rerun. The
observed Host Plan fingerprint remains diagnostic: verification does not apply
or adopt it and never changes `managed_fingerprint`. The CLI self-test is not a
managed-host session.

`volicord init` and `volicord connection add` keep a successfully written valid
setup even when a later operational check fails. They do not roll back managed
configuration because Codex is unavailable or the self-test fails. A fresh
valid setup with no managed-host observation is `action_required` and includes
the typed reload and first-use actions required to obtain those observations.
No Codex version or executable digest is an eligibility allowlist. These setup
commands apply or adopt host configuration before committing its managed
fingerprint, complete owner-coherent Registry and Guard state, derive the final
Connection revision, and only then verify and conditionally persist the report.

Actions are an ordered, deduplicated list derived from pending and failed
checks. Reload and first-use instructions state that actual Codex activity must
be observed. A passing `guard_files` check never produces an instruction to
reinstall Guard files.

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
humans and must not be parsed for automation. `complete`, `action_required`,
and every valid dry run exit `0`; a typed `failed` operational report exits
`1`; usage errors exit `2`. A failed JSON operational report is the only stdout
document and leaves stderr empty. A failed human operational report is rendered
on stdout. Unexpected runtime or serialization errors use stderr and exit `1`.
Exit selection uses the typed report status, never rendered text or reparsed
JSON.

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
