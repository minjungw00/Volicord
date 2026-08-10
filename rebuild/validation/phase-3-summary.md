# Phase 3 architecture conclusion

- Phase 4 architecture-contract gate: `blocked`
- Unblock condition: corrected Phase 3 contracts and strengthened architecture checks pass
  an independent final validation session

## Current architecture contract

All nine Phase 3 owners remain active with the subsystem graph and accepted product
Decisions unchanged.

| Active owner | Current responsibility relevant to the gate |
|---|---|
| `architecture.md` | Logical subsystem graph, Guarded confirmation transport/dispatch ownership, and D/Q/Acceptance/later-validation traceability |
| `domain-model.md` | Canonical relation direction plus Candidate identity, metadata, lifecycle, provenance and promotion meaning |
| `repository-intelligence.md` | Snapshot-bound polyglot analysis, capability, coverage, freshness, provenance and adapter boundaries |
| `privacy-and-provider-boundary.md` | Provider authority plus Candidate collection opt-out, retention and managed deletion boundaries |
| `inquiry-and-decision.md` | Question Candidate, frontier, exact response linkage, terminal outcomes and Decision reuse |
| `projections-and-documents.md` | Recall/documents plus the read-only Candidate Inspection projection and degradation boundary |
| `portable-context.md` | Portable content, binding, source-independent read, divergence, conflict and merge provenance |
| `versioning-policy.md` | Independent canonical, bundle, analysis, index and generated-document version behavior |
| `failure-and-recovery.md` | Cross-subsystem failure plus Guarded confirmation/execution and Candidate Inspection failure behavior |

## Corrected relation direction

The maintained relation-direction contract uses one direction for each relation:

- `supported_by`: statement-bearing record, Decision rationale or Checkpoint observation
  → supporting `Source`
- `derived_from`: Session Candidate, Derived State or generated explanation
  → used `Source` or canonical basis

No reverse alias under either relation name is part of the active contract.

## Candidate inspection and privacy boundary

Candidate meaning and lifecycle remain owned by `domain-model.md`. The named Candidate
Inspection read projection is owned by `projections-and-documents.md`; collection opt-out,
retention and deletion are owned by `privacy-and-provider-boundary.md`. The read projection
does not promote, correct, dismiss, expire, delete or reinterpret Candidate data.

Q10 and Acceptance G route Candidate retention/opt-out to V07, promotion/inspection to
V09, and the integrated journey to V11. Acceptance K routes privacy/deletion propagation
to V07 in addition to portable conflict behavior in V04.

## Guarded-effect boundary

The accepted high-risk categories remain unchanged. Host and User Adapters own current-host
confirmation transport and viewer/CLI fallback. Local Operations owns exact confirmation
validation, single-use consumption and effect dispatch. Guarded execution does not occur
before valid confirmation, while ordinary code edits, local tests, inventory and structural
analysis remain non-blocking.

Operational confirmation is cooperative rather than an OS sandbox or security-enforcement
claim, and it is not a seventh canonical core entity. Q12 and Acceptance M route host/fallback
behavior to V08 and integrated exact-match, non-reuse, execution outcome and ordinary-action
behavior to V11.

## Gate status

The architecture-contract checker must be strengthened with deterministic positive and
negative coverage for these maintained structures. This summary does not attest that the
corrected architecture or checker has passed the final aggregate audit. The Phase 4 gate
remains `blocked` until an independent final validation session passes.

## Maintained authorities

- `rebuild/docs/design/product-charter.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
- `rebuild/docs/design/validation-plan.md`
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
- `rebuild/docs/design/cutover-plan.md`
- `rebuild/scripts/check-architecture-contracts`
- `rebuild/scripts/validate`
