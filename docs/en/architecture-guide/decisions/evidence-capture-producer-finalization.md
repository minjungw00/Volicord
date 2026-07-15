# Evidence-capture intent and producer finalization

## Context

The evidence schema reserved producer classifications for verified command
execution, verified tool invocation, and registered connection observation, but
the baseline had no authority-owned records or transitions that could create
them. `record_run` therefore had to downgrade every direct external-tool or
connection claim to a cooperative report. Artifact integrity, descriptive tool
fields, `SourceRef`, raw guard payloads, and session timing could not safely fill
the missing producer authority. Target relevance remains a separate authority
axis.

A producer created directly from agent input would let the claimant choose its
own execution, output, observer, and relevance. A producer created immediately
inside a host callback would also split authority across a callback transaction
and a later Run commit, and would require a second run-less persistent artifact
lifecycle.

## Decision

Volicord uses one three-stage invariant for all three producer kinds:

`EvidenceCaptureIntent -> EvidenceCaptureReceipt -> record_run finalization`.

1. The additive stable workflow method `volicord.prepare_evidence_capture`
   creates only an immutable 15-minute intent. It binds the current Task,
   Change Unit, scope revision, baseline, target, workspace, requesting
   connection and actor, exact canonical command/tool input digest or
   Core-derived connection source-selector digest, and expected outcome.
2. A registered source fulfills the intent and atomically creates an immutable
   durable source-fact receipt plus bounded redacted transient receipt-artifact staging and an
   exclusive normalized claim for every underlying source fact. The agent API
   has no receipt or producer creation method.
3. `volicord.record_run` revalidates the complete chain and, in its existing
   atomic Core commit, promotes the receipt artifact, inserts the immutable
   producer, inserts its one-to-one EvidenceObservation, links records, appends
   the event and replay row, and advances state once.

The source adapters are deliberately different while the authority invariant
is shared:

- A Volicord-owned administrative command runner executes an exact
  digest-bound UTF-8 argument vector and records exit status and output
  digests. It does not authorize, approve, or sandbox the command.
- A registered tool adapter requires an exact pre/post pair with the same
  connection, session, host invocation ID, tool and canonical input. It does
  not use session/time fallback. Registered hooks provide cooperative host
  consistency, not cryptographic host attestation or protection from the same
  local principal.
- A registered connection observation requires an exact registered guard fact
  whose closed event kind matches the pre-intent selector, or a complete
  non-degraded session-watcher snapshot from the unique current active baseline
  for the intent-bound connection and session. Event/observation identity,
  observation time, and raw-event or snapshot digest are fixed only by the
  source-owned receipt.

Live-harness source availability is a durable source-observation barrier, not a
host-lifecycle barrier or source fulfillment itself. For a registered tool, the
barrier is reached when the complete post event is durably persisted with its
immutable intent and matching post-intent pre event whose decision was not
`deny`. For a Stop-selected registered connection observation, it is reached
when the exact post-intent Stop event is durably persisted; the Stop decision
is source outcome rather than an exit prerequisite. Neither barrier waits for a
model response, host turn, Stop `allow`, close readiness, or host-process exit.
After observing the barrier, a consumer must claim the exact source through the
separate fulfillment transaction before the intent expires; lifecycle signals
do not extend the 15-minute window or replace a missing source fact.
The exact release-cell consequences remain owned by
[Host Release Evidence](../../reference/host-release-evidence.md). Source
fulfillment remains owned by
[`volicord.prepare_evidence_capture`](../../reference/api/method-prepare-evidence-capture.md)
and rejected-transaction effects remain owned by
[Storage Effects](../../reference/storage-effects.md).

The Store derives the claim set from the strict receipt and immutable capture
spec rather than accepting caller-selected claims. Command capture claims its
normalized host invocation. Tool capture claims its normalized host invocation
and both distinct guard events. Guard connection capture claims its one guard
event, while watcher capture claims its one observation. Missing, extra, or
ambiguous class coordinates reject, and the receipt, staging body, and every
claim commit or roll back together. Host invocation claim identity includes its
connection, session, installation, and host-local invocation coordinates so
unrelated host-local namespaces do not collide.

The capture intent, receipt, producer, and promoted receipt-artifact chain does
not persist raw commands, environments, stdout, stderr, tool inputs, tool
responses, or unbounded host payloads. Existing guard-event subject storage is
a separate path: it may retain a redacted `raw_event`, including tool fields
allowed by the current guard redaction rules. That guard record is not a
capture receipt or producer and cannot substitute for either. The receipt
stores bounded safe identity, digests, observed outcome, source refs,
completeness and limitations. Incomplete or truncated sources do not produce
an eligible producer. Even a complete producer does not become Strong Evidence
without a separate supported relevance assessment.

