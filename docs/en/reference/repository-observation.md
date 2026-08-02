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
ambient repository status, or tool name alone is not correlation.

## Observation Terms

- The **exact host invocation** is the complete project, Connection, session,
  turn, tool-use ID, canonical tool-name, and compatible Guard-event
  coordinate above.
- The **repository baseline** is the exact stable `PreToolUse` snapshot
  persisted for that invocation.
- The **repository outcome** is the exact stable matching `PostToolUse`
  snapshot.
- The **repository delta** is the deterministic net transition from that
  baseline to that outcome.
- The **unmatched delta** is the non-empty portion of a complete repository
  delta not covered by that observation's exact expected write.
- **Observation unavailable** is a closed observation result that does not
  claim a complete repository delta.

## Regular-File Content Evidence

A directly observed regular file retains typed evidence for both content
domains:

- the SHA-256 identity of the exact worktree bytes read from the file; and
- the canonical Git blob identity produced by streaming those same bytes
  through Git's current path-aware clean conversion for the canonical Product
  Repository-relative path.

The conversion is read-only. It uses Git's repository, path, attributes,
configuration, encoding, and clean-filter context without writing the object.
The exact-byte hash and Git conversion consume one file stream. An immutable
tree-derived regular file carries its canonical Git blob identity and no
worktree-byte identity. Regular-file state also carries the executable bit.

Regular-file comparison uses one centralized rule:

- a regular file and any non-regular state differ;
- executable-bit differences differ;
- two directly observed worktree files compare by exact worktree-byte
  identity;
- any comparison involving a tree-derived regular file compares canonical Git
  blob identity; and
- two tree-derived regular files compare by canonical Git blob identity.

Symbolic links retain exact typed target identity, and Gitlinks retain their
canonical checked-out commit identity. Git conversion failure, filter failure,
timeout, malformed output, containment failure, or resource-limit exhaustion
produces observation unavailable rather than a successful partial or empty
snapshot.

## Observation States

```schema
RepositoryObservationState:
  open
  complete
  unavailable
```

`open` contains the strictly decoded canonical repository baseline and its
verified digest. It contains no post-tool event, repository outcome, repository
delta, unavailable reason, completion time, or terminal result.

`complete` contains the exact matching post-tool event, a strictly decoded
canonical repository outcome, the deterministic net repository delta, verified
snapshot and delta digests, and the completion time. Its delta may be empty.

`unavailable` contains one closed reason and a completion time. It does not
claim a complete delta. A valid baseline may remain present when the
invocation was denied or post-tool completion became unavailable.

An `open` observation closes at one of three exact lifecycle boundaries:

- exact matching `PostToolUse` produces `complete` or `unavailable` from the
  post-tool observation result;
- the next accepted `UserPromptSubmit` in the same managed project session
  closes open observations from different established turns as
  `unavailable(post_tool_not_observed)`; observations in the prompt's exact
  current turn remain open so parallel tool calls can finish; and
- authoritative termination of the owning `managed_host` runtime closes the
  remaining observations in its exact bounded project-session bindings as
  `unavailable(managed_session_terminated)`.

The closed stored unavailable-reason set is:

```text
invalid_observer_limits
invalid_repository_root
not_git_repository
git_layout_unavailable
git_command_unavailable
git_command_failed
process_timeout
git_output_limit_exceeded
process_input_limit_exceeded
candidate_path_limit_exceeded
total_hash_bytes_limit_exceeded
file_size_limit_exceeded
serialization_depth_limit_exceeded
serialization_size_limit_exceeded
invalid_relative_path
non_utf8_path
path_outside_repository
inaccessible_path
unsupported_path_state
unstable_repository
repository_identity_changed
observer_contract_mismatch
git_object_unavailable
invocation_denied
missing_open_observation
post_tool_not_observed
managed_session_terminated
```

The last two reasons close an open observation with a terminal lifecycle result
when PostTool is absent. A terminal row remains stable under replay.

Turn identity is exact typed equality, never lexical or numeric ordering. The
accepted prompt capture and its prior-turn terminalization share one immediate
bounded project transaction. Runtime cleanup uses only exact Registry
project-session bindings. Runtime Home recovery repeats that cleanup only for
Registry sessions already authoritatively terminal; replay leaves terminal
rows unchanged.

Lifecycle terminalization retains the pre-tool baseline, records the reason,
completion time, and one stable terminal result, and leaves the delta
unavailable. It performs no Product Repository scan, expected-write match,
write-ticket consumption, Unrecorded Change or finding creation, synthetic
path creation, actor attribution, or causation claim. A failed validation
rolls back the complete bounded project transaction.

Terminal observations do not return to `open`. Exact replay returns the stored
terminal result without rescanning the Product Repository. A conflicting
second `PostToolUse` event is rejected.

Snapshot, outcome, delta, and bounded metadata decoders reject unknown fields,
malformed Product Repository paths, invalid state combinations, noncanonical
encoding, duplicate or unordered transitions, semantically empty transitions,
and digest mismatch as corrupt stored data.

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
