<a id="volicordprepare_evidence_capture"></a>

# `volicord.prepare_evidence_capture` reference

## Owned here

This page owns the baseline behavior of `volicord.prepare_evidence_capture`:

- creation of one immutable, expiring `EvidenceCaptureIntent`
- the method request, result, defaults, replay, and no-effect branches
- the boundary between an Agent Connection request and source-owned fulfillment

Evidence and Run authority is owned by [Core Model](../core-model.md#9-evidence-and-run-authority).
The shared intent, receipt, and producer shapes are owned by
[API State Schemas](schema-state.md). Storage layout and finalization effects
are owned by the Storage Reference pages.

## Purpose and authority boundary

An Agent Connection can ask Volicord to bind a future command or host-tool
invocation to one exact current Evidence
target. The method creates an intent only. It never records that the source ran,
never creates an `EvidenceProducer`, and never grants Strong Evidence.

Only a registered source fulfills an intent:

- `verified_command_execution`: the administrative
  `volicord evidence capture-command` runner executes the exact digest-bound
  argument vector and captures its result. This local fulfillment runner is
  supported on Linux and macOS; other platforms reject it before execution.
- `verified_tool_invocation`: a registered tool source captures the exact host
  invocation with the same connection, host invocation ID, tool name,
  canonical input digest, and complete result digest. Session/time fallback is
  not eligible.

Fulfillment creates an immutable durable `EvidenceCaptureReceipt` source-fact
record and a bounded redacted transient receipt artifact staging handle. It does not advance Core
state. `volicord.record_run` is the only transition that can revalidate the
intent and receipt, promote the receipt artifact, create the immutable
`EvidenceProducer`, and create its one-to-one `EvidenceObservation` in one Core
commit.

## Request

### `PrepareEvidenceCaptureRequest` fields

| Field | Required | Nullable | Type |
|---|---|---|---|
| `baseline_ref` | yes | no | `string` |
| `capture` | yes | no | `EvidenceCaptureSpec` |
| `change_unit_id` | yes | no | `string` |
| `envelope` | yes | no | `ToolEnvelope` |
| `target` | yes | no | `EvidenceTarget` |
| `task_id` | yes | no | `string` |



`command_sha256` is the bare lowercase SHA-256 of canonical JSON for the
complete UTF-8 argument vector, including the executable as element zero.
`tool_input_sha256` is the same form of digest over the selected canonical tool
input object. These digests use the canonical JSON rules owned by
[API Schema Core](schema-core.md). Raw arguments, environment values, command
output, and tool input or output are not public request fields and are not
stored in the intent.

`command_label` uses shared display-text normalization, while `tool_name`
removes leading and trailing whitespace without rewriting its internal
identifier text. The non-empty and 256-UTF-8-byte limits are checked afterward,
and the resulting value is stored in the immutable intent.

MCP omission defaults are `expected_exit_code=0` and `expected_success=true`.
Explicit `null` has the same meaning. The method
rejects an empty safe label, malformed digest, target
outside the current Task, non-current Change Unit, incompatible baseline, or
missing verified Agent Connection context before commit. Tool capture also
requires an exact verified host invocation in the invocation context; command
capture does not.

The intent is bound to the selected project, Task, current Change Unit, current
scope revision, compatible baseline, exact target, current Git workspace
identity, requesting connection and actor, capture kind, canonical input
digest, expected outcome, creation time, and a fixed 15-minute expiry. An
unrelated later state-version increment does not alone expire the intent;
changes to a bound basis do.

## Result

```yaml
PrepareEvidenceCaptureResult:
  base: ToolResultBase
  capture_intent_ref: StateRecordRef
  capture_intent: EvidenceCaptureIntent
  expires_at: UtcTimestamp
```

A committed result uses `effect_kind=core_committed`, appends one authority
event, inserts one intent and replay row, and advances
`project_state.state_version` once. The returned ref has
`record_kind=evidence_capture_intent`. Exact idempotent replay returns the
original response and creates no second intent. A dry run creates no durable ID,
intent, event, replay row, receipt, artifact, producer, or state-version change.

## Fulfillment and receipt rules

The fulfilling source rechecks project and Task identity, current Change Unit,
scope revision, baseline, target, workspace identity, requesting connection,
expiry, and the exact Core-derived input digest. A disabled connection,
mismatched invocation ID or digest, truncated or incomplete output, or reused
intent cannot produce an eligible receipt.

The selected source observation must satisfy the half-open window
`intent.created_at <= observed_at < intent.expires_at`. Receipt creation must
satisfy `observed_at <= receipt.created_at < intent.expires_at`; its staging
handle expires exactly at `intent.expires_at`. A pre-intent observation, an
observation at the expiry instant, a receipt timestamp before its observation,
or a receipt created at or after expiry is rejected.
Core finalization also rejects an observation or receipt timestamp later than
the current Core clock, even when the stored intent expiry is later.

Fulfillment derives an exclusive normalized host-invocation claim from the
immutable intent and strict receipt shape; callers do not select the claim.
Each host invocation can fulfill only one intent and producer class in a
project. A missing, ambiguous, mismatched, or already claimed invocation
rejects the entire fulfillment, including receipt and staging creation.

The safe receipt JSON is bounded to 24 KiB and contains schema version, capture kind, intent ID,
input and result digests, expected and observed outcome, success/status or exit
code, registered connection and host-invocation identity, observation time,
completeness, limitations, and `redaction_state=redacted`. It contains no
raw command, environment, stdout, stderr, tool input, tool response, secret, or
unbounded host payload. The receipt record is one-time and content-bound to its
staged bytes.

`result_sha256` is the bare lowercase SHA-256 of canonical JSON for the complete
`observed_outcome`. Command outcomes retain exit code plus stdout/stderr digest
and byte count, not raw output. The command runner streams a combined maximum
of 16 MiB and must finish before intent expiry; exceeding either boundary
creates no receipt. Tool outcomes retain success, optional exit
code, and complete tool-result digest and byte count. The staged receipt itself is always
`redaction_state=redacted`.

Registered tool-source capture is cooperative local integration. It is not a
host signature, actor-attribution proof, OS isolation, or anti-forgery boundary
against the same local principal. The command runner records execution; it does
not approve a command, grant permission, create a sandbox, prove test
sufficiency, or prove broad correctness.

## `record_run` consumption

An evidence input claims this path by placing exactly one current
`evidence_capture_intent` ref in `input_refs`. Core loads the intent and receipt
directly; caller-supplied tool fields, actor fields, output refs, receipt handle,
and outcome metadata cannot replace stored source facts.

If the complete observed outcome satisfies the stored expectation, Core records
strong producer provenance but leaves observation relevance `unassessed`. The
registered source establishes what ran or was observed; it does not decide that
the result supports the selected target. A complete outcome that mismatches
the stored expectation is preserved as `contradicted`. A capture-backed observation therefore
cannot by itself make a required criterion sufficient. A separate
owner-defined relevance authority is required for `supported`. The capture
intent ref remains the classification basis in
`relevance_assessment.assessment_ref`, with no assessing actor; it is not a
separate relevance decision or support authority. A referenced
intent that is missing, expired,
already consumed, corrupt, cross-project, cross-Task, cross-Change-Unit,
cross-connection, stale for scope/baseline/workspace/target, or inconsistent
with receipt bytes is rejected with no Core commit; it is not silently
downgraded. Inputs without an intent continue to use the existing cooperative
downgrade rules.

One receipt yields at most one producer, and one producer belongs to exactly
one observation. Source-claim exclusivity prevents one fact from being
captured through several classes; class selection therefore follows the
intent's exact capture kind rather than a fallback match. Later reuse uses the
existing `reused_evidence` chain rather than consuming the intent again.

## Related owners

- [Record-run method](method-record-run.md)
- [Core Model](../core-model.md#9-evidence-and-run-authority)
- [API State Schemas](schema-state.md)
- [API Value Sets](schema-value-sets.md)
- [Agent Connection](../agent-connection.md)
- [Security](../security.md)
- [Storage Records](../storage-records.md)
- [Storage DDL](../storage-ddl.md)
- [Storage Effects](../storage-effects.md)
- [Storage Versioning](../storage-versioning.md)
