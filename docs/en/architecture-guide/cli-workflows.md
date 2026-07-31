# CLI Workflows

This guide explains durable implementation flow for the supported administrative
CLI. Exact command behavior belongs to
[Administrative CLI](../reference/admin-cli.md).

## Ownership Map

| Stage | Implementation responsibility |
|---|---|
| declare and inspect | `volicord-command-model` owns the complete Clap tree, command DTOs, value enums, syntax validators, public/hidden classification, canonical synopses, command-path traversal, and typed canonical invocation builders derived from that tree |
| parse and normalize | the command model rejects unknown, missing, or conflicting input and produces command DTOs for `volicord-cli` |
| resolve context | Runtime Home, canonical Product Repository, project, and Agent Connection selection |
| plan | read-only inspection builds exact proposed file and Store changes |
| validate | managed configuration, Connection, session, storage, and policy checks |
| commit | one owner-defined atomic filesystem/Store boundary |
| project and render | `volicord-user-action-presentation` owns typed `Cli*` UserAction inbox models, their JSON Schemas, and command-model-backed resolution paths; `volicord-cli` renders terminal text directly from those models or serializes one typed JSON document |

Parsing and rendering must not become alternate Core or Store authorities.

## Codex Setup

`init` accepts only Codex, `record`, and personal/shared scope. The CLI resolves
current canonical inputs, asks the Codex adapter to build a
managed configuration, previews exact managed changes, and applies only after
all preconditions pass. Repair reuses that flow; remove deletes only matching
managed content.

One typed managed MCP launch contract owns the command, arguments, static and
forwarded environment bindings, personal/shared distinction, canonical
projection, and fingerprint inputs. A personal configuration binds the selected
absolute Runtime Home as static `VOLICORD_HOME`; a shared configuration forwards
only `VOLICORD_HOME` and remains clone-portable. The Codex adapter serializes
that contract as TOML, parses the managed entry back into it, and preserves only
the allowed tool-approval overlay. The platform filesystem boundary separately
classifies Linux or WSL2 and validates the target and filesystem. Core receives
only a current `ValidatedAgentSession` produced from Store-owned operational
records.

## Connection Verification

`init` and the selected-Connection `add`, `status`, `verify`, `mode`, and
`remove` flows build one typed command report whose checks and actions use the
canonical verification types. One optional tagged result owns setup,
mode-transition, or removal facts without creating another status tree. The
JSON and text renderers consume that report, and binary exit handling reads its
typed aggregate status. Rendering does not reconstruct a parallel state tree
or parse its own output. Connection list retains a focused list projection that
does not depend on the command-report state, but every membership summary is
produced by the same current evaluator as selected status.

`connection status` reads current files and Store observations without running
active probes or writing files, reports, observations, or timestamps. One
current-evaluation service owns the exact Runtime Home, Agent Connection,
selected Connection Project membership, current integration revision, and
caller-supplied evaluation timestamp. It assembles current managed
configuration, runtime-session, selected-project Guard, policy, trust, and
repository facts with eligible persisted active-verification evidence. The
persisted report is an evidence input, not an aggregate-current-state cache.
Registration, evidence, configuration, membership, project Store, Guard, and
revision acquisition failures remain closed typed unavailable results for the
command handler.

`connection list` creates one request-scoped evaluation context per
Connection, reuses Connection-level inputs where practical, and evaluates
filtered memberships independently with one invocation timestamp. The
repository filter runs before current evaluation. One unavailable membership
is rendered beside successful memberships; a Runtime Home Registry enumeration
failure still terminates the command. No context survives the invocation.

`connection verify` performs current adapter and managed-configuration
inspection, runs permitted local probes, reads actual managed-host and Guard
observations, and commits at most one report through the Store owner. Executable
path and version are diagnostic probe facts. Authoritative managed runtime and
project sessions are recorded only by managed MCP lifecycle handling; the CLI
self-test records `session_source=cli_preflight` and cannot authorize a
managed-host call. Command handlers select coordinates and output mode, consume
the typed evaluation result, and perform final presentation without rebuilding
checks or activation state.

