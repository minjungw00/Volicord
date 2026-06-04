# Later: Operations Profile

Use this page to route later operations, recovery, export, and handoff material without making it part of the MVP implementation path.

This is planning and navigation documentation for future Harness behavior. It is not an MVP-1 requirement and it is not implemented runtime behavior. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, or product code in this repository.

## Read This When

- You are checking what belongs after the Assurance Profile.
- You need operator, diagnostics, recovery, export, artifact-integrity, projection-refresh, or handoff owner links.
- You need to keep future operations work separate from Engineering Checkpoint and MVP-1 User Work Loop.

## Bucket Boundary

Operations Profile is later than MVP-1 and Assurance Profile. It covers local operator and handoff hardening after the first user-value loop is proven. It does not make operations surfaces early MVP requirements.

| Operations bucket | Belongs here | Still out of this profile unless promoted |
|---|---|---|
| Export | Task export bundles, artifact integrity manifests, redaction/omission notes, retained or unavailable artifact reporting, and export non-leakage checks. | Hosted sharing, import/sync workflows, deployment authority, and broad release automation remain Roadmap candidates. |
| Recovery | Interrupted operation classification, compensating events, lock recovery, projection failure handling, artifact repair/replacement routing, and manual recovery escalation. | Automated rollback, production recovery, remote fleet repair, and external system recovery remain Roadmap candidates. |
| Handoff | Release Handoff report/export profile, close-relevant summary, evidence/verification/QA/risk refs, and external checklist guidance without deployment authority. | Merge, deploy, canary, rollback, and production monitoring automation remain Roadmap candidates. |
| Operator readiness | Local project/runtime registration health, MCP availability, surface capability posture, artifact-store health, and safe next operator action. | Hosted operator console, team permissions, remote/shared operations, and connector marketplaces remain Roadmap candidates. |
| Doctor/readiness surfaces | Full `doctor` category set, readiness levels, security posture diagnostics, projection freshness checks, reconcile visibility, and docs-maintenance report exposure. | Dashboards, analytics, long-term metrics, and automation that treats diagnostics as authority remain Roadmap candidates. |

Projection refresh, reconcile, artifact checks, and conformance run entrypoints belong to Operations Profile when the owner Reference docs define the behavior. They remain derived, diagnostic, or repair surfaces; they do not create a second state model.

## Main Path

Start with the stage boundary in [MVP-1 User Work Loop](../build/mvp-user-work-loop.md), then use only the owner needed for the operations question:

| Need | Owner |
|---|---|
| Operator commands, diagnostics, recover, reconcile, export, artifact checks, and conformance run entrypoints | [Operations And Conformance Reference](../reference/operations-and-conformance.md) |
| Runtime layout, artifact storage, locks, migrations, projection jobs, and validator storage | [Storage Reference](../reference/storage.md) |
| Security posture, trust boundaries, threat categories, controls, and guarantee wording | [Security Reference](../reference/security.md) |
| Runtime spaces, Core placement, transaction order, projection/reconcile placement, and recovery overview | [Runtime Architecture Reference](../reference/runtime-architecture.md) |
| Projection freshness and rendered output boundaries | [Projection And Templates Reference](../reference/projection-and-templates.md) and [Template Reference](../reference/templates/README.md) |
| Operations fixture mechanics and future operations scenarios | [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and [Future Fixtures](future-fixtures.md) |

## Boundary

Operations Profile is where export, recovery, handoff, operator readiness, and doctor/readiness surfaces are organized. It is not a dashboard, hosted workflow, team workflow, connector marketplace, deployment automation, or broad orchestration profile.

It does not make Runtime Home tamper-proof, make projections authoritative, create a hosted dashboard, or provide OS-level sandboxing, arbitrary-tool permission control, preventive blocking, or isolation unless a promoted owner path proves that exact mechanism.

Listing an item here does not make it an MVP-1 requirement, an implemented runtime behavior, or executable conformance. Future operations fixture rows stay in [Future Fixtures](future-fixtures.md) until an owner promotes the exact behavior and materializes exact-shape fixtures.
