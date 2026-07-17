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

## Required Tests

Durable contract tests cover:

- applied suppression with an exact unchanged recorded identity;
- applied outcome with no suppression candidate;
- exactly 512 candidates and more than 512 candidates;
- corrupt stored event and malformed correlation payload;
- Run and write-ticket lookup failures;
- path-identity calculation failure;
- Store read failure preserving every input path; and
- warning, diagnostic, and event reason projection without sensitive payloads.

## Adjacent Owners

- Product path normalization: [Runtime Boundaries](runtime-boundaries.md).
- Failure category meanings: [Failure Model](failure-model.md).
- Write-ticket and Run state shapes: [API State Schemas](api/schema-state.md).
- Guard release scenario: [Host Release Evidence](host-release-evidence.md).
- Security and diagnostic non-guarantees: [Security](security.md).
