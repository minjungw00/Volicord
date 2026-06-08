# 이후 후보 색인

이 문서는 이후 후보와 승격 경계를 다루는 단일 현재 문서 담당 색인입니다. 현재 MVP에서 제거되었거나 제외된 값, API 이름, 스키마 이름, 관문, validator, 작업 흐름, 프로필, 적합성 생태계, 내보내기/인계 형식을 간결하게 분류합니다.

이후 후보에는 현재 활성 동작이 없습니다. 이후 후보는 현재 닫기 가능 여부에 영향을 주지 않으며, 담당 문서가 승격하기 전까지 활성 API 메서드, enum 값, 저장소 테이블, validator, 관문을 만들지 않습니다.

아래 행은 계획 후보일 뿐입니다. 현재 MVP 요구사항, 활성 API 또는 스키마 계약, fixture 전체 본문, 템플릿 전체 본문, 런타임 동작, 구현 작업, 생성 산출물, 수락 증거, 런타임 작업 시작 허가가 아닙니다. 후보는 담당 문서가 명시적으로 승격하기 전까지 동작하지 않습니다.

승격에는 `later/index.md`의 언급만이 아니라 담당 문서의 명시적 변경이 필요합니다. 그 담당 문서 변경 전까지 이 색인에 이름이 있다는 사실은 활성 동작, 활성 API 메서드, 스키마 필드나 enum 값, 저장소 테이블이나 기록, 관문, validator, 보고서, 템플릿, fixture, 커넥터 동작, 생성 산출물, 닫기 효과, 보장 주장, 구현 작업을 만들지 않습니다. 메서드, enum 값, 필드, validator, 관문, 템플릿, 명령처럼 보이는 이름도 승격된 담당 문서가 활성 담당 계약을 고치기 전까지는 동작하지 않습니다. 이것이 이 문서의 active/later 경계입니다.

## 1. 경계

대조 기준으로, 현재 MVP 경계는 [MVP 계획](../build/mvp-plan.md)에 닫힌 목록으로 정해져 있습니다. 그 목록에는 평소 말 입력과 Task 생성, `update_scope`, 사용자 판단 기록, 민감 동작 승인 기록, 경로 수준 `prepare_write`와 Write Authorization, `record_run`, `stage_artifact`를 통한 스테이징된 아티팩트 등록, `EvidenceSummary`, `close_task` 차단 사유 계산, 읽는 시점의 상태/Projection, 등록된 로컬 접점 접근, 협력형 보장, 관련 활성 역량 확인이 실제로 통과한 뒤의 탐지형 보장만 포함됩니다. 이 페이지의 어떤 항목도 그 목록을 넓히지 않습니다.

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 이후 후보 색인 | 문서 전용 | 세부 계약이 돌아오려면 향후 담당 문서가 좁은 후보를 먼저 승격해야 합니다. | 문서에만 영향 |
| 현재 저장소 단계 | 문서 전용 계획 | 런타임 작업 전 `docs/*/build/mvp-plan.md`의 문서 수락과 별도 구현 준비 결정이 필요합니다. | 없음 |
| 후보 권한 | 이름만 있는 후보 | 승격된 담당 문서가 담당 문서 지정과 API, 스키마, 저장소, 보안, 적합성, 증거에 미치는 정확한 효과를 정해야 합니다. | 승격 전까지 없음 |
| 현재 활성 동작 없음 | 필요한 경계 | 이후 후보는 현재 닫기 가능 여부에 영향을 주지 않으며, 담당 문서가 승격하기 전까지 활성 API 메서드, enum 값, 저장소 테이블, validator, 관문, 런타임 동작을 만들지 않습니다. | 없음 |
| 한영 문서 동시 유지 | 대응 활성 문서 | 의미가 바뀌면 영어와 한국어를 같은 작업 묶음에서 함께 고칩니다. | 문서에만 영향 |

