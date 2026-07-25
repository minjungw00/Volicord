# Guard Recorded-Change Suppression

This document owns the canonical outcome used when Guard determines whether
already recorded product-file changes can be removed from a later observed-path
set. It defines the exact `SuppressionOutcome` variants, scan budget, failure
reasons, conservative fallback, diagnostics, and non-blocking Guard projection.

This logic is independent of the source that observed the paths. An adapter may
provide an observed path set, but the suppression service receives canonical
project paths and canonical correlation records only.

## Surface Stability

The outcome variants, fields, unavailable reasons, no-hidden-path invariant,
and scan-budget behavior are stable. Query layout, cache strategy, and helper
module placement are internal.

## Canonical Types

```yaml
SuppressionOutcome:
  Applied:
    outcome: applied
    remaining_paths: string[]
    suppressions: RecordedChangeSuppression[]
  Unavailable:
    outcome: unavailable
    remaining_paths: string[]
    reason: SuppressionUnavailableReason
    scan_budget: integer
    observed_count: integer

RecordedChangeSuppression:
  paths: string[]
  guard_event_id: string
  write_ticket_id: string
  run_id: string
  path_identity_digest: string

SuppressionUnavailableReason:
  event_window_exceeded
  store_read_failed
  stored_event_corrupt
  correlation_payload_invalid
  run_lookup_failed
  write_ticket_lookup_failed
  path_identity_failed
```

`remaining_paths` and every `paths` list contain normalized Product Repository
paths, are bytewise sorted and duplicate-free, and never contain an absolute
path. `path_identity_digest` is lowercase 64-hex SHA-256 of the canonical path
identity used for the comparison; it is not a Git object ID or an authority
grant.

`Applied` means every eligible correlation needed by the bounded scan was read
and validated. An empty `suppressions` array is a valid successful outcome: no
observed path had a proven unchanged identity backed by the required Guard
event, write ticket, and Run.

One `RecordedChangeSuppression` is valid only when all named records exist in
the same project and correlation, the recorded Run consumed the named write
ticket, its canonical observed paths equal `paths`, and the current canonical
path identity equals the recorded digest. Partial path overlap does not
suppress either the overlap or the rest.

## Bounded Scan

The current scan budget is exactly 512 eligible prior Guard events. The Store
query observes at most 513 candidates so it can distinguish exactly 512 from
more than 512. The budget is a resource bound, not permission to silently
truncate a successful result.

- zero through 512 eligible candidates may produce `Applied`;
- 513 observed candidates produce `Unavailable` with
  `reason=event_window_exceeded`, `scan_budget=512`, and
  `observed_count=513`;
- a future budget change is an explicit contract and test change, never an
  unreported query limit.

For another unavailable reason, `observed_count` is the number of candidates
read or entered before the failure was classified, bounded by 513. It is a
diagnostic count, not a count of suppressed paths.

## Conservative Unavailable Outcome

When suppression cannot complete truthfully, `Unavailable.remaining_paths`
is exactly the complete normalized input observed-path set. No path is removed,
hidden, marked recorded, or treated as authorized. The outcome has no
`suppressions` field and cannot be converted into `Applied` with an empty list.

The surrounding Guard result:

- continues with `decision=warn` (or the same owner-defined non-blocking
  conservative state) and `allowed=true`;
- processes every `remaining_paths` entry as not proven suppressed;
- exposes `suppression_outcome=unavailable` and the exact reason in its
  machine-readable result;
- does not claim that an unrecorded change exists solely because suppression
  was unavailable; and
- does not claim a clean or fully correlated observation.

The reason boundary is:

| Reason | Meaning |
|---|---|
| `event_window_exceeded` | More candidates exist than the explicit scan budget permits. |
| `store_read_failed` | A required Store read could not be completed and no narrower corruption or lookup reason was established. |
| `stored_event_corrupt` | A persisted Guard event claims the current contract but violates its typed or cross-field rules. |
| `correlation_payload_invalid` | The correlation payload is syntactically or structurally invalid for the current contract. |
| `run_lookup_failed` | The correlated Run could not be read or validated. |
| `write_ticket_lookup_failed` | The correlated write ticket could not be read or validated. |
| `path_identity_failed` | Current canonical path identity calculation could not be completed. |

Corrupt persisted data remains `Corrupt`; the Guard projection uses the
domain reason above without converting corruption into a successful empty
suppression. Environmental read failure remains `Unavailable`.

## Guard Hook Outcome Boundary

