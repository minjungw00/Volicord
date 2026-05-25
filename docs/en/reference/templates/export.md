# EXPORT Template

## Used when

Use `EXPORT` when an optional export or report projection is generated for review, archival, migration, or Release Handoff use.

## Source records

- included Task and gate records
- Change Units
- Runs
- approvals
- Evidence Manifests
- Eval records
- Manual QA records
- reconcile items
- projection snapshots and projection freshness
- artifact refs, redaction state, retention, and integrity metadata
- redaction, omission, and blocked-artifact summaries

## Rendered sections

- Scope
- State Snapshots
- Projection Snapshots
- Artifact Refs
- Redaction Summary
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
status: complete
source_state_version: 50
updated_at: 2026-05-06T10:30:00+09:00
---

# EXPORT-0001 Harness Export

## Scope
- project_id:
- task_ids:
- included state version range:
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

## Projection Snapshots
- TASK:
- APR:
- RUN-SUMMARY:
- EVIDENCE-MANIFEST:
- EVAL:
- DIRECT-RESULT:
- optional design projections:

## Artifact Refs
| Artifact ID | Kind | Owner Record | URI | SHA256 | Redaction State | Retention | Omission/Block Note |
|---|---|---|---|---|---|---|---|

## Redaction Summary
- secrets omitted:
- PII omitted:
- redacted artifacts:
- blocked artifacts:
- raw files excluded by policy:

## Omitted Or Blocked Content
| Artifact ID | Affected Owner Or Display | Redaction State | Note |
|---|---|---|---|

## Integrity
- export hash:
- manifest hash:
- generated at:

## Release Handoff
- close readiness:
- close blockers:
- evidence refs:
- verification refs:
- Manual QA refs:
- residual-risk refs:
- changed files:
- projection freshness:
- redaction/omission/block notes:
- suggested PR checklist:
- suggested deploy checklist:
- suggested rollback or monitoring notes:
- external authority reminder: Deployment, merge, approval, production monitoring, QA or verification waiver, gate satisfaction, final acceptance, residual-risk acceptance, assurance upgrade, and Task close remain outside this report.
````

## Notes

This template is a rendered shape, not canonical state. `EXPORT` is a `ProjectionKind` only; export snapshots and components remain artifacts linked to owner records or projection refs.

`EXPORT` must not embed raw secrets, PII, sensitive logs, network traces, screenshots, or other sensitive artifact bodies by default. Large or sensitive artifacts are listed by `ArtifactRef`; raw files are included only when policy and retention allow them, and `secret_omitted` or `blocked` entries stay represented by refs and notes.
