# Phase 3 architecture conclusion

- Phase 4 architecture-contract gate: `ready`
- Scope: logical architecture consistency and accepted-contract completeness
- Excluded claim: production implementation and later-validation behavior have not been
  validated by this conclusion

## Active architecture owners

All nine Phase 3 architecture owners are `active`, uniquely routed, and bounded by the
ownership precedence in `architecture-inputs.md`.

| Active owner | Owned architecture contract |
|---|---|
| `architecture.md` | Logical subsystem map, cross-subsystem dependency direction, integration boundaries, Guarded confirmation/dispatch, and cross-owner conflict resolution |
| `domain-model.md` | Information classes, six canonical core entities, identity, provenance, relation direction, Candidate lifecycle, promotion, correction, supersession, contradiction, review and forgetting |
| `repository-intelligence.md` | Repository/Analysis Snapshot, inventory, normalized analysis envelopes, capability, coverage, freshness, provenance and analyzer-adapter boundaries |
| `privacy-and-provider-boundary.md` | Local/interactive/background authority, Project opt-in, transmission scope, Candidate collection opt-out/retention, revoke and managed deletion |
| `inquiry-and-decision.md` | Question Candidate, material frontier, exact response linkage, terminal outcomes, Decision applicability, reuse and Checkpoint interaction |
| `projections-and-documents.md` | Recall, Candidate Inspection, map and document projections, grounding, preview, adoption and output-format boundaries |
| `portable-context.md` | Portable bundle, Project/clone binding, source-independent read, divergence, conflict, resolution and merge provenance |
| `versioning-policy.md` | Independent canonical schema, bundle, Analysis Snapshot, Derived Index and generated-document metadata version behavior |
| `failure-and-recovery.md` | Cross-subsystem failure, propagation, retry, repair/rebuild, Guarded execution, Candidate Inspection degradation and long-operation recovery |

Specialized owners use the subsystem direction from `architecture.md` and the information,
identity, provenance, relation and lifecycle meanings from `domain-model.md`; none defines a
second core architecture or canonical domain model.

## Architecture checks

| Maintained check | Result |
|---|---|
| `rebuild/scripts/validate self-test` | `passed`; complete stream, duration, termination, exit-status, exact final-command and non-fail-fast aggregate behavior verified |
| `rebuild/scripts/check-architecture-contracts --self-test` | `passed`; positive fixture and 18 deterministic negative cases, including relation, Candidate and Guarded regressions |
| `rebuild/scripts/check-architecture-contracts` | `passed`; nine owners, links, owner routing, traceability, corrected trust boundaries and prohibited-path checks verified |

The checker is structural test support. Product meaning remains in the active architecture
owners and accepted product Decisions.

## Relation direction

The relation-direction contract is consistent in its table, prose, diagrams and projection
examples:

- `supported_by`: statement-bearing record, Decision rationale or Checkpoint observation
  → supporting `Source`.
- `derived_from`: Session Candidate, Derived State or generated explanation
  → the `Source` or canonical basis actually used.

These directions support traversal from a claim to its evidence, from rebuildable or
generated state to its invalidation basis, and from a projection to its grounding. Portable
context preserves canonical relation identity and direction; neither relation has a reverse
alias with the same name.

## Candidate contract

`domain-model.md` remains the owner of Candidate information-class meaning, identity,
metadata, lifecycle and promotion. `privacy-and-provider-boundary.md` explicitly owns scoped
automatic collection, opt-out, retention/expiry and deletion. The named `Candidate
Inspection` projection in `projections-and-documents.md` is read-only.

Candidate Inspection exposes existence/identity, kind, origin/provenance, collection scope,
creation/observation basis, retention/expiry, promotion disposition and relevant opt-out
state. Reading or failing to read the projection does not promote, correct, dismiss, expire,
delete, rewrite or reinterpret a Candidate. Projection failure is isolated as degradation,
not Canonical Context mutation.

Scoped opt-out stops new automatic collection after its effective basis and does not silently
change existing Candidates. Existing Candidates remain inspectable until explicit deletion,
dismissal, promotion or retention expiry. Full prompts, full tool arguments, full Source
bodies and unlimited stdout/stderr streams remain outside default long-term collection.

Q10 retains `domain-model.md` as its primary owner. Acceptance G routes scoped
collection/retention to V07, promotion and read-only inspection to V09, and the integrated
journey to V11. Acceptance K additionally routes privacy and managed-deletion propagation to
V07.

## Guarded-effect contract

The accepted Guarded boundary represents all nine high-risk categories from Q12 while
ordinary code edits, local tests, repository inventory and local structural analysis remain
outside Guarded admission. A Guarded Effect Candidate and confirmation/operation state are
Derived or operational state, not a seventh canonical core entity or a general product
Decision.

Each immutable confirmation request revision includes request identity and revision, exact
action, target, expected effect, risk, scope, expiration, requesting actor/provenance and an
exact-match fingerprint. The explicit response is linked to the exact request revision and a
current-host user-turn `Source`.

Confirmation is action-, target- and scope-bound, expiring, single-use and non-transferable.
Changed action, target, expected effect, scope, request revision or expiration, an expired
request, or prior consumption requires a new valid confirmation. Current-host, viewer and CLI
paths carry the same logical request identity, revision and Source linkage. Local Operations
validates the exact request before dispatch and links confirmation consumption and effect
dispatch through one operation identity.