Guard hook processing separates three decisions that must not be inferred from
one another:

- `GuardObservationOutcome` records whether a compatible event was committed,
  an incompatible event was committed, or event persistence was unavailable.
- `GuardPolicyDecision` is optional and is exactly `Continue`,
  `ContinueWithContext`, `ContinueWithWarning`, or `Deny`. It exists only when
  structurally compatible input reached policy evaluation.
- The host adapter projects the outcome into host JSON, context, warning,
  denial, stderr, and process exit behavior. These are not Core or Store
  decisions.

`GuardHookOutcome` carries the observation outcome, optional policy decision,
at most eight typed diagnostics, and a safe context-or-warning feedback kind.
An event incompatible with the selected hook contract therefore has
`observation=IncompatibleRecorded` and no policy decision. It does not satisfy
the required phase, but it is not an automatic `Deny`.

For the Codex `record` profile, compatible events are recorded and non-denied
policy decisions continue. An explicit `PreToolUse` policy `Deny` is the only
branch projected as a Codex permission denial. Incompatible events and
event-persistence failure produce
bounded host context, empty stderr, and process exit `0`. Persistence failure
alone does not manufacture a policy denial. A `PostToolUse` warning describes
an action that already completed and never claims that Guard prevented or
reversed it.

The Codex adapter exclusively owns `hookSpecificOutput`,
`permissionDecision`, `additionalContext`, stderr, and exit-code projection.
Core-facing types and Store records do not encode Codex process-exit behavior.

The selected command-hook decoder is the `CodexCommandHooks` marker. It produces
only `CodexHookPromptCorrelation` (session/turn) or `CodexHookToolCorrelation`
(session/turn/tool-use ID/canonical tool name). It accepts unknown additive
fields within the bounded envelope, never requires a hook thread coordinate,
and never substitutes the separate `CodexMcpTurnMetadata` MCP correlation.

Hook occurrence and hook activation are separate projections.
`hook_source_activation` may become `effective_by_observation` only from a
compatible event owned by the current Guard Installation, policy hash,
integration revision, and current hook-definition boundary. Reapplying an
unchanged manifest preserves the boundary; changing managed definition content
advances it and invalidates earlier occurrence evidence. Host-reported
`disabled`, `managed_by_policy`, and `bypassed_for_invocation` states remain
explicit and distinct. Missing evidence remains `unknown`, never an inferred
trust decision.

Prompt, pre-tool, and post-tool observations are ambient phase details. They
support hook-execution diagnosis but are not independent proof of managed MCP
capability or correlated Guard verification. Project/configuration trust is a
separate host/user-owned check.

For `PreToolUse` and `PostToolUse`, the semantic Codex command-hook contract
owns one typed routing strategy. It forms a union of the reviewed native host
tools and server-qualified MCP routing. It uses the registered `McpServerKey`
namespace when the callable projection preserves that namespace; otherwise it
derives exact callable tokens from the canonical catalog. Generated matcher
JSON and strict configuration validation both project and reconstruct that
strategy; prompt capture has no tool matcher. The same owner supplies the
collision-checked `McpToolCatalog`, so routing never infers a server from a
dotted raw name and never selects behavior from a numeric host version.

Routing only delivers an event to the wrapper. Semantic filtering then decodes
the hook kind and callable, resolves the explicit server/tool identity to
`AgentToolId`, and assigns the catalog-owned `ProbeTarget`, `WorkflowControl`,
or `UnrelatedKnownTool` role before comparing any probe coordinate. Only
`AgentToolId::GuardProbe` at the exact current verification coordinate
satisfies Guard verification. Begin, status, and every other known tool are
nonterminal routed trace and never produce probe coordinate or callable
mismatch. An unknown same-server callable is nonterminal unless it claims the
exact current `verification_id`; that exact claim records terminal unknown
identity. A foreign server namespace is not routed.

Probe acquisition records one closed stage:
`ProbeAcknowledged`, `UnrelatedRoutedTool`, `HookEventNotObserved`,
`HookPayloadIncompatible`,
`CallableIdentityUnknown`, `CallableIdentityMismatch`,
`VerificationIdMismatch`, `SessionMismatch`, `TurnMismatch`,
`ToolUseMismatch`, `PreToolMatched`, or `PostToolMatched`. The record retains
only the expected agent-tool/callable identity, an optional bounded observed
callable, hook kind, categorical match facts, verification-ID presence/match,
and current installation/revision. It never retains the full hook payload,
prompt, tool input, or tool output. `HookEventNotObserved` means only that no
event reached Volicord; it does not prove whether host emission or routing was
responsible.
`UnrelatedRoutedTool` is bounded nonterminal trace. It neither supplies probe
proof nor chooses repair, acknowledgement, retry, or status-read behavior.

