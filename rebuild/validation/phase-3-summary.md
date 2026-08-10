# Phase 3 architecture conclusion

- Architecture baseline: `44b6985f115c84cc227e918a55384b36cf114219`
- Phase 4 architecture-contract gate: `ready`

## Architecture document status

| Active owner | Status | Evidence-backed contract status |
|---|---|---|
| `architecture.md` | `active` | Defines one acyclic logical subsystem graph, cross-subsystem dependency direction, integration boundaries, traceability, and the bounded Phase 4 handoff. |
| `domain-model.md` | `active` | Keeps Canonical Context, Session Candidate, and Derived State distinct and owns core identity, provenance, relations, promotion, and lifecycle meaning. |
| `repository-intelligence.md` | `active` | Defines snapshot-bound polyglot inventory and analysis envelopes, capability, coverage, freshness, provenance, and adapter responsibilities without selecting production analyzers. |
| `privacy-and-provider-boundary.md` | `active` | Separates local processing, current-host interactive access, and Project-opted-in background provider authority while preserving local-only operation. |
| `inquiry-and-decision.md` | `active` | Defines Question Candidate promotion, deterministic frontier behavior, exact current-host response linkage, terminal outcomes, Decision reuse, and re-questioning. |
| `projections-and-documents.md` | `active` | Defines bounded Recall, read-only maps and documents, grounding, omission, preview, publication, and explicit adoption without projection-side canonical mutation. |
| `portable-context.md` | `active` | Defines portable content, local binding separation, source-independent reads, divergence, six conflict classes, resolution authority, and merge provenance without choosing an algorithm. |
| `versioning-policy.md` | `active` | Defines independent canonical, bundle, analysis, index, and document-metadata version behavior, including read, write, upgrade, and rebuild responsibility. |
| `failure-and-recovery.md` | `active` | Defines cross-subsystem failure, degradation, propagation, retry ownership, repair/rebuild separation, canonical/projection failure separation, and long-operation results. |

The specialized owners reference the subsystem and core-domain contracts in
`architecture.md` and `domain-model.md`; they do not introduce competing
definitions for those contracts.

## Architecture-contract checker

| Interface | Result |
|---|---|
| `rebuild/scripts/check-architecture-contracts --self-test` | `passed` — the maintained positive fixture and all nine negative cases behaved as required. |
| `rebuild/scripts/check-architecture-contracts` | `passed` — all nine active owner documents, owner routing, required structure, links, traceability identifiers, maintained report paths, prohibited-path boundaries, and Phase 4 handoff structure passed. |

## Accepted-Decision traceability

`architecture.md` section 8 contains exactly one primary architecture-owner row
for every accepted D1–D12 constraint and for Q1–Q7, Q8-A, Q8-B, and Q9–Q13.
The routed contracts preserve the accepted polyglot capability, canonical and
derived separation, user-Decision authority, local-only operation, portable
context, Recall and Checkpoint, projection purity, risk boundary, and clean
product graph without adding an unsupported implementation choice.

Status: `complete` for the Phase 3 architecture contract.

## Acceptance A–Q traceability

`architecture.md` section 9 contains exactly one row for every Acceptance
scenario A–Q. Every row identifies a primary architecture owner, a later
implementation phase, and an owning executable validation. The routes retain
the complete scenario boundaries in `acceptance-scenarios.md`; the traceability
table does not replace those scenarios.

Status: `complete` for the Phase 3 architecture contract.

## Later-validation interfaces and known limits

These interfaces are defined architecture inputs for later executable work.
Their validation remains outstanding and this gate does not attest production
behavior.

| Validation | Interface status | Known limit retained by the validation |
|---|---|---|
| V02 | `defined` in `repository-intelligence.md` and `versioning-policy.md` | Production semantic adapters, the first three ecosystems, normalization accuracy, build degradation, packaging, and snapshot-version behavior remain unvalidated. |
| V04 | `defined` in `portable-context.md` and `versioning-policy.md` | Common-base discovery, the concrete merge algorithm, all six conflict classes, deletion propagation, user resolution, and bundle-version interaction remain unvalidated. |
| V06 | `defined` in `projections-and-documents.md` and `versioning-policy.md` | Four-document grounding quality, omission completeness, Markdown/HTML equivalence, stale invalidation, and adoption behavior remain unvalidated. |
| V07 | `defined` in `privacy-and-provider-boundary.md` | Opt-in enforcement, actual transmission scope, exclusion and secret filtering, revoke behavior, provider retention, managed deletion, and the local-only journey remain unvalidated. |
| V08 | `defined` in `architecture.md` and `failure-and-recovery.md` | Linux installation, Codex lifecycle, init/bind/health behavior, process cleanup, locale output, and clean-runtime operation remain unvalidated. |
| V09 | `defined` in `inquiry-and-decision.md` and `projections-and-documents.md` | Recall selection quality, omission reporting, no-mutation behavior, Decision repetition, meaningful Checkpoint detection, and dirty-change attribution remain unvalidated. |
| V10 | `defined` in `failure-and-recovery.md` and `versioning-policy.md` | Process and filesystem primitive choice, complete stream and termination capture, child cleanup, atomic publication, corruption handling, repair/rebuild, and upgrade failure remain unvalidated. |
| V11 | `defined` across `architecture.md` and all specialized owners | Combined multi-repository behavior, end-to-end portability, privacy, projection, versioning, and failure recovery remain unvalidated. |

## Decision revisit triggers

- D1–D12 remain `accepted`; Phase 3 evidence demonstrates no condition requiring
  a new product question.
- Q1–Q7, Q8-A, and Q9–Q13 revisit triggers are `inactive`. Phase 3 preserves
  their accepted scope and records their outstanding evidence under the owning
  later validations above.
- Q8-B has no revisit trigger in the accepted Decision register.

## Phase 4 architecture-contract gate

`ready`. The architecture is internally consistent, keeps Canonical Context
independent of analyzers, providers, hosts, and projections, assigns one active
owner per concept, and supplies executable later-validation boundaries. Phase 4
starts with the smallest dependency-respecting responsibility chain:

```text
Project and Source
→ Question
→ Decision
→ Context Item
→ Checkpoint
→ revision, supersession, contradiction, and forgetting
→ portable bundle and local binding
→ deterministic Recall basis
```

This handoff does not pre-create a crate taxonomy, API catalog, process layout,
storage schema, analyzer selection, provider implementation, or UI framework.

## Maintained references

Architecture and product authorities:

- `rebuild/docs/design/product-charter.md`
- `rebuild/docs/design/open-decisions.md`
- `rebuild/docs/design/acceptance-scenarios.md`
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

Validation authorities and evidence:

- `rebuild/docs/design/validation-plan.md`
- `rebuild/validation/README.md`
- `rebuild/validation/wave-1-summary.md`
- `rebuild/validation/repository-intelligence/polyglot-structural/report.md`
- `rebuild/validation/canonical-context/portability/report.md`
- `rebuild/validation/inquiry/frontier-resume/report.md`
- `rebuild/scripts/validate`
- `rebuild/scripts/check-architecture-contracts`