## 2. 승격 규칙

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 담당 문서 지정 | 승격 전 필요 | 이름 있는 담당 문서, 좁은 범위, 비목표, 대체 동작이 필요합니다. | 없음 |
| 명시적 담당 문서 변경 | 승격 전 필요 | 이 색인의 언급은 승격이 아닙니다. 활성 동작으로 존재하려면 담당 문서가 활성 메서드, enum 값, 저장소 테이블, validator, 관문, 작업 흐름, 프로필, 형식 계약을 고쳐야 합니다. | 승격 전까지 없음 |
| 계약 위치 | 색인 경계만 있음 | 정확한 API, 스키마, 저장소, Projection, 템플릿, fixture, 운영자 명령 계약은 알맞은 활성 담당 문서에 둡니다. | 승격 전까지 없음 |
| 승격 전 활성 동작 없음 | 필요한 경계 | 승격된 담당 문서가 범위, 대체 동작, 증명 기대를 이름 붙이기 전에는 어떤 후보도 런타임 동작, API/스키마 값, 저장소, 닫기, 템플릿, fixture, 보고서, 커넥터 동작, 보장 표시에 영향을 주지 않습니다. API/스키마 승격은 이 색인에 기대지 말고 활성 Schema Core 담당 문서를 고쳐야 합니다. | 승격 전까지 없음 |
| 활성 값 집합 담당 | 활성 담당 문서 경계 | 현재 활성 메서드 이름과 스키마 enum 값 집합은 `docs/*/reference/api/schema-core.md`에 둡니다. 여기에 적힌 이후 이름은 그 값 집합을 넓히지 않습니다. | 없음 |
| 보안 표현 | 이 문서에는 활성 보장 주장 없음 | 증명된 메커니즘에 맞는 협력형, 탐지형, 예방형, 격리형 표현이 필요합니다. | 승격 전까지 없음 |
| 향후 증명 경로 기대치 | 후보 목록은 현재 런타임 증명이 아님 | 승격된 동작에 대한 적합성 목표, fixture, 증거 기대치, 또는 담당 문서가 정한 다른 증명 경로가 필요합니다. | 승격 전까지 없음 |
| 활성 범위 상속 | 기본적으로 비활성 | 향후 담당 문서가 승격이 현재 MVP나 더 이른 스모크 목표에 근거 없는 요구사항을 더하지 않는다는 점을 증명해야 합니다. | 현재 MVP에 영향을 주면 안 됨 |
| 대체 불가 경계 | 필요한 경계 | Core 상태, 사용자 판단, 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태는 계속 분리합니다. | 없음 |

## 3. 보증 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 보증 강화 | 이후 후보 | 담당 문서가 관문, 대체 동작, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Full Evidence Manifest | 이후 후보 | 증거 담당 문서가 아티팩트 참조, 가림, 닫기 영향, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 영속 아티팩트로서의 Discovery Brief, Question Queue, Assumption Register | 이후 구체화 후보 | Core/API/저장소 담당 문서가 정확한 범위, 지속 방식, 대체 불가 규칙, 닫기 영향, 증명 경로 기대치를 정해야 합니다. 현재 MVP 구체화는 Task, Change Unit, `user_judgment`, 증거 요약, 차단 사유, 다음 안전한 행동 안에 남습니다. | 승격 전까지 없음 |
| 수동 QA 작업 흐름과 `qa_gate` | 이후 후보 | 수동 QA 담당 문서가 작업 흐름 단계, 면제 경로, 아티팩트 참조, 발견 사항, 정확한 `qa_gate` 활성화, 수동 QA 관문 닫기 영향을 정해야 합니다. | 승격 전까지 없음 |
| `qa_waiver` 수동 QA 면제 판단 | 이후 사용자 판단 후보 | 수동 QA와 사용자 판단 담당 문서가 정확한 `qa_waiver` 활성화, 허용 범위, 대체 불가 규칙, 잔여 위험 표시, 닫기 영향을 정해야 합니다. | 승격 전까지 없음 |
| `verification_gate` 검증 관문 | 이후 후보 | Core/API/Eval 담당 문서가 정확한 `verification_gate` 필드, 필수 조건, 대체 동작, 증명 기대치, 닫기 영향을 정해야 합니다. | 승격 전까지 없음 |
| `verification_risk_acceptance` 검증 위험 수락 | 이후 사용자 판단 후보 | 검증과 사용자 판단 담당 문서가 정확한 `verification_risk_acceptance` 활성화, 허용되는 위험 범위, 대체 불가 규칙, 닫기 영향을 정해야 합니다. | 승격 전까지 없음 |
| Eval / 분리형 검증 / 평가 작업 흐름 | 이후 후보 | Eval 담당 문서가 독립성 의미, 기준선 최신성, 아티팩트 무결성, 작업 흐름 영향, 보증 갱신 규칙을 정해야 합니다. | 승격 전까지 없음 |
| Full Decision Packet 형식과 `presentation=full` | 이후 후보 | 사용자 판단 담당 문서가 `presentation=full`과 전체 Decision Packet 형식을 켜되 둘 중 어느 것도 기본 현재 MVP 경로로 만들지 않아야 합니다. | 승격 전까지 없음 |
| 상세 위험 검토와 잔여 위험 생명주기 | 이후 후보 | Core와 사용자 판단 담당 문서가 상세 위험 기록, 검토 흐름, 만료, 닫기 영향을 정해야 합니다. 간결한 잔여 위험 표시는 Core/API 담당 경로를 통해서만 active로 남습니다. | 승격 전까지 없음 |
| 설계 gate와 닫기 category 이름: `design_gate`, `design_policy` | 이름만 있음 | Core/API/설계 품질 담당 문서가 승격 전에 정확한 필드, 범주 값, 대체 동작, 닫기 대체 불가 규칙, 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 설계 정책 waiver | 이후 waiver 후보 | Core, 사용자 판단, QA/검증, 설계 품질 담당 문서가 허용 범위, 대체 불가 규칙, 잔여 위험 표시, 정확한 기록 동작을 정해야 합니다. | 승격 전까지 없음 |
| 넓은 설계 validator, 설계 정책 validator, 심각도 기반 차단 정책 | 이후 후보 | Validator와 설계 품질 담당 문서가 정확한 ID, 심각도 의미, 닫기 영향, 대체 동작, waiver 경계, fixture 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 전체 설계 품질 정책 후보: 전체 `shared_design` 정책, `domain_language`, `vertical_slice`, `feedback_loop`, `tdd_trace`, `deep_module_interface`, `codebase_stewardship`, 상세 `manual_qa`, `two_stage_review_display`, 분리형 검증 정책, steward 정책 | 이름만 있음 | 설계 품질 담당 문서가 정확한 범위, 정책 경계, 증거 기대치, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |

