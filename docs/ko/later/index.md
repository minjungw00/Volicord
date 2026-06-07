# Later 후보 색인

이 문서는 later 후보와 승격 경계를 다루는 단일 활성 색인입니다.

아래 행은 계획 후보일 뿐입니다. 현재 MVP 요구사항, 활성 API 또는 스키마 계약, fixture 전체 본문, 템플릿 전체 본문, 런타임 동작, 구현 작업, 생성 산출물, 수락 증거, 런타임 작업 시작 허가가 아닙니다. 원칙은 간단합니다: 후보는 승격 전까지 동작하지 않는다.

## 1. 경계

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Later 후보 색인 | 문서 전용 | 세부 계약이 돌아오려면 향후 담당 문서가 좁은 후보를 먼저 승격해야 합니다. | 문서에만 영향 |
| 현재 저장소 단계 | 문서 전용 계획 | 런타임 작업 전 `docs/*/build/mvp-plan.md`의 문서 수락과 별도 구현 준비 결정이 필요합니다. | 없음 |
| 후보 권한 | 이름만 있는 후보 | 승격된 담당 문서가 담당 문서 지정과 API, 스키마, 저장소, 보안, 적합성, 증거에 미치는 정확한 효과를 정해야 합니다. | 승격 전까지 없음 |
| 한영 문서 동시 유지 | 대응 활성 문서 | 의미가 바뀌면 영어와 한국어를 같은 작업 묶음에서 함께 고칩니다. | 문서에만 영향 |

## 2. 승격 규칙

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| 담당 문서 지정 | 승격 전 필요 | 이름 있는 담당 문서, 좁은 범위, 비목표, 대체 동작이 필요합니다. | 없음 |
| 계약 위치 | 색인 경계만 있음 | 정확한 API, 스키마, 저장소, Projection, 템플릿, fixture, 운영자 명령 계약은 알맞은 활성 담당 문서에 둡니다. | 승격 전까지 없음 |
| 보안 표현 | 이 문서에는 활성 보장 주장 없음 | 증명된 메커니즘에 맞는 협력형, 탐지형, 예방형, 격리형 표현이 필요합니다. | 승격 전까지 없음 |
| 향후 증명 경로 기대치 | 후보 목록은 현재 런타임 증명이 아님 | 승격된 동작에 대한 적합성 목표, fixture, 증거 기대치, 또는 담당 문서가 정한 다른 증명 경로가 필요합니다. | 승격 전까지 없음 |
| 활성 범위 상속 | 기본적으로 비활성 | 향후 담당 문서가 승격이 현재 MVP나 더 이른 smoke 목표에 근거 없는 요구사항을 더하지 않는다는 점을 증명해야 합니다. | 현재 MVP에 영향을 주면 안 됨 |
| 대체 불가 경계 | 필요한 경계 | Core 상태, 사용자 판단, 증거, 검증, Manual QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태는 계속 분리합니다. | 없음 |

## 3. 보증 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| assurance hardening | later 후보 | 담당 문서가 관문, 대체 동작, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Evidence Manifest | later 후보 | 증거 담당 문서가 아티팩트 참조, 가림, 닫기 영향, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Manual QA | later 후보 | Manual QA 담당 문서가 면제 경로, 아티팩트 참조, 발견 사항, QA 관문 영향을 정해야 합니다. | 승격 전까지 없음 |
| Eval / detached verification | later 후보 | Eval 담당 문서가 독립성 의미, 기준선 최신성, 아티팩트 무결성, 보증 갱신 규칙을 정해야 합니다. | 승격 전까지 없음 |
| Decision Packet full-format presentation | later 후보 | 사용자 판단 담당 문서가 `presentation=full`을 켜되 기본 현재 MVP 경로로 만들지 않아야 합니다. | 승격 전까지 없음 |
| Risk review and residual-risk visibility | later 후보 | Core와 사용자 판단 담당 문서가 위험 표시, 수락, 만료, 닫기 영향을 정해야 합니다. | 승격 전까지 없음 |
| Full design-quality policy families: full `shared_design` policy, `domain_language`, `vertical_slice`, `feedback_loop`, `tdd_trace`, `deep_module_interface`, `codebase_stewardship`, detailed `manual_qa`, `two_stage_review_display`, detached-verification policy, steward policies | 이름만 있음 | 설계 품질 담당 문서가 정확한 범위, 검증기 경계, 면제/증거 규칙, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |

