# Failure model

This document owns the product-wide failure categories used across Core,
Store, adapters, transports, administrative commands, and diagnostics. It
defines the exact category identifiers, their semantic boundaries, persisted
state handling, and the prohibition on turning failure into a synthetic or
default success value.

It does not define a particular API response envelope, transport error shape,
method storage effect, domain-specific reason catalog, or repair command.
Those details remain with the focused owner for the affected surface.

<a id="surface-stability"></a>
## Surface Stability

The five category meanings, their machine-readable identifiers, persisted
authority and policy fail-closed rules, and no-default-conflation rules are
stable contracts. Human display text and domain-specific diagnostic details
are diagnostic unless their focused owner says otherwise.

## Canonical failure categories

Every machine-readable failure classification uses one of these exact primary
category identifiers:

| Category | Identifier | Meaning |
|---|---|---|
| `Rejected` | `rejected` | The request shape or required context is structurally invalid, so the requested operation does not proceed to policy evaluation or its successful operation branch. |
| `NotAllowed` | `not_allowed` | A structurally valid request and complete required context reached policy evaluation, and policy did not allow the requested operation. |
| `Unavailable` | `unavailable` | The operation, auxiliary capability, or required read cannot currently be performed, while the available data does not establish persisted contract corruption. |
| `Degraded` | `degraded` | The core operation can continue, but an explicitly identified verification, diagnostic, or auxiliary-information component is incomplete. |
| `Corrupt` | `corrupt` | Persisted or trusted owner data violates its declared schema, type, canonical encoding, or cross-field contract. |

A machine-readable result or diagnostic must carry the exact category
identifier and, when the domain distinguishes causes within that category, a
domain-owner-defined reason identifier. Human text alone is not a category or
reason. A reason identifier must not silently change the category meaning.

## Category selection boundaries

`Rejected` and `NotAllowed` are separated by policy evaluation. Missing or
invalid required context is `Rejected`; it must not be represented as a policy
denial. `NotAllowed` requires a structurally valid request, resolved required
context, and an actual policy decision. A method owner may define a committed
non-allow result, but the category by itself does not authorize a commit.

`Unavailable` and `Degraded` are separated by whether the core operation can
truthfully continue. If the required operation or read cannot be performed, the
category is `Unavailable`. If the core operation remains valid while a named
auxiliary verification or information source is incomplete, the category is
`Degraded`; the missing part and its effect must remain visible.

`Corrupt` applies when data that claims to follow a supported persisted or
trusted contract violates that contract. A malformed or unknown untrusted
boundary input that has not become persisted owner state is `Rejected`, not
`Corrupt`, and is not guessed into a supported shape.

### Versioned host-contract rejection

Codex wire input is decoded only by the explicitly selected profile.
`CodexMcpTurnMetadata` selects `codex-mcp-turn-metadata`, while the distinct
`CodexCommandHooks` selects `codex-command-hooks`. A failure under one is not retried,
reinterpreted, or completed with fields from the other. Unknown additive
fields are accepted, but missing or invalid required fields, an unexpected
event value, inconsistent MCP thread coordinates, and an input exceeding the
contract's size or depth bounds are `Rejected` before Store or policy
evaluation.

The typed host-contract error retains only a closed error code and the static
field label. It does not retain or project the complete hook payload, MCP
metadata, tool input, or tool response. A hook failure never manufactures a
thread coordinate from its session, environment, or runtime state. Registered
managed-session correlation remains an independent MCP authorization check
after successful profile decoding.

For a Guard hook in the `record` profile, that `Rejected` classification
describes the observation input; it is not a `NotAllowed` policy result and
does not say that the host action was denied. Guard records the incompatible
observation when Store is available, leaves `GuardPolicyDecision` absent, does
not count the event toward phase satisfaction, and asks the host adapter to
continue with bounded feedback. If event persistence is unavailable, the
observation outcome remains explicitly unavailable and persistence failure
alone does not become policy denial. An actual `NotAllowed` Guard result
requires compatible input that reached policy and produced `Deny`.

