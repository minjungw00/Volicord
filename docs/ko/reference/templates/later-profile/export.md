# EXPORT 템플릿

## 사용 시점

리뷰, 보관, 마이그레이션, Release Handoff를 위한 선택적 export/보고서 projection을 만들 때 `EXPORT`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 운영/export 보고서입니다. Export와 handoff bundle은 later operational/profile output이며 Core state 또는 artifact를 대체하지 않습니다.

## 기준 기록

- 포함된 Task와 gate 기록, 안전한 state/event version range facts
- Change Unit
- Run
- approval (later Approval profile only)
- Evidence Manifest
- Eval 기록
- 수동 QA 기록
- reconcile item
- report projection snapshot과 읽기용 보기 최신성(projection freshness)
- artifact 참조, owner relation, redaction status, retention/availability, integrity metadata
- redaction, omission, blocked-artifact summary
- omitted-secret note와 retained/expired artifact summary
- review 또는 Release Handoff display에 포함될 때 Write Authorization, User Judgment, Approval, Evidence Manifest, Eval, 수동 QA, acceptance context, Residual Risk, Artifact refs, redaction state, projection freshness를 보여주는 compact authority refs
- export profile boundary와 non-deployment/non-merge reminder 표시

## 렌더링 섹션

- 범위
- 상태 snapshot
- 보고서 projection snapshot
- Artifact 참조
- Redaction status 요약
- 생략되거나 차단된 content
- 무결성
- Release Handoff(릴리스 인계)

## 전체 템플릿

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

# EXPORT-0001 Harness export

> Projection 보기: `source_state_version`와 `updated_at` 기준으로 렌더링된 report snapshot입니다. Release Handoff/export 권한 경계는 [Operations And Conformance](../../operations-and-conformance.md#release-handoff-export-profile)가 담당합니다.

## 범위
- project_id:
- task_ids:
- 포함된 state version range:
- 포함된 event version range:
- policy 또는 profile 때문에 생략된 것:
- 만든 사람:
- 만든 시각:

## 상태 snapshot
- tasks:
- task gates:
- change units:
- runs:
- approvals (later Approval profile only; 그 외에는 none):
- evidence manifests:
- Eval records:
- 수동 QA records:
- reconcile items:
- state/event snapshot notes:

## 보고서 projection snapshot
- TASK:
- APR (later Approval profile only):
- RUN-SUMMARY:
- EVIDENCE-MANIFEST:
- EVAL:
- DIRECT-RESULT:
- optional design projections:

## Artifact 참조
| Artifact ID | Kind | Owner Record | URI | SHA256 | Size | Redaction Status | Retention / Availability | Export Treatment | Omission/Block Note |
|---|---|---|---|---|---|---|---|---|---|

## Redaction status 요약
- 생략된 secrets:
- 생략된 PII:
- artifact ref별 redaction status:
- redacted artifacts:
- blocked artifacts:
- 보존된 omission notes:
- 포함된 retained raw files:
- expired 또는 unavailable artifact refs:
- policy, expiry, unavailability, omission, block 때문에 제외된 raw files:

## 생략되거나 차단된 content
| Artifact ID | Affected Owner Or Display | Redaction Status | 이후 영향 | Note |
|---|---|---|---|---|

## 무결성
- export hash:
- manifest hash:
- 생성 시각:

## Release Handoff(릴리스 인계)
- close readiness:
- close blockers:
- authority refs: write={write_authorization_refs|none}; judgment={user_judgment_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_refs|none}; eval={eval_refs|none}; manual_qa={manual_qa_refs|none}; acceptance={acceptance_context_refs|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_status_summary}; freshness={projection_freshness}
- approval refs는 minimum MVP-1에서 `none`입니다. 민감 동작 coverage는 later Approval owner profile이 active가 아닌 한 `judgment_type=sensitive_action_approval`인 `user_judgment_refs`로 나타납니다.
- evidence refs:
- verification refs:
- 수동 QA refs:
- residual-risk refs:
- close/assurance display distinctions: self_checked={self_check_refs|none}; detached_verified={eval_refs|none}; verification_waived={verification_waiver_refs|none}; qa_waived={qa_waiver_refs|none}; risk_accepted_close={accepted_residual_risk_refs|none}
- changed files:
- projection freshness:
- artifact retention/availability:
- redaction/omission/block notes:
- suggested PR checklist:
- suggested deploy checklist:
- suggested rollback 또는 monitoring notes:
- 외부 권한 reminder: Deployment, merge, Approval, production monitoring, QA 또는 검증 면제, gate satisfaction, 작업 수락, 잔여 위험 수용, assurance upgrade, Task 닫기는 이 보고서 밖에 남는다.
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. `EXPORT`는 `ProjectionKind`일 뿐이며, export snapshot과 component는 owner 기록 또는 projection ref에 연결된 artifact로 남습니다.

`EXPORT`의 Release Handoff display는 self-checked work, `detached_verified`, 검증 면제, QA waiver, risk-accepted close를 ref 또는 명시적인 absence와 함께 분리해서 보여줘야 합니다. Export는 이런 표시를 보존할 수 있지만 Approval을 부여하거나, gate를 충족하거나, 작업 수락을 기록하거나, 잔여 위험 수용을 기록하거나, QA 또는 검증을 waive하거나, assurance를 높이거나, Task를 닫지 않습니다.

`EXPORT`는 기본적으로 원본 secret, PII, 민감 log, network trace, screenshot, 기타 민감 artifact 본문을 포함하면 안 됩니다. 크거나 민감한 artifact는 `ArtifactRef`로 나열합니다. 원본 file은 policy와 retention이 허용할 때만 포함하고, `secret_omitted` 또는 `blocked` entry는 ref와 note로만 표현합니다.

Export profile이 report projection snapshot, raw artifact, state snapshot을 생략한다면 bundle이 완전한 것처럼 암시하지 말고 무엇이 빠졌는지와 review 또는 Release Handoff에 미치는 영향을 보여줍니다. Retained artifact는 owner relation, integrity, redaction status, retention policy, export profile이 raw 포함을 허용할 때만 복사할 수 있습니다. Expired, unavailable, `secret_omitted`, `blocked` artifact는 ref, safe metadata, omission/block note로만 남습니다. Export는 projection, Markdown report, chat text, staging path에서 raw bytes를 다시 만들면 안 됩니다.

`secret_omitted`에서는 export가 안전한 omission note 또는 handle, 안전하게 저장된 bytes에 대한 hash를 포함할 수 있지만 생략된 값을 포함하면 안 됩니다. `blocked`에서는 export가 커밋된 metadata-only notice artifact와 그 hash, size, content type을 포함할 수 있습니다. 이 field들은 금지된 원본 payload가 아니라 notice bytes를 설명합니다. Release Handoff section은 export 전에 documented replacement, waiver, user judgment outcome, 받아들인 위험, fallback으로 해소되지 않은 omission 또는 block impact를 unavailable, insufficient, unresolved input 중 적절한 상태로 표시해야 합니다.

Recovery artifact가 export에 나타나면 recovery observation으로 label합니다. 별도의 owner record가 이미 그 path를 해결한 경우가 아니면, recovery artifact는 successful completion의 증거가 아니며 evidence, verification, QA, 작업 수락, close proof로 계산하면 안 됩니다.
