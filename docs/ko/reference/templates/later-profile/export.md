# EXPORT 템플릿

## 사용 시점

리뷰, 보관, 마이그레이션, 릴리스 인계(Release Handoff)를 위한 선택적 내보내기/보고서 상태 보기를 만들 때 `EXPORT`를 사용합니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 운영/내보내기 보고서입니다. Export와 handoff bundle은 나중 운영/프로필 산출물이며 Core 상태 또는 artifact를 대체하지 않습니다.

## 기준 기록

- 포함된 Task와 gate 기록, 안전한 상태/이벤트 version range 사실
- Change Unit
- Run
- 민감 동작 승인 기록(approval; later Approval 프로필이 활성화된 경우에만)
- 근거 목록(Evidence Manifest)
- Eval(분리 검증 결과) 기록
- 수동 QA 기록
- reconcile item
- 보고서 상태 보기 스냅샷과 읽기용 보기 최신성(projection freshness)
- artifact 참조, owner 관계, 가림 상태, 보존/사용 가능성, 무결성 metadata
- 가림, 생략, 차단된 아티팩트 요약
- 생략된 비밀 정보 메모와 보존/만료 artifact 요약
- review 또는 Release Handoff 표시에 포함될 때 쓰기 허가 기록(Write Authorization), 사용자 판단(User Judgment), 민감 동작 승인(Approval), 근거 목록(Evidence Manifest), Eval(분리 검증 결과), 수동 QA, 작업 수락 맥락, 잔여 위험(Residual Risk), 아티팩트 참조, 가림 상태, 읽기용 보기 최신성(projection freshness)을 보여주는 간결한 권한 참조
- 내보내기 프로필 경계와 배포/merge가 아님을 알리는 reminder 표시

## 렌더링 섹션