Connection-integration verification keeps tool-call rejection separate from
run lifecycle. A malformed ID or a call from another runtime, native session,
or turn is rejected without changing the run. Absence of the required current
prompt/pre/post correlation leaves an active run pending until its bounded
expiry; expiry is the run status `expired`, not fabricated historical success.
A terminal or expired run without a prior probe acknowledgement rejects a late
probe without creating one. When an exact caller-coordinate replay already has
an acknowledgement, an active probe replay returns
`awaiting_hook_completion` with the original timestamp, a completed replay
returns `complete`, and an effectively failed or expired replay returns the
corresponding typed `restart_required` state. No replay reactivates the run.
Coordinate rejection never exposes that timestamp to another session or turn.
A stored pass whose runtime, Guard Installation, policy, integration revision,
or hook contract is no longer current projects as the typed `failed` restart
reason. Failed and expired runs direct a new verification through the canonical
begin tool after the condition is repaired or the previous window expires.
These workflow states are Connection-check facts and do not redefine the
product-wide failure categories above.

Active connection verification discovers the configured Codex executable, runs
its version command, and reports every behavioral check using the five-state
model defined below. Failure to find or run the executable is `Unavailable` at
a general failure-category boundary and a failed `host_executable` connection
check. The PATH executable version remains an installation probe; it does not
replace actual MCP peer `clientInfo` or invalidate an otherwise valid managed
session merely because the two versions differ.

The administrative connection command report uses `failed` for a typed
operational result with at least one failed or blocked required check. Pending
host observation is `action_required`, not `Degraded`, a stale/broken public
status, or an unexpected runtime error. Usage and unexpected runtime or
serialization failures remain on the CLI error channel rather than being
fabricated into a successful or action-required report.

No category implies another. In particular:

- `Unavailable` is not an empty successful result.
- `Degraded` is not full verification or an unqualified success state.
- `Corrupt` is not a missing optional value.
- `NotAllowed` is not structural rejection.

## Persisted authority and policy data

Persisted authority or policy data that cannot be decoded and validated under
its declared contract is `Corrupt` and fails closed. Any operation that depends
on that data must stop before it derives authority, makes a policy decision,
records a successful effect, or mutates dependent owner state.

Typed persisted JSON must be decoded into its complete declared type. Syntax
failure, a wrong top-level shape, an unknown closed variant, a missing required
field, an extra field where the owner rejects extras, or a violated cross-field
invariant is `Corrupt`. None of those conditions becomes an empty array, empty
object, absent value, or host-specific default.

Missing display-only or auxiliary data is not automatically corruption. Its
focused owner must classify it as `Unavailable` or `Degraded` and state whether
the core operation can continue. A valid empty array or object remains a valid
empty value only when the declared schema explicitly permits that exact value.

Recoverable state may be regenerated only through an explicit owner-defined
verify or repair flow. Reads and ordinary execution must not repair, migrate,
guess, or silently replace the data while classifying the failure.

## Structured Diagnostic Findings

`DiagnosticFinding` is the shared read-only report projection. A producer must
choose `DiagnosticFindingLifecycle::Occurrence` or
`DiagnosticFindingLifecycle::CurrentState` explicitly and construct the
matching lifecycle type; runtime-session presence never selects a lifecycle.
Both forms carry a namespaced code, domain, stage, severity, producer source,
typed subject, safe projected facts, zero or more cause references and
recommended actions, an observation timestamp, and applicable correlation
coordinates. Domain owners retain their closed code vocabularies and
error-to-finding conversion rules.

`DiagnosticFinding` itself is not a writable lifecycle input. Store mutation
accepts `OccurrenceDiagnosticFinding` for insertion,
`CurrentDiagnosticFinding` for snapshot activation or refresh, and
`CurrentDiagnosticKey` for explicit resolution.

`OccurrenceDiagnosticFinding` records one runtime, process, protocol, or other
event-like observation. Each occurrence receives a newly generated opaque
`DiagnosticOccurrenceId`; repeating identical diagnostic data creates a
different ID and an independent immutable row. Occurrence graph insertion is
insert-only whether or not runtime correlation is present.

`CurrentDiagnosticFinding` consists of an immutable `CurrentDiagnosticKey` and
a replaceable `CurrentDiagnosticSnapshot`. The key includes the scope kind and
complete opaque scope identity, full diagnostic code, domain, stage, source,
and one `DiagnosticSubjectIdentity`. That subject identity is a validated,
opaque `sha256:<64 lowercase hex>` token derived only from the typed subject
owner's domain-separated, versioned, length-prefixed canonical identity bytes.
It is not a display string and does not expose the original path, installation
identifier, event identifier, or other identity input. The current-key
canonical identity is a separate domain-separated, versioned binary encoding
in which every variable component, including the complete subject identity
token, is length-prefixed. The ID is exactly
`finding.current.sha256:<64 lowercase hex>`, using the full SHA-256 digest of
that encoding. This preserves all identity distinctions while keeping paths
and other subject identity inputs out of the ID.