<a id="operations-candidates"></a>
## 4. 운영 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| operations hardening | later 후보 | 운영 담당 문서가 명령, 진단, 대체 동작, 보안 표현, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 향후 로컬 운영자 명령 묶음: `harness connect`, `harness serve mcp`, `harness doctor`, `harness projection refresh`, `harness reconcile`, `harness recover`, `harness export`, `harness artifacts check`, `harness conformance run` | 명령 이름만 있음 | 운영 담당 문서가 정확한 구문, 보안 태세, API와 저장소 영향, 보고, 대체 동작, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Export | later 후보 | Export 담당 문서가 저장소/아티팩트 처리, 가림, 생략, 무결성, 향후 유출 방지 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Release Handoff | later 후보 | Handoff 담당 문서가 배포, 병합, 롤백, 운영 환경 권한을 별도 승격 전까지 외부에 남겨야 합니다. | 승격 전까지 없음 |
| Recovery and reconcile | later 후보 | Operations, Storage, Projection, Reconcile, Security 담당 문서 규칙이 필요합니다. | 승격 전까지 없음 |
| Operator readiness and `doctor` surfaces | later 후보 | 운영 담당 문서가 진단, 기능 확인, 보안 태세, 지원되지 않는 접점의 대체 동작을 정해야 합니다. | 승격 전까지 없음 |
| Projection refresh and freshness diagnostics | later 후보 | Projection 담당 문서가 Projection이 비권위 상태 보기로 남는 동작을 정해야 합니다. | 승격 전까지 없음 |
| Future conformance run entrypoint | 런타임 fixture가 생긴 뒤의 later 후보 | 실행기, 스위트, 검증 주장, API, 저장소, 이벤트, 아티팩트, 오류, 보고 계약이 필요합니다. | 승격 전까지 없음 |

## 5. Later API 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| `harness.next` | 메서드 이름만 있음 | 별도 다음 행동 페이로드를 위한 담당 문서 활성화가 필요합니다. MVP는 계속 `harness.status.next_actions`를 씁니다. | 승격 전까지 없음 |
| `harness.launch_verify` | 메서드 이름만 있음 | Eval/검증 담당 문서가 기능 처리, 기준선 최신성, 정직한 격리 표현을 정해야 합니다. | 승격 전까지 없음 |
| `harness.record_eval` | 메서드 이름만 있음 | Eval 담당 문서가 독립성 검증, 아티팩트 참조, 관문/보증 갱신을 정해야 합니다. | 승격 전까지 없음 |
| `harness.record_manual_qa` | 메서드 이름만 있음 | Manual QA 담당 문서가 면제 경로, 아티팩트, 발견 사항, 관문 영향을 정해야 합니다. | 승격 전까지 없음 |
| Later read-only resources: policy, evidence-manifest, surface, report, bundle, journey, design | 리소스 이름만 있음 | 각 리소스 담당 문서가 읽기 전용 계약과 변경 부작용 없음을 정해야 합니다. | 승격 전까지 없음 |
| Later `harness.record_run` branches: verification input, feedback-loop updates, TDD trace updates | 분기 이름만 있음 | `record_run` 담당 문서 활성화와 단일 분기 페이로드 규칙이 필요합니다. | 승격 전까지 없음 |
| Later user-judgment branches: waiver, reconcile, residual-risk, richer acceptance visibility | 분기 이름만 있음 | 사용자 판단 담당 문서 활성화와 대체 불가 규칙이 필요합니다. | 승격 전까지 없음 |

<a id="later-schema-candidates"></a>
## 6. Later schema 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| later schema extensions | 스키마 이름만 있음 | 승격된 담당 문서가 정확한 필드와 검증기를 활성 계약에 정의해야 합니다. | 승격 전까지 없음 |
| Later close and assurance fields: `verifying`, `qa`, `completed_verified`, `detached_verified`, verification gate, QA gate, assurance blockers | 필드 이름만 있음 | Core/API 담당 문서 활성화와 닫기 대체 불가 규칙이 필요합니다. | 승격 전까지 없음 |
| Later next-action values: `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` | 값 이름만 있음 | 대응 API 또는 담당 문서 활성화가 필요합니다. | 승격 전까지 없음 |
| Recommended playbooks and judgment context | 메타데이터 이름만 있음 | Agent Integration/API 담당 문서가 메타데이터를 읽기 전용으로 두고 상태를 만족시키지 못하게 해야 합니다. | 승격 전까지 없음 |
| Later ref and artifact values: bundle, manifest, QA capture, export component, design, Eval, Manual QA, TDD, projection, related refs | 값 이름만 있음 | ArtifactRef, StateRecordRef, Storage, 관련 담당 문서 활성화가 필요합니다. | 승격 전까지 없음 |
| ValidatorResult later stable IDs: design, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, context-hygiene checks | ID 이름만 있음 | Validator 담당 문서가 stable ID, 심각도, 면제, 향후 fixture 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Waiver, reconcile, and residual-risk branches | 분기 이름만 있음 | 사용자 판단, Core, 닫기 담당 문서 규칙이 필요합니다. | 승격 전까지 없음 |