- 범위
- 상태 스냅샷
- 보고서 상태 보기 스냅샷
- 아티팩트 참조
- 가림 상태 요약
- 생략되거나 차단된 내용
- 무결성
- 릴리스 인계(Release Handoff)

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

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링된 보고서 스냅샷입니다. Release Handoff/export 권한 경계는 [Operations And Conformance](../../operations-and-conformance.md#release-handoff-export-profile)가 담당합니다.

## 범위
- project_id:
- task_ids:
- 포함된 state version range:
- 포함된 event version range:
- 정책 또는 프로필 때문에 생략된 것:
- 만든 사람:
- 만든 시각:

## 상태 스냅샷
- Task:
- Task gate:
- Change Unit:
- Run:
- 민감 동작 승인 참조(approvals; later Approval 프로필이 활성화된 경우에만; 그 외에는 none):
- 근거 목록(evidence manifests):
- Eval(분리 검증 결과) 기록:
- 수동 QA 기록:
- reconcile 항목:
- state/event 스냅샷 메모:

## 보고서 상태 보기 스냅샷
- TASK:
- APR(민감 동작 승인; later Approval 프로필이 활성화된 경우에만):
- RUN-SUMMARY:
- EVIDENCE-MANIFEST:
- EVAL:
- DIRECT-RESULT:
- 선택적 설계 상태 보기:

## 아티팩트 참조
| 아티팩트 ID | 종류 | 소유 기록 | URI | SHA256 | 크기 | 가림 상태 | 보존 / 사용 가능성 | Export 처리 | 생략/차단 메모 |
|---|---|---|---|---|---|---|---|---|---|

## 가림 상태 요약
- 생략된 비밀 정보:
- 생략된 PII:
- artifact ref별 가림 상태:
- 가림 처리된 아티팩트:
- 차단된 아티팩트:
- 보존된 omission notes:
- 포함된 보존 원본 파일:
- 만료되었거나 사용할 수 없는 artifact refs:
- 정책, 만료, 사용 불가, 생략, 차단 때문에 제외된 원본 파일:

## 생략되거나 차단된 내용
| 아티팩트 ID | 영향받는 owner 또는 표시 | 가림 상태 | 이후 영향 | 메모 |
|---|---|---|---|---|

## 무결성
- export hash:
- manifest hash:
- 생성 시각:

## 릴리스 인계(Release Handoff)
- 닫기 준비 상태:
- 닫기 막힘:
- 권한 참조: write={write_authorization_refs|none}; judgment={user_judgment_refs|none}; approval={approval_refs|none}; evidence={evidence_manifest_refs|none}; eval={eval_refs|none}; manual_qa={manual_qa_refs|none}; acceptance={acceptance_context_refs|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_status_summary}; freshness={projection_freshness}
- 민감 동작 승인 참조(`approval_refs`)는 minimum MVP-1에서 `none`입니다. 민감 동작 뒷받침 범위는 later Approval 담당 프로필이 활성화되지 않은 한 `judgment_type=sensitive_action_approval`인 `user_judgment_refs`로 나타납니다.
- 근거 참조(evidence refs):
- 검증 참조(verification refs):
- 수동 QA 참조:
- 잔여 위험 참조:
- 닫기/보증 표시 구분: self_checked={self_check_refs|none}; detached_verified={eval_refs|none}; verification_waived={verification_waiver_refs|none}; qa_waived={qa_waiver_refs|none}; risk_accepted_close={accepted_residual_risk_refs|none}
- 변경된 파일:
- 보기 최신성:
- artifact 보존/사용 가능성:
- 가림/생략/차단 메모:
- 제안 PR 점검표:
- 제안 배포 점검표:
- 제안 rollback 또는 monitoring notes:
- 외부 권한 reminder: Deployment, merge, 민감 동작 승인(Approval), production monitoring, QA 또는 검증 면제, gate satisfaction, 작업 수락, 잔여 위험 수용, assurance upgrade, Task 닫기는 이 보고서 밖에 남는다.
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. `EXPORT`는 `ProjectionKind`일 뿐이며, export 스냅샷과 구성 요소는 owner 기록 또는 projection ref에 연결된 artifact로 남습니다.

`EXPORT`의 릴리스 인계(Release Handoff) 표시는 자체 확인된 작업, `detached_verified`, 검증 면제, QA waiver, risk-accepted close를 참조 또는 명시적인 부재와 함께 분리해서 보여줘야 합니다. Export는 이런 표시를 보존할 수 있지만 민감 동작 승인(Approval)을 부여하거나, gate를 충족하거나, 작업 수락을 기록하거나, 잔여 위험 수용을 기록하거나, QA 또는 검증을 waive하거나, assurance를 높이거나, Task를 닫지 않습니다.

`EXPORT`는 기본적으로 원본 비밀 정보, PII, 민감 로그, network trace, screenshot, 기타 민감 artifact 본문을 포함하면 안 됩니다. 크거나 민감한 artifact는 `ArtifactRef`로 나열합니다. 원본 file은 정책과 retention이 허용할 때만 포함하고, `secret_omitted` 또는 `blocked` entry는 ref와 note로만 표현합니다.

Export 프로필이 보고서 상태 보기 스냅샷, 원본 artifact, 상태 스냅샷을 생략한다면 bundle이 완전한 것처럼 암시하지 말고 무엇이 빠졌는지와 review 또는 릴리스 인계(Release Handoff)에 미치는 영향을 보여줍니다. 보존된 artifact는 owner 관계, integrity, 가림 상태, retention policy, export 프로필이 원본 포함을 허용할 때만 복사할 수 있습니다. Expired, unavailable, `secret_omitted`, `blocked` artifact는 ref, safe metadata, omission/block note로만 남습니다. Export는 projection, Markdown report, chat text, staging path에서 raw bytes를 다시 만들면 안 됩니다.

`secret_omitted`에서는 export가 안전한 omission note 또는 handle, 안전하게 저장된 bytes에 대한 hash를 포함할 수 있지만 생략된 값을 포함하면 안 됩니다. `blocked`에서는 export가 커밋된 metadata-only notice artifact와 그 hash, size, content type을 포함할 수 있습니다. 이 field들은 금지된 원본 payload가 아니라 notice bytes를 설명합니다. 릴리스 인계(Release Handoff) section은 export 전에 documented replacement, waiver, user judgment outcome, 받아들인 위험, fallback으로 해소되지 않은 omission 또는 block impact를 unavailable, insufficient, unresolved input 중 적절한 상태로 표시해야 합니다.

복구 artifact가 export에 나타나면 recovery observation으로 label합니다. 별도의 owner record가 이미 그 path를 해결한 경우가 아니면, recovery artifact는 successful completion의 증거가 아니며 evidence, verification, QA, 작업 수락, close proof로 계산하면 안 됩니다.
