# Repository Observation

This document owns the invocation-scoped Product Repository observation
contract used by Guard, expected writes, and Unrecorded Changes.

## Surface Stability

The observation states, exact host correlation, snapshot and delta integrity,
expected-write matching, unavailable behavior, and Unrecorded Change creation
rules are stable. Observer implementation and bounded resource limits are
internal.

## Exact Invocation

One repository observation belongs to exactly one compatible Codex tool
invocation. Its semantic coordinate contains:

- project and Agent Connection identity;
- exact host session, host turn, host tool-use ID, and canonical host tool
  name;
- the compatible `PreToolUse` Guard event and, when the invocation completes,
  the exact matching compatible `PostToolUse` Guard event;
- the current Guard Installation when the managed Guard boundary requires it;
  and
- the semantic repository-observer contract digest.

The Store permits at most one observation for this complete invocation
coordinate. A different turn, tool-use ID, tool name, Connection, project, or
Guard event cannot satisfy the coordinate. A reported effect, path hint,
current dirty-path list, or tool name alone is not correlation.

## Observation States

```schema
RepositoryObservationState:
  open
  complete
  unavailable
```

`open` contains a strictly decoded canonical pre-tool snapshot and its verified
digest. It contains no post-tool event, post-tool outcome, delta, unavailable
reason, completion time, or terminal result.

`complete` contains the exact matching post-tool event, a strictly decoded
canonical post-tool snapshot, the deterministic net repository delta, verified
snapshot and delta digests, and the completion time. Its delta may be empty.

`unavailable` contains one closed reason and a completion time. It does not
claim a complete delta. A valid baseline may remain present when the
invocation was denied or post-tool completion became unavailable.

Terminal observations do not return to `open`. Exact replay returns the stored
terminal result without rescanning the Product Repository. A conflicting
second `PostToolUse` event is rejected.

Snapshot, outcome, delta, and bounded metadata decoders reject unknown fields,
malformed Product Repository paths, invalid state combinations, noncanonical
encoding, duplicate or unordered transitions, and digest mismatch as corrupt
stored data.

## Pre-Tool Aggregate

For every compatible tool invocation, Guard attempts to capture a stable
pre-tool Product Repository snapshot. It parses the typed
`CodexHookToolCorrelation`; it does not search a generic invocation field.

For `may_write_product` and `unknown_product_effect`, a stable baseline must be
captured and persisted before Guard returns an allow decision. Baseline capture
or aggregate-persistence failure denies the invocation with a typed reason.

For `no_product_write`, unavailable observation is recorded explicitly and the
invocation may continue. The result does not claim that no Product Repository
change was observed. A successfully captured baseline remains able to detect a
non-empty post-tool delta from a tool declared read-only.

One immediate Store transaction records:

- the compatible `PreToolUse` Guard event;
- the exact host invocation;
- an `open` observation with its baseline, or a terminal `unavailable`
  observation;
- the exact expected write when current write authority warrants one.

Any failure rolls back the complete aggregate. A denied invocation leaves no
`open` observation that expects a post-tool event. Guard returns its host
decision only after the required transaction commits.

## Post-Tool Aggregate

Guard captures a stable post-tool snapshot for the exact matching invocation.
Host-provided paths may be bounded candidate hints only when the selected host
contract owns that field. They are not Product Repository changes by
themselves.

One immediate Store transaction:

1. loads and strictly verifies the exact `open` observation;
2. records the compatible matching `PostToolUse` Guard event;
3. stores the complete post snapshot and deterministic net delta, or closes the
   observation as `unavailable`;
4. matches the exact observation's expected write against a complete delta;
5. creates an Unrecorded Change only for a non-empty unmatched portion; and
6. stores and returns the stable terminal result.

A missing, conflicting, corrupt, or unavailable baseline produces an explicit
unavailable result and diagnostic. It does not produce an empty delta or an
Unrecorded Change. An empty complete delta creates no Unrecorded Change and
does not satisfy an expected write.

The delta records the net Product Repository transition during the exact
invocation window. It does not claim actor identity or exclusive causation.

## Expected Writes

Each expected write belongs to exactly one repository observation and its exact
host tool invocation. It carries a non-empty, canonical, duplicate-free path
set covered by the current write authority.

Only a complete non-empty delta can match an expected write:

- exact covered paths are recorded as matched;
- a fully covered delta creates no Unrecorded Change;
- a partially covered delta creates one Unrecorded Change for only the
  additional paths; and
- an empty or unavailable observation leaves the expected write unmatched.

No session-only lookup, time-window lookup, post-event search, or alternate
invocation identifier participates in matching.

## Unrecorded Changes

An Unrecorded Change represents only a complete observed unmatched delta. Its
observed path set is non-empty, canonical, sorted, and duplicate-free. It links
to the exact repository observation and stores the deterministic unmatched
delta digest.

Identity is derived from the project, exact repository observation, and
unmatched delta digest. The Guard event ID is not an identity salt. Replaying
the same terminal observation is idempotent.

An unresolved Unrecorded Change participates in reconciliation and close
readiness. Observation-unavailable diagnostics remain separate operational
facts and never become synthetic path findings.

## Guard Outcome Boundary

Guard returns one typed repository-observation result containing:

- observation state and exact observation identity;
- the complete delta summary when state is `complete`;
- the closed unavailable reason when state is `unavailable`;
- exact expected-write match facts; and
- Unrecorded Changes created from the unmatched delta.

The Codex adapter owns host JSON, context, warning, denial, stderr, and exit
projection. `PostToolUse` output describes an invocation that already
completed and never claims prevention or reversal.

Guard and repository observations are cooperative local records. They do not
prove actor identity, intent, complete monitoring, OS enforcement, or
correctness.

## Related Owners

- [Runtime Boundaries](runtime-boundaries.md)
- [Storage Records](storage-records.md)
- [Storage DDL](storage-ddl.md)
- [Storage Effects](storage-effects.md)
- [Storage Versioning](storage-versioning.md)
- [Security](security.md)
- [Reconcile Changes](api/method-reconcile-changes.md)
- [State Schemas](api/schema-state.md)