<a id="operations-candidates"></a>
## 4. 운영 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 운영 강화 | 이후 후보 | 운영 담당 문서가 명령, 진단, 대체 동작, 보안 표현, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 향후 로컬 운영자 명령 묶음: `harness connect`, `harness serve mcp`, `harness doctor`, `harness projection refresh`, `harness reconcile`, `harness recover`, `harness export`, `harness artifacts check`, `harness conformance run` | 명령 이름만 있음 | 운영 담당 문서가 정확한 구문, 보안 태세, API와 저장소 영향, 보고, 대체 동작, 향후 승격에 필요한 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Export | 이후 후보 | Export 담당 문서가 저장소/아티팩트 처리, 가림, 생략, 무결성, 향후 유출 방지 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| Release Handoff | 이후 후보 | Handoff 담당 문서가 배포, 병합, 롤백, 운영 환경 권한을 별도 승격 전까지 외부에 남겨야 합니다. | 승격 전까지 없음 |
| 내보내기/인계 형식 | 이후 후보 | Export/Handoff 담당 문서가 파일 형식, 가림, 생략, 무결성, 출처 추적, 대체 동작, 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 복구와 reconcile | 이후 후보 | Operations, Storage, Projection, Reconcile, Security 담당 문서 규칙이 필요합니다. | 승격 전까지 없음 |
| 운영자 준비 상태와 `doctor` 접점 | 이후 후보 | 운영 담당 문서가 진단, 기능 확인, 보안 태세, 지원되지 않는 접점의 대체 동작을 정해야 합니다. | 승격 전까지 없음 |
| Projection 새로고침과 최신성 진단 | 이후 후보 | Projection 담당 문서가 Projection이 비권위 상태 보기로 남는 동작을 정해야 합니다. | 승격 전까지 없음 |
| 영속 Projection 작업과 Projection 작업 저장소 | 이후 후보 | Projection과 Storage 담당 문서가 작업 생명주기, 저장 행, 최신성, 실패 동작, 증명 기대치를 정의해야 합니다. 현재 MVP는 읽는 시점의 간결한 상태/Projection만 사용합니다. | 승격 전까지 없음 |
| Projection reconcile과 관리 블록 drift 복구 | 이후 후보 | Projection, Core, API, Storage 담당 문서가 사람이 편집한 입력 처리, reconcile 결과, 복구 후보, 상태 변경 라우팅, 대체 불가 규칙, 증명 기대치를 정의해야 합니다. 사람이 편집한 Projection은 활성 상태가 아닙니다. | 승격 전까지 없음 |
| 예방형 프로필, 격리형 프로필, 명령 관찰, 네트워크 관찰, 비밀값 접근 관찰, 접점 자체 아티팩트 캡처, 도구 실행 전 차단, 격리를 위한 더 강한 로컬 역량 프로필 | 이후 후보 | Agent Integration, Security, API, Storage, Conformance 담당 문서가 정확한 역량 필드, 대상 동작, 대체 동작, 오류, 증명 경로를 정해야 합니다. | 승격 전까지 없음 |
| 명령 실행 관찰, 네트워크 관찰, 비밀값 접근 관찰 | 이후 역량 후보 | API, Core, Security, Agent Integration, Conformance 담당 문서가 정확한 요청 필드, 관찰 권한, 대체 동작, 공개 오류, 저장소 영향, 증명 기대치를 정의해야 합니다. | 승격 전까지 없음 |
| 명령/네트워크/비밀값 도구 실행 전 차단 | 이후 예방형 후보 | 향후 예방형 담당 문서가 정확한 차단 메커니즘, 대상 동작, 대체 동작, 사용자에게 보이는 보장 표현, 증명 경로를 정의해야 합니다. | 승격 전까지 없음 |
| 향후 적합성 실행 진입점 | 런타임 fixture가 생긴 뒤의 이후 후보 | 실행기, 스위트, 검증 주장, API, 저장소, 이벤트, 아티팩트, 오류, 보고 계약이 필요합니다. | 승격 전까지 없음 |