The same semantic host contract owns `HookObservationPolicy`, independently of
host package versions. The reviewed `CodexCommandHooks` profile is synchronous
and permits one status read after probe acknowledgement. That read maps the
persisted acquisition stages to a precise terminal repair reason if the exact
prompt/pre/post chain did not complete; cleanup expiry is not a substitute for
this decision. A new attempt is never created automatically in the same
semantic turn.

## Managed-file setup transaction

After acquiring the canonical Runtime Home setup lease, `volicord init` plans
the exact final bytes for every Guard-owned hook
configuration, dispatch script, phase wrapper, Codex rule, policy, Git exclude
block, and AGENTS guidance block without writing a target. Prepare validates
the current snapshot and stages each changed file in the target directory.
Commit replaces those files atomically in deterministic path order before
Codex configuration and the integration revision commit. The lease remains
held through managed-file rollback, so another supported setup cannot accept
Guard or Store state that this transaction can later remove.

On failure, committed Guard file replacements are restored in reverse order
only while their current digest still matches the staged setup digest. A later
external edit is preserved and produces a partial-rollback diagnostic instead
of being overwritten. Unmanaged bytes outside a managed block remain part of
the exact original snapshot and are restored unchanged.

## Diagnostics And Event Projection

Every `Unavailable` outcome emits a bounded diagnostic containing the project,
Guard event identifier, `suppression_outcome=unavailable`, reason, scan budget,
observed count, and observation time. When the related Guard event Store write
is available, the same machine-readable fields are included in that event.

Diagnostics and events do not include the complete path list, correlation
payload, file contents, tokens, or secrets. If the Store failure also prevents
persistent diagnostic or event recording, the machine-readable Guard response
still carries the outcome and reports diagnostic persistence as unavailable; it
must not claim that a record was committed.

Guard installation and observation diagnostics use closed source enums rather
than rendered summaries:

| Code | Typed condition |
|---|---|
| `guard.managed_file.missing` | A required non-wrapper managed file is missing. |
| `guard.managed_file.integrity_failed` | Managed content, ownership, marker, permission, or hook contract differs. |
| `guard.manifest.mismatch` | The strict manifest or wrapper authority binding differs. |
| `guard.hook_wrapper.missing` | A required phase wrapper or its metadata is missing. |
| `guard.hook_wrapper.not_executable` | A required wrapper lacks executable behavior. |
| `guard.hook_process.failed` | A typed Guard hook process failed. |
| `guard.phase.required_not_observed` | A required current phase has not been observed. |
| `guard.observation.incompatible` | A current event has an incompatible hook contract. |
| `guard.event.persistence_unavailable` | Guard could not commit the event observation. |
| `guard.policy.denied` | Compatible pre-tool input reached policy and was denied. |
| `guard.host_output.projection_failure` | A host adapter could not project the typed outcome. |
| `guard.internal.unexpected_failure` | Guard encountered an unexpected failure without a narrower typed mapping. |
| `guard.prompt_capture.unsupported` | The host boundary does not support configured prompt capture. |
| `guard.prompt_capture.unobserved` | Supported configured prompt capture has not been observed. |
| `guard.probe.hook_event_not_observed` | The current synchronous observation did not receive the required hook event; a deferred semantic policy may use its explicit deadline. |
| `guard.probe.payload_incompatible` | The acquired hook payload was structurally incompatible. |
| `guard.probe.callable_mismatch` | The bounded observed callable did not match the expected Guard probe callable. |
| `guard.probe.verification_id_mismatch` | The hook event carried another verification ID. |
| `guard.probe.session_mismatch` | The hook event belonged to another host session. |
| `guard.probe.turn_mismatch` | The hook event belonged to another host turn. |
| `guard.probe.tool_use_mismatch` | Pre/post events did not share the required tool-use identity. |
| `guard.probe.current_contract_changed` | The integration revision, hook definition, or policy changed. |