The replaceable snapshot carries the safe `DiagnosticSubject` presentation in
addition to severity, facts, actions, correlation coordinates, integration
revision, observation time, outgoing cause edges, and active or resolved
state. Re-observing the same key may replace that safe display subject and the
other snapshot fields and reactivates a resolved condition. Identity fields,
including `DiagnosticSubjectIdentity`, are compared and never updated.
Changing only redaction, formatting, or another safe presentation detail does
not change the finding ID. Explicit resolution records `resolved_at`, removes
current actions and outgoing causes, retains the last safe subject and facts,
and excludes the row from active-current reports. Explicit ID reads may still
return that resolved snapshot. A read reconstructs the key from the stored
subject identity independently of the safe display subject, recomputes the
current digest and ID, and treats any mismatch as corrupt persisted state.

A CLI-owned operational diagnostic has one immutable definition for its code,
domain, stage, source, default severity, and summary. Its closed typed subject
owns the scope, constructs `DiagnosticSubjectIdentity` from its own canonical
identity encoding, and supplies a separate safe display projection. Typed
subject namespaces are part of that canonical encoding, so equal display text
in different subject families does not collapse to one identity. A
path-bearing subject canonicalizes filesystem aliases before deriving the
opaque identity and never persists those canonical path bytes. Optional active
verification reconciles each complete owner observation set: observed
conditions are activated or refreshed and previously active owned conditions
omitted from that set are explicitly resolved.

A current report resolves its selected IDs through an explicit-provenance
overlay. An inline finding calculated by the current evaluation takes
precedence over Store lookup. An explicitly persisted seed is then resolved
from immutable occurrences or active current-state rows and may extend the
same bounded cause graph. Only an explicitly persisted reference with no Store
row becomes `diagnostics.finding_record_missing`; a calculated inline finding
never receives that substitution.

Safe facts are bounded before storage or rendering. Their typed projection
redacts sensitive keys and limits text size, collection size, and nesting
depth. Raw environment maps, request bodies, tool argument sets, credentials,
and unrestricted child-process output are not diagnostic facts. A producer
must supply a bounded safe summary instead of moving those inputs into a
finding.

The Registry stores each shared finding as structured columns plus bounded
safe subject, facts, and action JSON. A current-state row additionally stores
its validated opaque subject identity token; occurrence rows do not. Cause
references are separate edges. Every
edge must resolve to an existing finding, duplicate and self edges are
rejected, cycles are rejected by validated graph insertion and the Registry
constraint, and bounded traversal is deterministic. Inserting a finding graph
is one transaction: an invalid node, missing cause, duplicate edge, or cycle
leaves no partial graph or dangling edge. MCP terminal finding insertion and
runtime-session linking may likewise be one transaction.

Root-cause selection uses only those typed cause edges. It never parses a
summary, compares stage or enum ordering, or chooses the first failed check.
Selection sorts IDs for deterministic output, retains multiple independent
roots, and removes a downstream symptom when a selected ancestor already
explains it. Shared ancestors therefore appear once even when several selected
findings converge on them. Traversal is limited to 32 cause edges; an unknown
reference, a cycle, or a path beyond that bound rejects selection rather than
guessing a root. `DiagnosticReport.root_cause_ids` is the derived result for
the report's findings and cannot be supplied as an independent caller choice.

`DiagnosticReport` is the lossless selected-Connection diagnostic JSON
envelope. Its `schema_version` is `2`. It contains the typed `operation`, aggregate `status`,
generation timestamp, optional Connection context, complete check array,
bounded finding graph, derived root-cause IDs, deduplicated report actions,
operation-specific typed details, and report limits. A report action contains a
namespaced code, bounded summary, and the exact root IDs it remediates.
Connection context includes role-preserving runtime-session evidence selected
as `latest_attempt` and `latest_complete_proof`. IDs come from check evidence as
well as finding correlation; one ID with both roles is represented once with
both roles. The roles distinguish current attempt health from a possibly older
same-session capability proof and prohibit consumers from treating milestones
across sessions as one success.
Deserialization rejects any other schema version, unknown top-level member,
duplicate check or finding ID, invalid cause graph, supplied root list that
differs from the derived roots, duplicate action code, or action that refers to
a non-root finding. There is no second legacy connection-report schema.

