# EXPORT Template

## 사용 시점

리뷰, 보관, 마이그레이션, Release Handoff를 위한 선택적 export/보고서 projection을 만들 때 `EXPORT`를 사용합니다.

## 기준 기록

- 포함된 Task와 gate 기록
- Change Unit
- Run
- approval
- Evidence Manifest
- Eval 기록
- Manual QA 기록
- reconcile item
- projection snapshot과 projection 최신성
- artifact 참조, redaction state, retention, integrity metadata
- redaction, omission, blocked-artifact summary

## 렌더링 섹션

- Scope
- State Snapshots
- Projection Snapshots
- Artifact Refs
- Redaction Summary
- Omitted Or Blocked Content
- Integrity
- Release Handoff

## 전체 템플릿

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

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링된 report snapshot입니다. Deploy, merge, gate 충족, risk 수용, projection 최신성 변경, Task 닫기를 수행하지 않습니다.

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
| Artifact ID | Affected Owner Or Display | Redaction State | 이후 영향 | Note |
|---|---|---|---|---|

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
- suggested rollback 또는 monitoring notes:
- 외부 권한 reminder: Deployment, merge, Approval, production monitoring, QA 또는 verification waiver, gate satisfaction, final acceptance, residual-risk acceptance, assurance upgrade, Task 닫기는 이 보고서 밖에 남는다.
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. `EXPORT`는 `ProjectionKind`일 뿐이며, export snapshot과 component는 owner 기록 또는 projection ref에 연결된 artifact로 남습니다.

`EXPORT`는 기본적으로 원본 secret, PII, 민감 log, network trace, screenshot, 기타 민감 artifact 본문을 포함하면 안 됩니다. 크거나 민감한 artifact는 `ArtifactRef`로 나열합니다. 원본 file은 policy와 retention이 허용할 때만 포함하고, `secret_omitted` 또는 `blocked` entry는 ref와 note로만 표현합니다.

`secret_omitted`에서는 export가 안전한 omission note 또는 handle, 안전하게 저장된 bytes에 대한 hash를 포함할 수 있지만 생략된 값을 포함하면 안 됩니다. `blocked`에서는 export가 커밋된 metadata-only notice artifact와 그 hash, size, content type을 포함할 수 있습니다. 이 field들은 금지된 원본 payload가 아니라 notice bytes를 설명합니다. Release Handoff section은 export 전에 documented replacement, waiver, Decision Packet outcome, accepted risk, fallback으로 해소되지 않은 omission 또는 block impact를 unavailable, insufficient, unresolved input 중 적절한 상태로 표시해야 합니다.
