# Request Lifecycle

This guide follows a supported request from managed stdio through adapter
validation, Core planning, Store access, commit, and response projection.

## End-To-End Flow

```text
Codex -> stdio MCP -> public argument DTO -> Core request -> plan
      -> Store read/validation -> optional atomic commit -> public result
      -> MCP projection -> Codex
```

1. The stdio process resolves its managed launch context, validates the
   Connection and project selection, opens the exact StorageManifest, and
   records the authoritative runtime and project sessions.
2. JSON-RPC validates lifecycle, method name, and public argument object.
3. For each project tool call, the MCP adapter rejects hidden envelope or
   invocation fields, validates the current managed runtime/project session,
   and builds the complete Core request from server-owned context.
4. Core common preflight validates the typed Agent Session, actor, operation
   category, project, replay identity, expected state, current Task context,
   and structural input.
5. The method planner reads one coherent snapshot and produces typed
   method-specific result fields plus exact proposed effects. Shared semantic
   work is delegated to focused identity, artifact, continuity, evidence,
   Write Ticket, close-readiness, and projection owners. Those owners return
   typed facts or plans and do not construct method responses.
6. The shared pipeline selects the typed read-only, no-effect, dry-run, or
   committed branch. Mutation branches revalidate commit preconditions and
   apply one atomic Store transaction.
7. After the branch effect, state-version, event, and replay facts are known,
   the pipeline constructs and serializes the complete public response once.
   MCP projects the owner-defined detail without changing authority meaning.

Failures before Core have no Core or Store effects. A failure after commit
preserves operation-result recovery coordinates and never retries the mutation
implicitly.

## Read-Only Requests

`volicord.status`, `volicord.check_close`, and eligible
`volicord.get_operation_result` calls use a coherent read snapshot. They do not
create replay rows, authority events, current pointers, or state-version
advances. Typed pagination cursors are validated before lookup.

## Structural Rejection

Structural input validation precedes policy and storage mutation. In particular,
`volicord.prepare_write` rejects a missing current Change Unit with
`NO_ACTIVE_CHANGE_UNIT` and `details.reason=current_change_unit_required` before
ticket lookup, invalidation, policy evaluation, or any other effect. This is
`Rejected`, not a policy `NotAllowed` decision.

## Mutation Planning And Commit

Planners return a closed outcome, a typed method-fields value, and exact commit
inputs. One public-method declaration in
[`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)
binds the method name, request and result types, exact response family,
contract IDs, schemas, and committed-result replay eligibility. The shared
declaration also owns the exact successful result effects. The shared pipeline
carries the declared fields and method-specific base types through branch
selection and rejects an effect that the method's response family does not
contain. Store
performs owner-defined final validation inside the transaction, inserts
immutable rows, updates current pointers, appends authority events and replay
when applicable, and advances `state_version` exactly once.

For a method-result branch, the pipeline constructs that method's exact result
metadata only after those common facts are available. Compile-time branch
capabilities expose only the method's declared read-only, no-effect, staging,
or committed constructors. Effect-specific types own fixed dry-run and event
cardinality facts. Rejection and dry-run responses use their distinct
`ToolRejectedBase` and `ToolDryRunBase` metadata types. Strict decoding rejects
unknown, cross-branch, and method-impossible effect fields before an untagged
family can select another branch.

CLI and MCP adapters decode the returned public object as that method's exact
response family before rendering or protocol projection. Adapter carriers
therefore cannot introduce a response branch that Core and the method schema
do not declare.

Rejected, dry-run, unavailable, corrupt, unsupported-contract, and conflict
branches follow [Storage Effects](../reference/storage-effects.md). They never
borrow effects from a nearby success branch.

## Record Run Flow

After common preflight, the public `record_run` entry point delegates to the
recording package. `recording/context.rs` normalizes the request and acquires
the current typed operation facts. Capture-intent and receipt resolution,
evidence observation and producer planning, and artifact verification or
promotion planning then run through `recording/authority.rs`,
`recording/evidence.rs`, and `recording/artifact.rs`, which reuse the focused
evidence and artifact owners.

The Write Ticket owner admits any required ticket from semantic operation
facts. The close-readiness recording owner resolves close-basis references and
residual-risk inputs. `recording/plan.rs` combines these results into one typed
mutation plan, preserving distinct domain mutation variants and Store
application order. `recording/state.rs` acquires the post-operation state facts,
and Store-free `recording/projection.rs` constructs the method fields. Only the
public method module maps semantic errors into response branches; the shared
pipeline retains transaction, replay, and response-envelope ownership.

## Write Ticket Flow

`prepare_write` evaluates current Task, Change Unit, scope, baseline, policy,
sensitive approval, normalized paths, and current write-authority fingerprint
through `crates/volicord-core/src/write_ticket/`.
An existing ticket is reusable only when every owner-defined coordinate remains
valid. For `record_run`, `write_ticket/admission.rs` receives the typed Task,
Change Unit, invocation, observed-change, policy-fingerprint, and operation
facts. It applies the same validity and attempt-scope policy without decoding
physical rows or constructing a method response. The recording plan consumes
only an exactly matched admitted ticket inside the same commit as the Run.

## UserAction Separation

`volicord.request_user_action` creates a strict pending request or uses its
explicit read-only resume branch. The MCP adapter returns only an agent-safe
summary and current projection. It never renders or submits the resolving form.

The local CLI inbox reads the strict stored form and calls
`volicord.resolve_user_action` with local-user provenance. Resolution is a
separate user-only mutation and never replaces the original request result.
Guard prompt observations remain observations.

## Guard Suppression

Reconciliation calls the bounded suppression service. `Applied` contains exact
remaining paths and suppression records. `Unavailable` preserves all observed
paths, reason, scan budget, and observed count. Store failure or corrupt
correlation never becomes an empty success.

## Response Projection

Public method results remain the authority-bearing response. Their flat JSON
shape and generated schema come from the complete public result type, while
method planning handles only its method-specific fields. A committed replay row
stores that complete serialized result and replay decoding validates the same
current type. MCP structured content conforms to the advertised schema; text is
a bounded human rendering. Compact schemas and summary views may omit display
detail but cannot omit required authority coordinates or relax server
validation.

## Related Owners

- [MCP Transport](../reference/mcp-transport.md)
- [API Methods](../reference/api/methods.md)
- [Storage Effects](../reference/storage-effects.md)
- [Failure Model](../reference/failure-model.md)
- [Guard Suppression](../reference/guard-suppression.md)