## 5. 이후 API 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| `harness.next` | 메서드 이름만 있음 | 별도 다음 행동 페이로드를 위한 담당 문서 활성화가 필요합니다. MVP는 계속 `harness.status.next_actions`를 씁니다. | 승격 전까지 없음 |
| `harness.launch_verify` | 메서드 이름만 있음 | Eval/검증 담당 문서가 기능 처리, 기준선 최신성, 정직한 격리 표현을 정해야 합니다. | 승격 전까지 없음 |
| `harness.record_eval` | 메서드 이름만 있음 | Eval 담당 문서가 독립성 검증, 아티팩트 참조, 관문/보증 갱신을 정해야 합니다. | 승격 전까지 없음 |
| `harness.record_manual_qa` | 메서드 이름만 있음 | 수동 QA 담당 문서가 면제 경로, 아티팩트, 발견 사항, 관문 영향을 정해야 합니다. | 승격 전까지 없음 |
| 이후 읽기 전용 리소스: policy, evidence-manifest, surface, report, bundle, journey, design | 리소스 이름만 있음 | 각 리소스 담당 문서가 읽기 전용 계약과 변경 부작용이 없음을 정해야 합니다. | 승격 전까지 없음 |
| 이후 `harness.record_run` 분기: verification input, feedback-loop updates, TDD trace updates | 분기 이름만 있음 | `record_run` 담당 문서 활성화와 단일 분기 페이로드 규칙이 필요합니다. | 승격 전까지 없음 |
| 명령 관찰, 네트워크 관찰, 비밀값 접근 관찰에 대한 역량 조건부 `prepare_write` / `record_run` 관찰 | 이후 후보 | API, Core, Security, Agent Integration, Conformance 담당 문서가 정확한 요청 필드, 호환성 확인, validator 동작, 공개 오류, 저장소 영향, 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 이후 사용자 판단 분기: `qa_waiver`, `verification_risk_acceptance`, waiver, reconcile, residual-risk, richer acceptance visibility | 분기 이름만 있음 | 사용자 판단 담당 문서 활성화와 대체 불가 규칙이 필요합니다. | 승격 전까지 없음 |