A complete outcome matching the stored expectation yields strong producer provenance and unassessed
relevance; it never self-authorizes support for the selected target. A complete
outcome that mismatches the stored expectation yields contradicted relevance. Explicit missing,
stale, corrupt, cross-context, or already consumed intent references reject
rather than silently downgrade. An input with no intent keeps the previous
cooperative downgrade behavior. Exclusive source claims prevent one fact from
being captured through several classes, so producer class follows the exact
capture kind in the immutable intent. Producer reuse uses the existing evidence
reuse chain.

## Storage and compatibility

The model adds `evidence_capture_intents`, `evidence_capture_receipts`,
`evidence_capture_source_claims`, and `evidence_producers`, plus the
`evidence_producer` artifact-link owner kind. Intent, claim, and producer
records are insert-only. Receipt, staged bytes, and all source claims are
created together at the Store boundary. The project-scoped source-claim key
prevents any exact underlying invocation, event, or watcher observation from
fulfilling more than one intent or producer class. A composite receipt foreign
key prevents producer intent/receipt cross-pairing. Producer intent and
observation identities are unique so a completed intent cannot be consumed
twice.

The initial producer-finalization model was the incompatible
`baseline_sqlite_v4` / `0.7.0` canonical SQLite shape change. The then-current
`baseline_sqlite_v5` carried these record families. The follow-up
connection-selector correction removes the caller-supplied future observation
digest from the public and persisted capture-spec shape but adds no table,
column, index, or constraint. It was therefore completed inside the pre-major
`baseline_sqlite_v5` / `0.8.0` contract batch rather than creating a second
storage-profile or package-version transition. Current `baseline_sqlite_v6` /
`0.9.0` retains the corrected record families; its host-capability Registry
addition is independent of this decision. The removed shape has no legacy
alias or fallback decoder. Incompatible v5 or earlier Runtime Homes fail
compatibility checks and require recreation.

## Consequences

- Agents can declare intended capture and its target, but cannot create the
  execution receipt, producer authority, or supported relevance they later
  cite.
- A connection caller can bind only a pre-intent source selector. It cannot be
  required to predict a host-generated event/observation ID, source timestamp,
  snapshot digest, or redacted raw-event digest.
- Source output, target identity, and Run authority are content- and
  context-bound without making source fulfillment a separate Core commit;
  target relevance remains independently assessed.
- A failed `record_run` leaves no producer, persistent artifact, observation,
  event, replay row, or state increment. Its only possible residue is an
  expiring safe staged receipt.
- The producer record is the canonical execution/observation receipt. Its
  promoted artifact is a bounded output receipt and not a second authority
  body.
- Checked-in Codex and Claude Code fixtures establish parser compatibility only.
  Real-host support claims require opt-in live validation of invocation IDs,
  output completeness, retries, resumes, and parallel calls.

## Rejected alternatives

- Caller-created producer records were rejected because they make claimed
  provenance self-authorizing.
- Treating artifact integrity, `SourceRef`, tool metadata, or a raw guard event
  as the producer was rejected because none independently binds source,
  current basis, target relevance, and exact output.
- Treating a matching source outcome as supported relevance was rejected
  because successful execution or observation cannot self-approve an arbitrary
  criterion.
- Creating a persistent producer immediately in a hook was rejected because it
  separates producer authority from Run finalization and adds a run-less
  persistent artifact transaction.
- Correlating tool events by session and time was rejected for Strong Evidence
  because concurrent, retried, and resumed invocations can collide.
- Waiting for host-turn or process exit before claiming an already durable
  source was rejected because a Detective Stop can legitimately deny while the
  Task is not close-ready and the immutable intent can expire during that
  unrelated lifecycle wait.
- Binding a connection intent to a digest over a future host-generated event or
  watcher observation was rejected because a post-intent source identity,
  timestamp, and snapshot/raw-event digest are not caller-known intent facts.
- Persisting raw command or tool output was rejected because it expands secret,
  privacy, retention, and response-budget risk without improving authority.
- Reusing the v3 storage profile for a new canonical shape was rejected because
  it would make compatibility diagnostics dishonest.

## Relevant implementation and owners

- `crates/volicord-core/src/methods`: intent creation and Run finalization
- `crates/volicord-store/src`: source receipt staging and insert-only records
- `crates/volicord-cli/src`: administrative command and registered-source adapters
- `crates/volicord-mcp/src`: intent-only Agent Connection tool projection
- [`volicord.prepare_evidence_capture`](../../reference/api/method-prepare-evidence-capture.md)
- [Host Release Evidence](../../reference/host-release-evidence.md)
- [Core Model](../../reference/core-model.md#9-evidence-and-run-authority)
- [Storage Effects](../../reference/storage-effects.md)
- [Storage Versioning](../../reference/storage-versioning.md)