Finding facts may identify only the bounded artifact kind, phase, categorical
state, and current revision coordinates. They do not project managed-file
contents, prompt text, arbitrary event payloads, or unrestricted paths.
Hook occurrence facts are limited to the contract profile, hook event kind,
missing or malformed field category and static field label, Guard Installation
ID, integration revision, and Guard event ID when available. They never include
complete prompts, tool inputs, tool responses, parser prose, or unrestricted
stderr.
File, manifest, wrapper, and incompatible-observation findings project to the
typed activation steps `repair_hook_contract` or
`repair_managed_configuration` according to their typed condition. A missing
current observation projects the user-level
`request_integration_verification`; its workflow-directed `volicord.guard_probe`
is nested inside the agent sequence and is never a sibling top-level step. A
terminal `repair_required` attempt projects its typed repair step rather than a
blind probe retry. No step is selected by parsing a human summary.

Current-state Guard diagnostics use the exact managed artifact, installation,
required phase, or incompatible event as their typed subject. Their stable ID
is the full fixed digest of the complete `CurrentDiagnosticKey`: Connection
scope, code, domain, stage, source, and the opaque typed subject identity. The
identity token and ID contain no managed path. The separate subject kind and
reference are safe snapshot presentation. The same Guard code can therefore
identify several affected artifacts or phases without collision. A repeated
observation of one subject refreshes that finding's safe subject presentation,
facts, observation time, revision coordinates, and cause edges; it does not
append another stale current-state copy. A Connection report includes only the
Guard findings referenced by its current checks and their bounded cause chains.

Guard verification reconciles its complete typed observation set. A repaired
artifact or installation, newly observed required phase, compatible current
event, supported prompt-capture boundary, or matching integration revision
omits the prior condition and explicitly resolves its active finding. Resolved
Guard findings remain available by exact ID but are excluded from the current
Connection report. The immutable definition for each closed Guard diagnostic
owns its code, domain, stage, source, default severity, and summary; action
selection consumes that definition, typed facts, and typed check state.

### Guard Verification Dependencies

Connection verification uses this explicit Guard dependency graph:

```text
hook_source_activation -> ambient_hook_coverage -> correlated_guard_verification
```

Each check has exactly one of five statuses. `passed` means the check completed
successfully. `pending` means the required external observation or
user-triggered event has not occurred and no failed prerequisite prevents it.
`failed` means that check itself observed a failure. `blocked` means it could
not run or be observed because a prerequisite finding failed.
`not_applicable` means the check does not apply to the Connection or profile.

A Guard managed-file or current-definition failure makes
`ambient_hook_coverage` fail or remain blocked and blocks
`correlated_guard_verification` by the same resolved root finding. A passed
ambient check proves only the current definition and general configured-phase
coverage. The correlated check uses the latest current attempt: absent or
genuinely deferred active is pending, complete is passed, and
`repair_required` is failed with typed recoverability and action. Its details
retain the latest attempt separately from the latest completed current proof,
so an older proof cannot hide a newer terminal failure. Root codes come from
the typed repair reason and acquisition stage, not rendered summaries. The
report does not request downstream observation while a prerequisite check is
blocked. Root selection follows typed finding cause edges, retains independent
roots in deterministic order, and does not inspect summaries. The complete
check graph and aggregate report status are owned by
[Agent Connection](agent-connection.md).

## Required Tests

Durable contract tests cover:

- applied suppression with an exact unchanged recorded identity;
- applied outcome with no suppression candidate;
- exactly 512 candidates and more than 512 candidates;
- corrupt stored event and malformed correlation payload;
- Run and write-ticket lookup failures;
- path-identity calculation failure;
- Store read failure preserving every input path;
- warning, diagnostic, and event reason projection without sensitive payloads;
- incompatible prompt, pre-tool, and post-tool events continuing without a
  policy denial while remaining unsatisfied observations;
- explicit pre-tool denial and non-blocking post-tool projection; and
- event-persistence failure continuing with bounded Codex feedback;
- typed routing-strategy generation and reconstruction, followed by semantic
  exclusion of unrelated read-only tools from Guard verification; and
- rejection of mismatched session, turn, tool-use ID, tool name,
  verification ID, policy, revision, hook digest, ordering, and expiry.

## Adjacent Owners

- Product path normalization: [Runtime Boundaries](runtime-boundaries.md).
- Failure category meanings: [Failure Model](failure-model.md).
- Write-ticket and Run state shapes: [API State Schemas](api/schema-state.md).
- Guard implementation tests and optional host smoke: [Testing Strategy](../architecture-guide/testing-strategy.md).
- Security and diagnostic non-guarantees: [Security](security.md).