<a id="later-schema-candidates"></a>
## 6. 이후 스키마 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 이후 스키마 확장 | 스키마 이름만 있음 | 승격된 담당 문서가 정확한 필드와 검증기를 활성 계약에 정의해야 합니다. | 승격 전까지 없음 |
| 역량 프로필 지원 필드: `command_observation_supported`, `network_observation_supported`, `secret_access_observation_supported`, `artifact_capture_supported`, `pre_tool_blocking_supported`, `isolation_supported` | 필드 이름만 있음 | 승격된 Agent Integration, API/스키마, Security, Storage, Conformance 담당 문서가 정확한 프로필 형태, 대상 동작, 대체 동작, 검증, 저장소, 오류, 증명 기대치를 정의해야 합니다. 기준 `reference-local-mcp`는 활성 프로필에서 이 필드를 생략하며 해당 역량을 지원하지 않는 것으로 다룹니다. | 승격 전까지 없음 |
| 역량 조건부 `Write Authorization` 관찰 필드: `intended_commands`, `intended_network`, `intended_secret_scope`; 명령 관찰, 네트워크 관찰, 비밀값 접근 관찰 범주 이름: `network_write`, `external_service_write`, `secret_access` | 필드와 값 이름만 있음 | 승격된 API/스키마 담당 문서가 정확한 형태, 프로필 조건, 검증, 저장소, `record_run` 호환성 의미를 정의해야 합니다. 기준 `reference-local-mcp`는 활성 `AuthorizedAttemptScope`나 활성 `SensitiveCategory`에 이 필드와 값을 포함하지 않습니다. | 승격 전까지 없음 |
| 이후 actor, producer, capture source 값: `evaluator`, `operator`, `capture_adapter` | 값 이름만 있음 | 승격된 Eval, 운영, 캡처, API/스키마, 저장소 담당 문서가 정확한 요청 권한, 아티팩트 관계, 대체 동작, 증명 기대치를 정의해야 합니다. 현재 MVP의 활성 표에는 이 값이 없습니다. | 승격 전까지 없음 |
| `captured_artifact`와 캡처된 아티팩트 핸들 | 값 이름만 있음 | 승격된 캡처/API/스키마/저장소 담당 문서가 정확한 핸들 출처, 역량 프로필 조건, 검증, 저장소, 가림 처리, 대체 동작, 증명 기대치를 정의해야 합니다. 현재 MVP는 대신 `stage_artifact`와 `source_kind=staged_file`을 사용합니다. | 승격 전까지 없음 |
| 이후 닫기와 보증 필드: `verifying`, `qa`, `completed_verified`, `detached_verified`, `design_gate`, `verification_gate`, `qa_gate`, 수동 QA 관문, 설계 정책 차단 사유, 보증 차단 사유 | 필드 이름만 있음 | Core/API 담당 문서 활성화, 닫기 대체 불가 규칙, 정확한 활성 스키마 필드, 대체 동작, 증명 기대치가 필요합니다. | 승격 전까지 없음 |
| 이후 다음 행동 값: `launch_verify`, `record_eval`, `record_manual_qa`, `reconcile` | 값 이름만 있음 | 대응 API 또는 담당 문서 활성화가 필요합니다. | 승격 전까지 없음 |
| 추천 playbook과 판단 맥락 | 메타데이터 이름만 있음 | Agent Integration/API 담당 문서가 메타데이터를 읽기 전용으로 두고 상태를 만족시키지 못하게 해야 합니다. | 승격 전까지 없음 |
| 이후 참조와 아티팩트 값: bundle, manifest, QA capture, export component, design, Eval, 수동 QA, TDD, projection, related refs | 값 이름만 있음 | ArtifactRef, StateRecordRef, Storage, 관련 담당 문서 활성화가 필요합니다. | 승격 전까지 없음 |
| `ValidatorResult` 이후 stable ID와 정책 계열: design, design-policy, autonomy, feedback-loop, TDD, stewardship, residual-risk, shared-design, manual-QA, context-hygiene checks | ID와 계열 이름만 있음 | Validator 담당 문서가 stable ID, 심각도, waiver 경계, 닫기 영향, 향후 fixture 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| waiver, reconcile, 잔여 위험 분기 | 분기 이름만 있음 | 사용자 판단, Core, 닫기 담당 문서 규칙이 필요합니다. | 승격 전까지 없음 |

<a id="later-template-candidates"></a>
## 7. 이후 템플릿 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| Decision Packet full-format presentation (`DEC`), `APR`, Approval Card, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, Verification Result Card, `MANUAL-QA`, Manual QA Card, `TASK`, `DIRECT-RESULT`, `JOURNEY-CARD`, `DESIGN`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `TDD-TRACE`, `EXPORT` | 템플릿 이름만 있음 | 템플릿 담당 문서 지정, 원천 기록, 대체 동작, 대체 불가 규칙, 최신성 동작, 향후 승격에 필요한 증명 경로 기대치가 필요합니다. | 승격 전까지 없음 |

