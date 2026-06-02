# EXPORT Template

## Used when

Use `EXPORT` when an optional export or report projection is generated for review, archival, migration, or Release Handoff use.

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: Operations/export reports. Export and handoff bundles are later operational/profile outputs and never replace Core state or artifacts.

## Source records

- included Task and gate records, with safe state/event version range facts
- Change Units
- Runs
- approvals
- Evidence Manifests
- Eval records
- Manual QA records
- reconcile items
- report projection snapshots and projection freshness
- artifact refs, owner relations, redaction status, retention/availability, and integrity metadata
- redaction, omission, and blocked-artifact summaries
- omitted-secret notes and retained/expired artifact summaries
- compact authority refs for Write Authorization, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, work acceptance context, Residual Risk, Artifact refs, redaction state, and projection freshness when included in review or Release Handoff display
- export profile boundary and non-deployment/non-merge reminder display

## Rendered sections

- Scope
- State Snapshots
- Report Projection Snapshots
- Artifact Refs
- Redaction Status Summary
- Omitted Or Blocked Content
- Integrity
- Release Handoff

## Full template

````md
---
doc_type: export_manifest
export_id: EXPORT-0001
project_id: PRJ-0001
profile: standard | release_handoff
export_bundle_status: current
source_state_version: 50
updated_at: 2026-05-06T10:30:00+09:00
---

# EXPORT-0001 Harness Export

> Projection view: rendered from `source_state_version` at `updated_at`; this export is a report snapshot. The Release Handoff/export authority boundary is owned by [Operations And Conformance](../operations-and-conformance.md#release-handoff-export-profile).

## Scope
- project_id:
- task_ids:
- included state version range:
- included event version range:
- omitted by policy or profile:
- created by:
- created at:

## State Snapshots
- tasks:
- task gates:
- change units:
- runs:
- approvals:
- evidence manifests:
- Eval records:
- Manual QA records:
- reconcile items:
- state/event snapshot notes:

## Report Projection Snapshots
- TASK:
- APR:
- RUN-SUMMARY:
- EVIDENCE-MANIFEST:
- EVAL:
- DIRECT-RESULT:
- optional design projections:

## Artifact Refs
| Artifact ID | Kind | Owner Record | URI | SHA256 | Size | Redaction Status | Retention / Availability | Export Treatment | Omission/Block Note |
|---|---|---|---|---|---|---|---|---|---|

## Redaction Status Summary
- secrets omitted:
- PII omitted:
- redaction status by artifact ref:
- redacted artifacts:
- blocked artifacts:
- omission notes preserved:
- retained raw files included:
- expired or unavailable artifact refs:
- raw files excluded by policy, expiry, unavailability, omission, or block:

## Omitted Or Blocked Content
| Artifact ID | Affected Owner Or Display | Redaction Status | Downstream Effect | Note |
|---|---|---|---|---|

## Integrity
- export hash:
- manifest hash:
- generated at:

## Release Handoff
- close readiness:
- close blockers:
- authority refs: write={write_authorization_refs|none}; decision={decision_packet_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_refs|none}; eval={eval_refs|none}; manual_qa={manual_qa_refs|none}; acceptance={acceptance_context_refs|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_status_summary}; freshness={projection_freshness}
- evidence refs:
- verification refs:
- Manual QA refs:
- residual-risk refs:
- close/assurance display distinctions: self_checked={self_check_refs|none}; detached_verified={eval_refs|none}; verification_waived={verification_waiver_refs|none}; qa_waived={qa_waiver_refs|none}; risk_accepted_close={accepted_residual_risk_refs|none}
- changed files:
- projection freshness:
- artifact retention/availability:
- redaction/omission/block notes:
- suggested PR checklist:
- suggested deploy checklist:
- suggested rollback or monitoring notes:
- external authority reminder: Deployment, merge, Approval, production monitoring, QA or verification waiver, gate satisfaction, work acceptance, residual-risk acceptance, assurance upgrade, and Task close remain outside this report.
````

## Notes

This template is a rendered shape, not canonical state. `EXPORT` is a `ProjectionKind` only; export snapshots and components remain artifacts linked to owner records or projection refs.

Release Handoff display in `EXPORT` should keep self-checked work, `detached_verified`, verification waiver, QA waiver, and risk-accepted close separate, with refs or explicit absence. The export may preserve those displays, but it does not grant Approval, satisfy gates, accept results, accept residual risk, waive QA or verification, upgrade assurance, or close the Task.

`EXPORT` must not embed raw secrets, PII, sensitive logs, network traces, screenshots, or other sensitive artifact bodies by default. Large or sensitive artifacts are listed by `ArtifactRef`; raw files are included only when policy and retention allow them, and `secret_omitted` or `blocked` entries stay represented by refs and notes.

If the export profile omits a report projection snapshot, raw artifact, or state snapshot, show the omission and its review or Release Handoff impact rather than implying the bundle is complete. Retained artifacts may be copied only when their owner relation, integrity, redaction status, retention policy, and export profile allow raw inclusion. Expired, unavailable, `secret_omitted`, or `blocked` artifacts stay represented by refs, safe metadata, and omission/block notes; export must not recreate their raw bytes from projections, Markdown reports, chat text, or staging paths.

For `secret_omitted`, export may include safe omission notes or handles and hashes over safe stored bytes, but not omitted values. For `blocked`, export may include the committed metadata-only notice artifact and its hash, size, and content type; those fields describe the notice bytes, never the forbidden raw payload. Release Handoff sections must show the same omission or block impact as unavailable, insufficient, or unresolved input unless a documented replacement, waiver, Decision Packet outcome, accepted risk, or fallback resolved it before export.

If recovery artifacts appear in an export, label them as recovery observations. They do not prove successful completion and must not be counted as evidence, verification, QA, work acceptance, or close proof unless a separate owner record already resolved that path.