Exact finding and runtime-session reads use the separate schema-1
`DiagnosticLookupReport`. Its `lookup_status` is exactly `found` or
`not_found` and does not use Connection aggregate or check statuses. A found
finding root and every entry in its bounded cause graph use
`StoredDiagnosticFinding`: an occurrence is tagged `lifecycle=occurrence`; a
current record is tagged `lifecycle=current_state` and carries its explicit
`current_state_status` plus `resolved_at`. The same envelope can carry the
distinct runtime-session root while retaining lifecycle-aware terminal and
correlated occurrences. Finding severity and current-state status describe
stored conditions; neither changes a successful lookup into a failed lookup.

Machine consumers distinguish observation outcomes structurally. An absent
observation is an omitted optional value or an owner-defined typed
`observation_state=absent`; an observed empty collection is the present value
`[]`; an observation failure is a `failed` check with cause findings; and a
prerequisite-blocked observation is a `blocked` check with those root IDs.
Producers must not encode any of these states as a human summary that consumers
must parse. Renderers may select and label typed facts, but cannot derive cause
edges or action categories from prose.

Connection verification consumes this cause graph through exactly five check
states. `passed` means the check completed successfully. `pending` means its
required external observation or user-triggered event has not occurred and no
failed prerequisite prevents it. `failed` means the check itself observed a
failure. `blocked` means a prerequisite finding failed, so the check could not
run or be observed. `not_applicable` means the check does not apply to the
Connection or profile. A blocked check carries the resolved root finding IDs;
root-derived actions are deduplicated and a blocked downstream observation
does not create an action before its blocker is resolved. The canonical check
dependencies and aggregate report rules are owned by
[Agent Connection](agent-connection.md).

When failure occurs before the Registry can be opened, the only shared stderr
fallback envelope is one bounded line in this exact form:

```text
VOLICORD_DIAGNOSTIC_V1 <bounded-json>
```

The JSON is exactly one current shared `DiagnosticFinding`. Formatting and
parsing enforce the shared field validation, safe-fact bounds, exact prefix,
single-line shape, and whole-envelope byte limit. The fallback does not permit
environment dumps or create a second diagnostic model.

## No synthetic or default conflation

Failure must not be converted into an ordinary value by using:

- an empty string, array, object, or zero value that the contract did not
  actually contain
- a synthetic identifier, placeholder record, fabricated timestamp, or
  fabricated capability
- a default enum variant selected after decoding or lookup failed
- a fallback host, adapter, decoder, external contract, or storage shape
- a historical alias or deprecated shape treated as the current contract
- a successful response whose only indication of failure is human text

Implementation convenience such as `unwrap_or_default()` does not change this
contract. A default is valid only when it was established before persistence by
the owner-defined typed construction path and the resulting stored value itself
passes the complete declared contract.

### Activation conditions are not interchangeable

Connection activation preserves these distinctions:

- `unknown` means authoritative hook state and current-definition observation
  are absent; it is not `disabled`, failed, or untrusted.
- `review_required_by_setup` means setup changed the definition and host review
  is outstanding; it is not a configuration failure.
- `bypassed_for_invocation` is explicit invocation-local host evidence; it is
  not durable activation.
- `disabled` requires explicit host evidence and routes to
  `inspect_hook_contract`.
- a terminal current managed session is a failure even if an older
  `latest_complete_proof` remains available.
- a missing current Guard correlation is pending and routes to
  `run_guard_probe` only after prerequisites pass.

Failures and blocked checks retain typed root finding IDs. Remediation is chosen
from the closed connection actions `reload_host`, `review_hooks`,
`run_mcp_verification`, `run_guard_probe`, `inspect_hook_contract`,
`repair_managed_configuration`, `inspect_runtime_session`, and
`reinstall_current_build`; renderers do not infer an action from prose.
Project/configuration trust remains separate from hook-source activation.

## Effects and response routing

The failure category does not by itself define state mutation, retryability,
HTTP or JSON-RPC status, CLI exit code, API response branch, or display text.
The affected method, adapter, transport, CLI, and storage-effect owners define
those projections while preserving this category meaning.

Adjacent owners:

- Public API branch routing and public codes: [API Errors](api/errors.md).
- Persisted record contracts: [Storage Records](storage-records.md).
- Runtime Home placement and the pre-Registry stderr fallback boundary:
  [Runtime Boundaries](runtime-boundaries.md).
- Method-to-storage effects and no-effect branches:
  [Storage Effects](storage-effects.md).
- Administrative verification and repair commands:
  [Administrative CLI](admin-cli.md).