<a id="future-fixture-families"></a>
## 8. 향후 fixture 계열

아래 긴 영어 항목은 향후 fixture 계열 이름만 보존한 것입니다. 현재 MVP 요구사항도 아니고 실행 가능한 적합성 모음도 아닙니다.

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| Intake and decision routing; Core, evidence, verification, and close; Artifact redaction and export non-leakage; Agency and user-judgment separation; Connector capability honesty; Design-quality and stewardship; Context hygiene and resume freshness; Projection, reconcile, and verification boundary; Operations diagnostics, export, recover, and handoff; Browser QA Capture | 향후 fixture 계열 이름만 있음 | 적합성 담당 문서 지정, 정확한 fixture 형태, 검증 주장, 페이로드, API, 저장소, 오류 영향, 향후 승격에 필요한 증명 경로 기대치가 필요합니다. | 승격 전까지 없음 |

## 9. 넓은 향후 후보

| 후보 | 상태 | 승격 경계 | 현재 MVP 영향 |
|---|---|---|---|
| 대시보드, 호스팅 작업 흐름, 아티팩트 대시보드, 더 풍부한 카드와 시각화 | 이후 후보 | 파생 표시 담당 문서가 읽기 전용, 비권위 동작을 정해야 합니다. | 승격 전까지 없음 |
| Verification Result Card와 더 풍부한 검증 작업 흐름 | 이후 후보 | Projection/template, Core/API, Eval, 수동 QA 담당 문서가 원천 기록, 최신성, 대체 불가 규칙, 대체 동작, 수동 QA 경계, 증명 경로 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 브라우저 캡처 자동화 | 이후 후보 | Capture 담당 문서가 가림/PII, 보존, 대체 동작, QA/수락 대체 불가 규칙을 정해야 합니다. | 승격 전까지 없음 |
| 접점 간 검증 | 이후 후보 | Core/Eval 담당 문서가 반환 기록, 독립성, 지원되지 않는 접점의 대체 동작을 정해야 합니다. | 승격 전까지 없음 |
| 더 넓은 커넥터, 커넥터 마켓플레이스, 호스팅 UI, 호스팅/원격 런타임 | 이후 후보 | 커넥터/API/보안 담당 문서와 향후 로컬 권위 경계 증명 기대치가 필요합니다. | 승격 전까지 없음 |
| 커넥터 적합성 생태계 | 이후 후보 | 커넥터, API, 보안, 적합성 담당 문서가 역량 주장, 커넥터 검증 주장, 스위트/보고 형식, 마켓플레이스 주장, 증명 기대치를 정해야 합니다. | 승격 전까지 없음 |
| 네이티브 후크, 예방형 guard 확장, 고급 sidecar 감시 | 이후 후보 | 예방형, 격리, 임의 도구 제어 주장을 하기 전에 담당 문서가 증명한 대상 메커니즘이 필요합니다. | 승격 전까지 없음 |
| Context Index, 로컬 파생 지표, 장기 지표 | 이후 후보 | 읽기 전용 검색/진단 담당 문서가 필요하며 권한이나 닫기 효과가 없어야 합니다. | 승격 전까지 없음 |
| 팀 작업 흐름, 권한, 공유 역량 집합, 오케스트레이션, 병렬 흐름 | 이후 후보 | 범위, 권한, 허가 체계, 사용자 소유 판단 담당 문서가 필요합니다. | 승격 전까지 없음 |
| 고급 내보내기, 릴리스/배포/카나리/롤백/병합/운영 환경 모니터링 자동화 | 이후 후보 | 별도 담당 범위가 필요합니다. 명시적으로 승격하기 전까지 배포와 운영 환경 권한은 외부에 남습니다. | 승격 전까지 없음 |
| 고급 validator, 설계 정책 validator, 언어 또는 인터페이스 확인 | 이후 후보 | Validator 담당 문서가 정확한 ID, 심각도, waiver 경계, 닫기 영향, fixture 동작을 정해야 합니다. | 승격 전까지 없음 |