<a id="later-template-candidates"></a>
## 7. Later template 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`), `APR`, Approval Card, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, Verification Result Card, `MANUAL-QA`, Manual QA Card, `TASK`, `DIRECT-RESULT`, `JOURNEY-CARD`, `DESIGN`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `TDD-TRACE`, `EXPORT` | 템플릿 이름만 있음 | 템플릿 담당 문서 지정, 원천 기록, 대체 동작, 대체 불가 규칙, 최신성 동작, 향후 승격에 필요한 증명 경로 기대치가 필요합니다. | 승격 전까지 없음 |

<a id="future-fixture-families"></a>
## 8. 향후 fixture 계열

아래 긴 영어 항목은 향후 fixture 계열 이름만 보존한 것입니다. 현재 MVP 요구사항도 아니고 실행 가능한 적합성 묶음도 아닙니다.

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Intake and decision routing; Core, evidence, verification, and close; Artifact redaction and export non-leakage; Agency and user-judgment separation; Connector capability honesty; Design-quality and stewardship; Context hygiene and resume freshness; Projection, reconcile, and verification boundary; Operations diagnostics, export, recover, and handoff; Browser QA Capture | 향후 fixture 계열 이름만 있음 | 적합성 담당 문서 지정, 정확한 fixture 형태, 검증 주장, 페이로드, API, 저장소, 오류 영향, 향후 승격에 필요한 증명 경로 기대치가 필요합니다. | 승격 전까지 없음 |

## 9. 넓은 향후 후보

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| Dashboard, hosted workflows, artifact dashboard, richer cards, richer visualizations | later 후보 | 파생 표시 담당 문서가 읽기 전용, 비권위 동작을 정해야 합니다. | 승격 전까지 없음 |
| Browser capture automation | later 후보 | Capture 담당 문서가 가림/PII, 보존, 대체 동작, QA/수락 대체 불가 규칙을 정해야 합니다. | 승격 전까지 없음 |
| Cross-surface verification | later 후보 | Core/Eval 담당 문서가 반환 기록, 독립성, 지원되지 않는 접점의 대체 동작을 정해야 합니다. | 승격 전까지 없음 |
| Broader connectors, connector marketplace, hosted UI, hosted/remote runtime | later 후보 | Connector/API/보안 담당 문서와 향후 로컬 권위 경계 증명 기대치가 필요합니다. | 승격 전까지 없음 |
| Native hooks, preventive guard expansion, advanced sidecar watcher | later 후보 | 예방형, 격리, 임의 도구 제어 주장을 하기 전에 담당 문서가 증명한 대상 메커니즘이 필요합니다. | 승격 전까지 없음 |
| Context Index, local derived metrics, long-term metrics | later 후보 | 읽기 전용 검색/진단 담당 문서가 필요하며 권한이나 닫기 효과가 없어야 합니다. | 승격 전까지 없음 |
| Team workflows, permissions, shared capability sets, orchestration, parallel lanes | later 후보 | 범위, 권한, 허가 체계, 사용자 소유 판단 담당 문서가 필요합니다. | 승격 전까지 없음 |
| Advanced exports, release/deployment/canary/rollback/merge/production-monitoring automation | later 후보 | 별도 담당 범위가 필요합니다. 명시적으로 승격하기 전까지 배포와 운영 환경 권한은 외부에 남습니다. | 승격 전까지 없음 |
| Advanced validators and language or interface checks | later 후보 | Validator 담당 문서가 정확한 ID, 심각도, 면제, fixture 동작을 정해야 합니다. | 승격 전까지 없음 |

## 10. 명시적으로 제거한 상세 자료

| 후보 | 상태 | 승격 조건 | 현재 MVP 영향 |
|---|---|---|---|
| later template 전체 본문 | later template 전체 본문은 제거되었다. | 승격된 템플릿 담당 문서에서만 다시 정의한다. | 현재 MVP에 영향을 주면 안 됨 |
| fixture YAML draft 전체 본문 | fixture YAML draft 전체 본문은 제거되었다. | 승격된 적합성 담당 문서에서만 다시 정의한다. | 현재 MVP에 영향을 주면 안 됨 |
| later schema 전체 본문 | later schema 전체 본문은 제거되었다. | 승격된 스키마/API/저장소 담당 문서에서만 다시 정의한다. | 현재 MVP에 영향을 주면 안 됨 |