## Project And Policy Workflows

Project commands resolve canonical registered Git work trees. Policy apply uses
plan, strict validation, and atomic commit. Neither command family infers
authority from a display name or repairs unknown stored values.

The CLI crate owns a semantics-neutral human-presentation vocabulary for
headlines, sections, fields, nested records, bullets, repeated collection
items, action hints, yes/no and none/count values, and compact or verbose
detail. It owns spacing, indentation, control-character-safe text, and the
single trailing newline, but it does not own project, policy, Core, Store, MCP,
Guard, host, or product semantics. Each command-specific projection chooses
which facts, labels, empty-state sentence, count, and action to provide.

`project current` and `project list` project Store-validated typed project
records directly into those human primitives. The list preserves the
canonical Store order and uses repeated records, so long values remain
complete on dedicated field lines. Their `--json` paths serialize the complete
typed project records separately; human rendering is never converted through
JSON. A command exposes verbose output only when it has a meaningful verbose
projection.

`policy show` reads one strictly decoded authoritative
`ProjectWorkflowPolicy` and builds one typed `PolicyShowReport`. The report
keeps Store authority, managed-file synchronization, active-Task escalation,
and repair action as separate facts. Compact and verbose command-specific
projections feed the neutral human primitives; JSON serializes the same report
with its complete nested policy. Managed-file comparison uses the canonical
policy fingerprint and remains read-only. `policy validate` similarly builds
one typed successful validation result and projects either conclusion-first
human text or that result's JSON.

## UserAction Workflow

`inbox` asks Core for adapter-neutral pending facts. `volicord-types` derives
the semantic `UserActionResolutionForm`; shared UserAction presentation
projects it into `CliUserActionInboxResponse`, typed channel availability, and
a tagged request-specific capture path. The available path's command is a typed
command-model invocation whose path and option spellings come from the same
Clap declaration that parses it. `inbox resolve` submits one stored choice or
evidence observation through `volicord.resolve_user_action`. Text rendering
consumes the typed CLI model directly, while `--json` serializes it once. The
MCP adapter can create or resume a request but cannot call this resolution path
or consume the CLI inbox presentation.

Guard prompt observations never become a CLI answer. Corrupt stored request or
resolution data fails with a persisted-data error rather than a default form.

## Reconciliation

`changes reconcile` routes through the public Core method and renders its
unresolved findings, resolution results, user-action route, Close Status, and
next action. Repository-observation unavailability remains a separate Guard
diagnostic and is not synthesized as a path finding. Exact observation and
resolution behavior belongs to
[Repository Observation](../reference/repository-observation.md) and
[`volicord.reconcile_changes`](../reference/api/method-reconcile-changes.md).

## Diagnostics And Output

`doctor`, status, and preflight collect read-only facts and report named next
actions. For connection-report commands, `dry_run` is an operation boolean and
the aggregate remains three-state. `--json` serializes the typed
result once. Human text, logs, and diagnostic metadata are not parsed back into
authority state.

## Boundaries

- `volicord-command-model` depends only on Clap. It does not depend on Core,
  Store, MCP, CLI rendering, Runtime Home implementation, or application
  services.
- `volicord-user-action-presentation` depends on the command model and shared
  types. It owns no Core policy, Store read, command execution, terminal
  rendering, or MCP envelope.
- `volicord-cli` depends on the command model, shared UserAction presentation,
  Core, and Store; none of those crates depends on `volicord-cli`.
- Codex-specific configuration remains in the adapter.
- No command starts a network transport.
- No noninteractive command supplies user judgment.
- Client and host version observations are diagnostics. A changed host version
  renews operational observation; managed-call authorization uses current
  authoritative session ownership and exact bindings.

## Related Routes

- [Source Map](source-map.md)
- [Request Lifecycle](request-lifecycle.md)
- [Agent Connection](../reference/agent-connection.md)
- [MCP Transport](../reference/mcp-transport.md)
- [Testing Strategy](testing-strategy.md)