Missing, denied, stale, expired, mismatched, reused, dispatch-failed, execution-failed and
execution-indeterminate outcomes remain distinct. Invalid confirmation never silently
dispatches an effect; uncertain execution is not success and is not silently retried. Durable
history may reference the response Source and resulting operation from a Checkpoint or Context
Item without adding another canonical entity. The contract is cooperative confirmation, not
an OS sandbox or security-enforcement guarantee.

Q12 retains `architecture.md` as its primary owner. Acceptance M routes current-host and
viewer/CLI transport behavior to V08 and exact validation, non-reuse, outcome handling and
ordinary-action behavior to V11.

## Accepted-Decision and acceptance traceability

- D1–D12 each have one authoritative architecture owner and retain their accepted scope.
- Q1–Q13, including Q8-A and Q8-B, each have one authoritative architecture owner and retain
  their accepted scope.
- Acceptance A–Q each has one owning architecture interface, future implementation phase and
  later-validation route.
- V02, V04, V06, V07, V08, V09, V10 and V11 each have an explicit Phase 3 interface owner and
  a precise later-validation contract.

No product implementation technology is selected by the active owners. Wave 1 prototypes
remain evidence-only validation support, and no production reconstruction crate or runtime
behavior is changed by the Phase 3 trust-boundary documentation or architecture checker.

## Later-validation ownership and known limits

| Validation owner | Remaining evidence-backed limit |
|---|---|
| V02 — `repository-intelligence.md`, `versioning-policy.md` | Production semantic normalization, source ranges, diagnostics, incomplete-build degradation and at least three ecosystem adapters remain unvalidated. |
| V04 — `portable-context.md`, `versioning-policy.md` | Common-base discovery, all six conflict classes, user-owned resolution, deletion propagation, merge provenance and bundle-version interaction remain unvalidated. |
| V06 — `projections-and-documents.md`, `versioning-policy.md` | Four-document grounding and omission, Markdown/HTML equivalence, stale behavior, preview purity and explicit adoption remain unvalidated. |
| V07 — `privacy-and-provider-boundary.md` | Local-only operation, provider opt-in/transmission scope, Candidate opt-out/retention, managed deletion and privacy propagation remain unvalidated. |
| V08 — `architecture.md`, `failure-and-recovery.md` | Linux/Codex installation, init/bind/health, current-host Guarded transport, viewer/CLI fallback and process cleanup remain unvalidated. |
| V09 — `inquiry-and-decision.md`, `projections-and-documents.md` | Recall selection, Checkpoint accuracy, Decision reuse, Candidate promotion/disposition, Candidate Inspection completeness and no-mutation behavior remain unvalidated. |
| V10 — `failure-and-recovery.md`, `versioning-policy.md` | Process/filesystem primitives, complete stream and termination observation, child cleanup, atomic publication, corruption, repair/rebuild and upgrade failure remain unvalidated. |
| V11 — `architecture.md` and all specialized owners | The multi-repository integrated Candidate, Inquiry, Guarded, portability, document and degraded-recovery journey remains unvalidated. |

These limits are assigned validation work, not production guarantees and not evidence that an
accepted product Decision is infeasible.

## Decision revisit triggers

All recorded Q1–Q13 revisit triggers are inactive on the maintained Phase 3 evidence. Q8-B
has no recorded revisit trigger and its clean-product boundary remains unchanged. The
outstanding V02/V04/V06–V11 limits do not themselves satisfy a revisit trigger; a later
validation must preserve its evidence and use the Decision revisit procedure if it
demonstrates one.

## Phase 4 gate

`ready`. The nine-owner architecture is semantically consistent, relation direction is
usable across grounding and invalidation, Candidate and Guarded trust boundaries are complete,
accepted Decisions and Acceptance A–Q remain traceable, maintained structural checks pass, and
no unresolved architecture contradiction or active Decision revisit trigger blocks Phase 4
architecture-contract work.

This status does not certify production behavior or complete V02, V04, V06, V07, V08, V09,
V10 or V11.

## Maintained architecture and validation references

- `rebuild/docs/design/product-charter.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/cutover-plan.md`
- `rebuild/docs/design/architecture-inputs.md`
- `rebuild/docs/design/architecture.md`
- `rebuild/docs/design/domain-model.md`
- `rebuild/docs/design/repository-intelligence.md`
- `rebuild/docs/design/privacy-and-provider-boundary.md`
- `rebuild/docs/design/inquiry-and-decision.md`
- `rebuild/docs/design/projections-and-documents.md`
- `rebuild/docs/design/portable-context.md`
- `rebuild/docs/design/versioning-policy.md`
- `rebuild/docs/design/failure-and-recovery.md`
- `rebuild/validation/wave-1-summary.md`
- `rebuild/validation/repository-intelligence/polyglot-structural/report.md`
- `rebuild/validation/canonical-context/portability/report.md`
- `rebuild/validation/inquiry/frontier-resume/report.md`
- `rebuild/scripts/check-architecture-contracts`
- `rebuild/scripts/validate`
