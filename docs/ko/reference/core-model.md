# 핵심 모델 참조

이 참조 문서는 향후 하네스 Core의 권한 모델을 정의하는 참조 문서입니다. 이 저장소에는 아직 하네스 런타임이나 서버 구현이 없으며, 현재 문서가 구현 완료 상태인지는 유지보수자가 담당하는 [구현 가이드](../build/implementation-guide.md)의 상태로만 판단합니다.

Core는 Task 범위, 사용자 소유 판단, 증거, 검증 기대치, 닫기 준비 상태, 잔여 위험을 다루는 로컬 기준 기록입니다. 이 문서는 그 경계의 제품 의미를 담당합니다. 보안 보장 표현과 비주장은 [보안](security.md)이 담당합니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- Core 권한 불변 조건과 대체 금지 규칙.
- Task 범위, Change Unit 경계, 사용자 소유 판단, 증거, 닫기 준비 상태, 정직한 닫기, 면제, 잔여 위험의 제품 의미.
- 개념 수준의 생명주기와 관문 경계.
- `WriteDecisionReason`, 닫기 준비 상태의 차단 사유, `CloseReadinessBlocker`의 차이.
- Core 개념이 API, 저장소, 보안, 상태 보기, 이후 후보 자료와 닿을 때의 담당 문서 연결.

이 문서는 담당하지 않습니다.

- 공개 API 페이로드 스키마, 응답 분기 형태, 공통 요청 래퍼, 메서드 결과 구조. [API 메서드](api/methods.md), 메서드 담당 문서, [API 코어 스키마](api/schema-core.md), API 스키마 담당 문서를 봅니다.
- 저장소 DDL, 지속 저장 JSON 배치, 잠금, 마이그레이션, Runtime Home 배치, 메서드별 저장 효과. [저장소 기록](storage-records.md), [저장 효과](storage-effects.md), [아티팩트 저장소](storage-artifacts.md), [저장소 버전 관리](storage-versioning.md)를 봅니다.
- 정확한 활성 enum 형태 값과 API 필드 목록. [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md)를 봅니다.
- 공개 오류 코드 정의나 오류 우선순위. [API 오류](api/errors.md)를 봅니다.
- 렌더링된 상태 보기 본문, 템플릿 문구, 커넥터 사용법, 보안 보장 어휘, 이후 후보 목록.

정확한 식별자는 의미 설명에 필요할 때만 이 문서에 둘 수 있습니다. 해당 스키마 형태, 값 집합, 저장 효과, 공개 오류 동작은 연결된 담당 문서가 맡습니다.

<a id="2-kernel-불변-조건"></a>
## 2. Core 권한 불변 조건

| 불변 조건 | 상세 |
|---|---|
| Core가 소유한 상태가 기준입니다. | [Core 상태 권한](#core-invariant-state-authority)을 참고합니다. |
| 하네스는 하네스 기록을 다룹니다. | [하네스 기록 경계](#core-invariant-harness-record-boundary)를 참고합니다. |
| 제품 쓰기에는 호환되는 활성 범위가 필요합니다. | [제품 쓰기 범위](#core-invariant-product-write-scope)를 참고합니다. |
| 사용자 소유 판단은 사용자에게 남습니다. | [사용자 소유 판단 권한](#core-invariant-user-owned-judgment)을 참고합니다. |
| `Write Authorization` 생성은 좁습니다. | [`Write Authorization` 생성](#core-invariant-write-authorization-creation)을 참고합니다. |
| Run은 실제로 일어난 일을 기록합니다. | [Run 기록 권한](#core-invariant-run-record-authority)을 참고합니다. |
| 증거 기록은 실제 기록한 주장만 뒷받침합니다. | [증거 기록 권한](#core-invariant-evidence-record-authority)을 참고합니다. |
| 닫기는 정직해야 합니다. | [정직한 닫기](#core-invariant-honest-close)를 참고합니다. |
| 현재 MVP와 이후 후보는 분리됩니다. | [현재 MVP와 이후 후보 경계](#core-invariant-mvp-later-boundary)를 참고합니다. |

<a id="core-invariant-state-authority"></a>
### Core 상태 권한

개념:
- Core가 소유한 상태가 하네스 동작의 기준입니다.

같은 것이 아님:
- 대화
- 보고서
- 생성된 Markdown
- 상태 보기
- 템플릿 출력

담당 문서 링크:
- [상태 보기 권한 참조](projection-and-templates.md)
- [템플릿 본문](template-bodies.md)

<a id="core-invariant-harness-record-boundary"></a>
### 하네스 기록 경계

개념:
- 하네스는 하네스 기록과 상태 전이를 다룹니다.

같은 것이 아님:
- 일반 보안 통제 표면

담당 문서 링크:
- [보안](security.md)

<a id="core-invariant-product-write-scope"></a>
### 제품 쓰기 범위

조건:
- 제품 쓰기에는 호환되는 활성 범위가 필요합니다.

효과:
- 현재 Task와 Change Unit 밖의 쓰기 경로는 호환되기 전에 다시 구체화해야 합니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-invariant-user-owned-judgment"></a>
### 사용자 소유 판단 권한

개념:
- 사용자 소유 판단은 사용자에게 남습니다.

같은 것이 아님:
- 에이전트 추론
- 포괄적 동의
- 증거
- 상태 보기 문구
- 생성된 요약

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)

<a id="core-invariant-write-authorization-creation"></a>
### `Write Authorization` 생성

조건:
- 소비 가능한 `Write Authorization`은 `dry_run=false`인 allowed `prepare_write` 경로만 만듭니다.

효과:
- `Write Authorization`은 호환되는 제품 파일 쓰기 시도 하나에 한 번만 쓰입니다.

허용되지 않는 것:
- 재사용 가능한 범위나 일반 권한으로 취급하지 않습니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)

<a id="core-invariant-run-record-authority"></a>
### Run 기록 권한

개념:
- Run은 실제로 일어난 일을 기록합니다.

같은 것이 아님:
- 범위, 필요한 판단, 민감 동작 승인, `Write Authorization`이 없었던 작업의 사후 승인

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)

<a id="core-invariant-evidence-record-authority"></a>
### 증거 기록 권한

개념:
- 증거 기록은 실제 기록한 주장만 뒷받침합니다.

같은 것이 아님:
- 수락
- QA
- 검증
- 잔여 위험 수락
- 기록되지 않은 사실의 증명

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-invariant-honest-close"></a>
### 정직한 닫기

조건:
- 닫기 관련 차단 사유가 남아 있습니다.

효과:
- Core는 Task를 성공 완료로 처리하지 않고 차단 사유를 보고합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-invariant-mvp-later-boundary"></a>
### 현재 MVP와 이후 후보 경계

개념:
- 현재 MVP와 이후 후보는 분리됩니다.

승격 전까지 비활성:
- 이후 검증
- 수동 QA
- 풍부한 면제
- 보증 자료

담당 문서 링크:
- [현재 MVP 범위](scope.md)
- [범위 참조](scope.md)

## 3. 핵심 엔터티

아래 엔터티는 권한 관계를 설명합니다. 저장소 테이블이나 API 본문을 정의하지 않습니다.

| 엔터티 | 상세 |
|---|---|
| Task | [Task](#core-entity-task-boundary)를 참고합니다. |
| Change Unit | [Change Unit](#core-entity-change-unit)을 참고합니다. |
| Autonomy Boundary | [Autonomy Boundary](#core-entity-autonomy-boundary)를 참고합니다. |
| `user_judgment` | [`user_judgment`](#core-entity-user-judgment)를 참고합니다. |
| `Write Authorization` | [`Write Authorization`](#core-entity-write-authorization-boundary)을 참고합니다. |
| Run | [Run](#core-entity-run)을 참고합니다. |
| 증거 요약 | [증거 요약](#core-entity-evidence-summary)을 참고합니다. |
| `ArtifactRef` | [`ArtifactRef`](#core-entity-artifactref-boundary)를 참고합니다. |
| 차단 사유 | [차단 사유](#core-entity-blocker)를 참고합니다. |
| 잔여 위험 요약 | [잔여 위험 요약](#core-entity-residual-risk-summary)을 참고합니다. |
| 상태 보기 출력 | [상태 보기 출력](#core-entity-projection-output)을 참고합니다. |
| 템플릿 출력 | [템플릿 출력](#core-entity-template-output-boundary)을 참고합니다. |
| `ShapingReadiness` | [`ShapingReadiness`](#core-entity-shaping-readiness)를 참고합니다. |

<a id="core-entity-task-boundary"></a>
### Task

개념:
- Task는 구체화, 실행, 차단, 닫기의 대상이 되는 사용자 가치 단위입니다.

담당 문서 링크:
- 정확한 생명주기 값과 공개 상태 필드는 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md)가 담당합니다.

<a id="core-entity-change-unit"></a>
### Change Unit

개념:
- Change Unit은 쓰기 가능한 작업의 활성 범위 경계입니다.

같은 것이 아님:
- 최종 수락
- 증거
- 범위를 조용히 넓히는 허가

<a id="core-entity-autonomy-boundary"></a>
### Autonomy Boundary

개념:
- Autonomy Boundary는 Change Unit 안에서 에이전트가 가질 수 있는 재량 범위입니다.

같은 것이 아님:
- 범위 확장
- 민감 동작 승인
- 사용자 소유 판단을 대신할 권한

<a id="core-entity-user-judgment"></a>
### `user_judgment`

개념:
- `user_judgment`는 사용자가 소유하는 결정을 위한 기록군입니다.

효과:
- 기록된 판단이 영향을 받는 대상, 범위, 결과, 닫기 또는 쓰기 영향과 맞으면 호환성에 반영될 수 있습니다.

같은 것이 아님:
- 활성 범위 변경
- 증거 생성
- `Write Authorization`
- 그 판단 종류를 정확히 묻고 기록하지 않은 상태에서의 민감 동작 승인, 최종 수락, 잔여 위험 수락
- 그 자체의 Task 닫기

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)

<a id="core-entity-write-authorization-boundary"></a>
### `Write Authorization`

개념:
- `Write Authorization`은 호환되는 제품 파일 쓰기 시도 하나를 위한 오래 남는 1회용 Core 권한 기록입니다.

허용되지 않는 것:
- OS 권한, 명령 승인, 민감 동작 승인, 최종 수락, 재사용 가능한 범위로 취급하지 않습니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [실행 기록 메서드](api/method-record-run.md)
- [API 상태 스키마](api/schema-state.md)
- [저장 효과](storage-effects.md)

<a id="core-entity-run"></a>
### Run

개념:
- Run은 실행 또는 관찰 기록입니다.

같은 것이 아님:
- 빠진 범위, 빠진 판단, 빠진 승인, 빠진 `Write Authorization`을 사후 승인하는 기록.
- 읽기 전용 또는 구체화 전용 Run이 이후 제품 쓰기를 호환되게 만든다는 의미.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)

<a id="core-entity-evidence-summary"></a>
### 증거 요약

개념:
- 증거 요약은 닫기 관련 뒷받침, 공백, 참조, 범위 기대치를 위한 간결한 Core 경로입니다.

같은 것이 아님:
- 최종 수락.
- 잔여 위험 수락.
- 담당 문서가 승격하지 않은 전체 `Evidence Manifest` 동작.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)

<a id="core-entity-artifactref-boundary"></a>
### `ArtifactRef`

개념:
- `ArtifactRef`는 아티팩트 담당 문서가 허용할 때 증거에 쓸 수 있는 오래 남는 참조입니다.

담당 문서 링크:
- 아티팩트 형태, 스테이징, 승격, 무결성, 가림 처리, 본문 읽기 규칙은 [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md)가 담당합니다.

<a id="core-entity-blocker"></a>
### 차단 사유

개념:
- 차단 사유는 진행, 쓰기, Run 기록, 닫기가 정직하게 이어질 수 없는 구조화된 이유입니다.

같은 것이 아님:
- 상태 보기 문구
- 포괄적 승인
- 성공처럼 보이는 닫기 결과

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)

<a id="core-entity-residual-risk-summary"></a>
### 잔여 위험 요약

개념:
- 잔여 위험 요약은 알려진 남은 불확실성, 한계, 절충점을 보여 주는 간결한 경로입니다.

같은 것이 아님:
- 검증
- 증거 충분성
- 최종 수락
- 무위험 결과
- 담당 문서가 승격하지 않은 풍부한 잔여 위험 기록이나 보증 표시

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)
- [범위 참조](scope.md)

<a id="core-entity-projection-output"></a>
### 상태 보기 출력

개념:
- 상태 보기 출력은 Core 상태와 참조에서 파생된 표시입니다.

같은 것이 아님:
- 권한
- 증거
- 수락

담당 문서 링크:
- [상태 보기 권한 참조](projection-and-templates.md)

<a id="core-entity-template-output-boundary"></a>
### 템플릿 출력

개념:
- 템플릿 출력은 카드, 요청, 요약, 결과, 패킷의 렌더링 본문 문구입니다.

담당 문서 링크:
- 본문 기대치는 [템플릿 본문](template-bodies.md)이 담당합니다.

허용되지 않는 것:
- 읽기 쉽거나 사람이 고쳤다는 이유로 권한이 되지 않습니다.

<a id="core-entity-shaping-readiness"></a>
### `ShapingReadiness`

개념:
- `ShapingReadiness`는 다음 안전한 행동을 위해 Core 상태에서 파생되는 간결한 보기입니다.

입력:
- Task.
- Change Unit.
- 대기 중인 판단.
- 증거 요약.
- 차단 사유.
- 다음 행동 상태.

담당 문서 링크:
- 준비 의미는 Core가 담당합니다. 현재 담당 상태가 다음 안전한 행동에 충분히 구체적인지입니다.
- API 필드는 [API 상태 스키마](api/schema-state.md)가 담당합니다.

## 4. 사용자 소유 판단

개념:
- 사용자 소유 판단은 하네스가 추론하지 않고 사용자에게 묻거나 사용자의 기록된 선택으로 보존해야 하는 경계입니다.
- 이 문서는 제품 의미를 담당합니다. 정확한 스키마 필드와 입력 형태는 판단 스키마 담당 문서가 담당합니다.

입력:
- 사용자에게 속한 제품, 기술, 범위, 민감 동작, 최종 수락, 잔여 위험, 취소 질문.
- 하나의 사용자 응답이 여러 판단 종류를 만족하려는 경우 영향을 받는 대상, 범위, 결과, 닫기 또는 쓰기 영향.

같은 것이 아님:
- 에이전트 추론, 포괄적 동의, 증거, 상태 보기 문구, 생성된 요약.
- 그 판단 종류를 정확히 묻고 기록한 경우가 아니라면 활성 범위 변경, `Write Authorization`, 민감 동작 승인, 최종 수락, 잔여 위험 수락.

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)

판단 종류:

| 판단 종류 | 사용자가 소유하는 결정 |
|---|---|
| `product_decision` | 사용자에게 보이는 동작, 사용자 흐름, 문구, UX, 접근성, 릴리스 약속, 제품상 절충, 사용자 가치. |
| `technical_decision` | [`technical_decision`](#core-judgment-technical-decision)을 참고합니다. |
| `scope_decision` | 범위 확장, 범위 밖 항목 제거, Change Unit 경계 변경, Autonomy Boundary 변경. |
| `sensitive_approval` | 경계가 정해진 `SensitiveActionScope` 안에서 이름 붙은 민감 단계에 대한 허가. |
| `final_acceptance` | 닫기 경로가 수락을 요구할 때 사용자가 결과를 판단하는 것. |
| `residual_risk_acceptance` | 요청한 닫기를 위해 이름 붙은 보이는 잔여 위험을 사용자가 수락하는 것. |
| `cancellation` | 성공 완료 결과 없이 Task를 멈추는 것. |

<a id="core-judgment-technical-decision"></a>
### `technical_decision`

조건:
- 질문이 아키텍처, 의존성이나 외부 서비스 도입, 인증 방향, 마이그레이션에 관한 것입니다.
- 질문이 공개 인터페이스, 호환성을 깨는 변경, 데이터 보관, 개인정보, 보안에 관한 것입니다.
- 질문이 그 밖의 중요하고 되돌리는 비용이 큰 기술 방향에 관한 것입니다.

에이전트 재량:
- 받아들인 범위와 수락 기준 안에서 제품 동작, 기술 방향, 범위, 보안/개인정보 태세, 호환성, 되돌리는 비용이 큰 아키텍처를 바꾸지 않는 평범한 구현 세부사항은 에이전트가 정할 수 있습니다.

같은 것이 아님:
- 새 권한 시스템.
- 다른 판단 종류를 조용히 만족하는 포괄적 동의.

여러 판단:
- "진행해", "좋아", "looks good" 같은 말은 다른 판단 종류를 조용히 만족할 수 없습니다.
- 하나의 답변이 여러 판단을 만족하려면 프롬프트가 그 판단들을 명시적으로 물었고, Core가 영향을 받는 대상, 범위, 결과, 닫기 또는 쓰기 영향과 함께 각 판단을 호환되게 기록해야 합니다.

## 5. 대체 금지 규칙

| 경계 | 상세 |
|---|---|
| 표시와 생성된 텍스트 | [표시와 생성된 텍스트](#core-non-substitution-displays)를 참고합니다. |
| 증거와 Run 기록 | [증거와 Run 기록](#core-non-substitution-evidence-runs)을 참고합니다. |
| `final_acceptance` | [`final_acceptance`](#core-non-substitution-final-acceptance)를 참고합니다. |
| `residual_risk_acceptance` | [`residual_risk_acceptance`](#core-non-substitution-residual-risk-acceptance)를 참고합니다. |
| `sensitive_approval` | [`sensitive_approval`](#core-non-substitution-sensitive-approval)을 참고합니다. |
| `Write Authorization`과 `AuthorizedAttemptScope` | [`Write Authorization`과 `AuthorizedAttemptScope`](#core-non-substitution-write-authorization)를 참고합니다. |
| `WriteDecisionReason` | [`WriteDecisionReason`](#core-non-substitution-write-decision-reason)을 참고합니다. |
| `CloseReadinessBlocker` | [`CloseReadinessBlocker`](#core-non-substitution-close-readiness-blocker)를 참고합니다. |
| 면제 또는 수락된 위험 | [면제 또는 수락된 위험](#core-non-substitution-waiver-risk)을 참고합니다. |

<a id="core-non-substitution-displays"></a>
### 표시와 생성된 텍스트

적용 대상:
- 대화, 보고서, 생성된 Markdown, 상태 보기 문구, 상태 카드.

대신할 수 없는 것:
- Core가 소유한 상태.

<a id="core-non-substitution-evidence-runs"></a>
### 증거와 Run 기록

적용 대상:
- 증거, 로그, 스크린샷, 아티팩트, 테스트 출력, Run 기록.

대신할 수 없는 것:
- 최종 수락
- 향후 수동 QA
- 향후 검증
- 잔여 위험 수락

<a id="core-non-substitution-final-acceptance"></a>
### `final_acceptance`

대신할 수 없는 것:
- 증거
- QA
- 검증
- 민감 동작 승인
- 범위 변경
- 잔여 위험 수락
- 차단 사유 우회

<a id="core-non-substitution-residual-risk-acceptance"></a>
### `residual_risk_acceptance`

대신할 수 없는 것:
- 검증
- 증거 충분성
- QA
- 최종 수락
- 무위험 결과

<a id="core-non-substitution-sensitive-approval"></a>
### `sensitive_approval`

대신할 수 없는 것:
- 제품 방향
- 기술 방향
- 범위
- 정확성
- 증거
- QA
- 최종 수락
- 잔여 위험 수락
- `Write Authorization`

<a id="core-non-substitution-write-authorization"></a>
### `Write Authorization`과 `AuthorizedAttemptScope`

대신할 수 없는 것:
- 명령 승인
- 의존성 승인
- 호스트, 네트워크, 비밀값 접근
- 배포 승인
- 파괴적 동작 승인
- 시스템 접근
- 최종 수락

<a id="core-non-substitution-write-decision-reason"></a>
### `WriteDecisionReason`

대신할 수 없는 것:
- 닫기 차단 사유
- `CloseReadinessBlocker`

<a id="core-non-substitution-close-readiness-blocker"></a>
### `CloseReadinessBlocker`

대신할 수 없는 것:
- `prepare_write` 판단 사유
- 닫기 준비 상태 개념 전체
- 증거
- 수락
- 그 자체의 저장 효과

<a id="core-non-substitution-waiver-risk"></a>
### 면제 또는 수락된 위험

대신할 수 없는 것:
- 자동 성공
- 검증
- 증거
- 최종 수락
- 남은 담당 경로 없는 닫기

사용자에게 보이는 압축 표시가 이 경계를 요약할 수는 있지만, 권한 경계를 하나로 뭉개면 안 됩니다.

## 6. Task 생명주기

| 생명주기 영역 | 상세 |
|---|---|
| 입력과 구체화 | [입력과 구체화](#core-lifecycle-intake-and-shaping)를 참고합니다. |
| 범위 업데이트 | [범위 업데이트](#core-lifecycle-scope-update)를 참고합니다. |
| 실행과 관찰 | [실행과 관찰](#core-lifecycle-execution-observation)을 참고합니다. |
| 대기 또는 차단 | [대기 또는 차단](#core-lifecycle-waiting-blocked)을 참고합니다. |
| 닫기 시도 | [닫기 시도](#core-lifecycle-close-attempt)를 참고합니다. |
| 종료 결과 | [종료 결과](#core-lifecycle-terminal-outcome)를 참고합니다. |

<a id="core-lifecycle-intake-and-shaping"></a>
### 입력과 구체화

효과:
- 평이한 사용자 의도를 구체적 목표, 활성 범위, 범위 밖 항목, 수락 기준, Autonomy Boundary, 다음 안전한 행동으로 바꿉니다.

필요한 정직성:
- 사용자 소유 문제가 다음 안전한 행동을 막으면 추론하지 말고 판단 필요성을 드러냅니다.

<a id="core-lifecycle-scope-update"></a>
### 범위 업데이트

효과:
- 받아들인 범위나 Change Unit 변경은 `harness.update_scope`를 통해 적용합니다.

같은 것이 아님:
- `scope_decision` 기록이 그 자체로 활성 범위를 바꾸는 것.

<a id="core-lifecycle-execution-observation"></a>
### 실행과 관찰

효과:
- Run 기록은 행동이나 관찰을 설명합니다.

필요한 정직성:
- 제품 파일 쓰기는 활성 범위와 `Write Authorization`에 호환되어야 합니다.
- 읽기 전용 작업은 이후 쓰기를 승인하지 않습니다.

<a id="core-lifecycle-waiting-blocked"></a>
### 대기 또는 차단

조건:
- 담당 경로가 없거나, 오래됐거나, 호환되지 않거나, 우회하기 안전하지 않습니다.

효과:
- 진행이 멈춥니다.
- 차단 사유는 공백을 숨기지 말고 다음 안전한 담당 경로를 가리킵니다.

<a id="core-lifecycle-close-attempt"></a>
### 닫기 시도

개념:
- Core는 Task를 정직하게 닫을 수 있는지 평가합니다.

입력:
- 마지막 대화 요약만이 아니라 현재 Core 상태.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-lifecycle-terminal-outcome"></a>
### 종료 결과

효과:
- 완료, 취소, 대체가 Task 경로를 끝냅니다.

허용되지 않는 것:
- 취소와 대체는 종료이지만 성공 완료가 아닙니다.
- 완료에 필요한 증거, 수락, 위험 요구를 만족하지 않습니다.

## 7. 현재 관문

관문은 진행, 쓰기, Run 기록, 닫기를 위한 호환성 요약입니다. 이 문서는 제품 의미를 담당합니다. 공개 필드, 정확한 값, API 형태는 [API 상태 스키마](api/schema-state.md), [API 값 집합](api/schema-value-sets.md)이 담당합니다.

| 관문 영역 | 상세 |
|---|---|
| 범위 관문 | [범위 관문](#core-gate-scope)을 참고합니다. |
| 판단 관문 | [판단 관문](#core-gate-decision)을 참고합니다. |
| 민감 동작 승인 관문 | [민감 동작 승인 관문](#core-gate-sensitive-action-approval)을 참고합니다. |
| 쓰기 호환성 관문 | [쓰기 호환성 관문](#core-gate-write-compatibility)을 참고합니다. |
| 증거 관문 | [증거 관문](#core-gate-evidence)을 참고합니다. |
| 수락 관문 | [수락 관문](#core-gate-acceptance)을 참고합니다. |
| 잔여 위험 관문 | [잔여 위험 관문](#core-gate-residual-risk)을 참고합니다. |
| 닫기 준비 상태 관문 | [닫기 준비 상태 관문](#core-gate-close-readiness)을 참고합니다. |

<a id="core-gate-scope"></a>
### 범위 관문

조건:
- 활성 범위와 Change Unit이 요청한 작업을 포함해야 합니다.

같은 것이 아님:
- 사용자를 대신한 제품 질문 결정
- 사용자를 대신한 기술 질문 결정

담당 문서 링크:
- [범위 업데이트 메서드](api/method-update-scope.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-gate-decision"></a>
### 판단 관문

조건:
- 진행, 쓰기, Run 기록, 닫기를 계속하려면 사용자 소유 판단이 먼저 해결되어야 합니다.

같은 것이 아님:
- 증거
- 민감 동작 승인
- 최종 수락
- 잔여 위험 수락

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-gate-sensitive-action-approval"></a>
### 민감 동작 승인 관문

조건:
- `SensitiveActionScope` 안의 이름 붙은 민감 단계에 승인이 필요합니다.

같은 것이 아님:
- `Write Authorization`
- 포괄적 권한
- 제품 정확성

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [쓰기 준비 메서드](api/method-prepare-write.md)

<a id="core-gate-write-compatibility"></a>
### 쓰기 호환성 관문

개념:
- 제품 파일 쓰기 시도가 활성 범위와 소비 가능한 `Write Authorization`에 호환되는지 보는 관문입니다.

허용되지 않는 것:
- 명령, 호스트, 네트워크, 비밀값, 배포, 파괴적 동작을 승인하지 않습니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [실행 기록 메서드](api/method-record-run.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-gate-evidence"></a>
### 증거 관문

조건:
- 닫기 관련 필수 뒷받침이 닫기 경로에 충분히 존재하고 사용할 수 있어야 합니다.

같은 것이 아님:
- 기록한 것 이상의 증명
- 사용자 수락
- 잔여 위험 수락

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)

<a id="core-gate-acceptance"></a>
### 수락 관문

조건:
- 필요한 최종 수락이 보이는 닫기 근거에 대해 존재해야 합니다.

같은 것이 아님:
- 증거 공백 채우기
- 잔여 위험 수락
- 범위 변경

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [Task 닫기 메서드](api/method-close-task.md)

<a id="core-gate-residual-risk"></a>
### 잔여 위험 관문

조건:
- 닫기 관련 잔여 위험이 보이고, 필요할 때 수락되어야 합니다.

같은 것이 아님:
- 검증
- 무위험 결과
- 최종 수락

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)

<a id="core-gate-close-readiness"></a>
### 닫기 준비 상태 관문

개념:
- 닫기 준비 상태 관문은 닫기 관련 모든 확인이 정직한 닫기를 뒷받침하는지 요약합니다.

효과:
- 닫기 차단 사유가 남아 있으면 담당 경로가 처리할 때까지 Task는 열린 상태로 남습니다.

같은 것이 아님:
- `CloseReadinessBlocker`
- `intent=complete`
- 사용자 수락만으로 충분하다는 의미

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)

검증과 수동 QA는 현재 MVP에서 개념 경계일 뿐 활성 관문이 아닙니다. 향후 담당 문서가 승격하기 전까지 활성 닫기 요구사항처럼 쓰면 안 됩니다.

## 8. 쓰기 권한 부여 경계

개념:
- `Write Authorization`은 제품 파일 쓰기 시도 하나를 현재 하네스 상태와 호환되게 만드는 Core 기록입니다.

생성:
- API 담당 문서가 정의하는 호환 `dry_run=false` `prepare_write` 경로를 통해서만 만들어집니다.

입력:
- 현재 하네스 상태.
- 활성 Task와 Change Unit 범위.
- 의도한 제품 파일 쓰기 시도.
- 호환되는 `dry_run=false` `prepare_write` 결과.

속성:
- 범위 제한: 의도한 제품 파일 쓰기 시도만 다루며, 미래 작업이나 더 넓은 프로젝트 영역을 다루지 않습니다.
- 1회용: 호환되는 제품 쓰기 Run이 한 번 소비합니다. 재사용, 재실행, 오래된 상태 동작은 API와 저장소 담당 문서가 맡습니다.
- 협력형: 연결된 에이전트나 접점에 하네스 상태와 호환되는 것을 알려 줍니다. OS 수준 예방을 강제하지 않습니다.

같은 것이 아님:
- `sensitive_approval`, 명령 승인, 의존성 승인, 호스트/네트워크/비밀값 접근, 배포 승인, 파괴적 동작 승인, 시스템 접근, 최종 수락.
- 쓰기가 실행되었다는 증명, 증거 생성, 수락, 잔여 위험 수락, Task 닫기.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [실행 기록 메서드](api/method-record-run.md)
- [API 상태 스키마](api/schema-state.md)
- [저장 효과](storage-effects.md)

판단 사유 경계:
- `WriteDecisionReason`은 `prepare_write` 판단 출력에 속합니다.
- `CloseReadinessBlocker`는 닫기 준비 상태의 차단 데이터를 표현합니다.
- 두 개념은 서로 다른 질문에 답하며 교환해서 쓰면 안 됩니다.

## 9. 실행과 증거의 권한

| 기록 | 상세 |
|---|---|
| Run | [Run 권한](#core-evidence-run-authority)을 참고합니다. |
| 증거 요약 | [증거 요약 권한](#core-evidence-summary-authority)을 참고합니다. |
| `ArtifactRef` | [`ArtifactRef` 증거 사용](#core-evidence-artifactref-use)을 참고합니다. |
| 상태 보기 또는 보고서 | [상태 보기 또는 보고서 권한](#core-evidence-projection-report-authority)을 참고합니다. |

<a id="core-evidence-run-authority"></a>
### Run 권한

세울 수 있는 것:
- 사용 가능한 맥락과 참조와 함께 실행 또는 관찰이 기록되었다는 사실.

세울 수 없는 것:
- 빠진 권한 부여, 빠진 판단, 빠진 승인이 사후에 존재했다는 사실.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)

<a id="core-evidence-summary-authority"></a>
### 증거 요약 권한

세울 수 있는 것:
- 특정 닫기 관련 주장에 대해 기록된 뒷받침, 공백, 참조, 범위 기대치가 있다는 사실.

세울 수 없는 것:
- 기록되지 않은 동작이 일어났다는 사실.
- 결과가 수락되었다는 사실.
- 잔여 위험이 수락되었다는 사실.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)

<a id="core-evidence-artifactref-use"></a>
### `ArtifactRef` 증거 사용

세울 수 있는 것:
- 아티팩트 담당 문서가 허용할 때 증거로 쓸 수 있는 아티팩트 참조가 있다는 사실을 세울 수 있습니다.

세울 수 없는 것:
- 기록된 무결성, 가림 처리, 가용성 사실을 넘어 아티팩트 본문이 안전하거나 충분하거나 읽을 수 있다는 사실을 세우지 않습니다.

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="core-evidence-projection-report-authority"></a>
### 상태 보기 또는 보고서 권한

세울 수 있는 것:
- 사용 가능한 상태와 참조에서 표시가 생성되었다는 사실.

세울 수 없는 것:
- 표시 자체가 권한이라는 사실.
- 표시 자체가 증거라는 사실.
- 표시 자체가 수락이라는 사실.

담당 문서 링크:
- [상태 보기 권한 참조](projection-and-templates.md)
- [템플릿 본문](template-bodies.md)

### 증거 권한

개념:
- 증거 기록은 기록된 범위에서 기록한 주장만 뒷받침합니다.

입력:
- Run 기록.
- 증거 요약.
- 아티팩트 담당 문서가 허용하는 증거용 아티팩트와 `ArtifactRef` 값.
- 관련 참조와 범위 기대치.

세울 수 있는 것:
- 통과한 테스트 로그는 이름 붙인 테스트를 뒷받침합니다.
- 스크린샷은 포착한 화면 상태를 뒷받침합니다.
- 아티팩트는 아티팩트 담당 문서가 표현한 내용과 무결성 사실만 뒷받침합니다.

같은 것이 아님:
- 더 넓은 정확성의 증명.
- 최종 수락, 향후 수동 QA, 향후 검증, 잔여 위험 수락.
- 기록되지 않은 동작의 증명.

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)
- [API 판단 스키마](api/schema-judgment.md)

<a id="close_task"></a>
## 10. 닫기 준비 상태

개념:
- 닫기 준비 상태는 현재 Task를 정직하게 닫을 수 있는지에 대한 Core 평가 개념입니다.

입력:
- 현재 Core 상태.
- Task 범위와 Change Unit 범위.
- 필요한 증거와 닫기 관련 아티팩트.
- 필요한 사용자 소유 판단.
- 필요한 민감 동작 승인.
- 쓰기와 Run 호환성.
- 해결되지 않은 차단 사유.
- 필요한 최종 수락.
- 필요한 경우 수락된 잔여 위험.
- 복구 제약.

같은 것이 아님:
- `CloseReadinessBlocker`.
- `intent=complete`.
- 사용자 수락만으로 충분하다는 의미.
- 사전 확인 거절.

스키마 경계:
- `CloseReadinessBlocker`는 닫기 차단 사유를 나타내는 데이터 표현이지, 닫기 준비 상태 평가 개념이 아닙니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)
- [저장 효과](storage-effects.md)
- [API 오류](api/errors.md)

`intent=complete` 닫기 시도에서 Core는 아래 개념 순서로 차단 사유를 평가합니다. 뒤의 행은 앞의 행을 대신 만족할 수 없습니다.

| 순서 | 확인 영역 | 닫기 준비 상태 의미 |
|---:|---|---|
| 1 | Task 생명주기 | 선택된 Task가 요청한 종료 경로에 들어갈 수 있어야 합니다. |
| 2 | 열려 있거나 복구되지 않은 Run | 열려 있거나, 안전하지 않거나, 중단되었거나, 호환되지 않거나, 복구되지 않은 Run 상태에는 닫기가 의존할 수 없습니다. |
| 3 | 범위와 Change Unit | 활성 범위, 수락 기준, 적용되는 완료 정책이 닫기 주장을 뒷받침해야 합니다. |
| 4 | 사용자 소유 판단 | 필요한 제품, 기술, 범위, 그 밖의 비민감 사용자 판단이 해결되어 있고 호환되어야 합니다. |
| 5 | 민감 동작 승인 | 필요한 민감 동작 승인이 경계가 정해진 단계와 호환되게 존재해야 합니다. |
| 6 | 쓰기와 Run 호환성 | 제품 쓰기 주장은 호환되는 권한 부여와 기록된 Run 관계로 뒷받침되어야 합니다. |
| 7 | 기준 상태와 접점 역량 | 기준 상태와 연결된 접점이 닫기 주장과 필요한 보장 표시를 정직하게 뒷받침해야 합니다. |
| 8 | 증거 충분성 | 필수 증거 범위가 닫기 근거에 대해 존재하고 최신이며 사용할 수 있어야 합니다. |
| 9 | 아티팩트 가용성 | 닫기 관련 아티팩트가 아티팩트 담당 규칙에 따라 사용할 수 있어야 합니다. |
| 10 | 최종 수락 | 필요한 최종 수락이 보이는 닫기 근거와 연결되어야 합니다. |
| 11 | 잔여 위험 표시 | 알려진 닫기 관련 위험은 사용자가 판단할 수 있을 만큼 보여야 합니다. |
| 12 | 잔여 위험 수락 | 보이는 잔여 위험의 필수 수락은 요청한 닫기와 호환되어야 합니다. |
| 13 | 복구 제약 | 남은 수리, 손상, 조정, 복구 작업은 닫기 전에 처리되어야 합니다. |
| 14 | 닫기 전이 | [닫기 전이](#core-close-readiness-close-transition)를 참고합니다. |

<a id="core-close-readiness-close-transition"></a>
### 닫기 전이

조건:
- 닫기 차단 사유가 남아 있지 않습니다.

효과:
- API 담당 메서드 동작을 통해 종료 전이가 진행될 수 있습니다.

차단된 경우:
- Task는 열린 상태로 남습니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)

사전 확인 실패:
- 오래된 상태, 잘못된 요청 식별 정보, 평가 전 로컬 접근 실패 같은 API 담당 실패는 의미적 닫기 준비 상태 발견 사항이 아닙니다.
- 이런 경우는 API와 오류 담당 문서가 다룹니다.

## 11. 차단 사유와 면제

### 차단 사유

개념:
- 차단 사유는 진행, 쓰기, Run 기록, 닫기가 정직하게 이어질 수 없는 구조화된 이유입니다.

같은 것이 아님:
- 상태 보기 문구.
- 포괄적 승인.
- 성공처럼 보이는 닫기 결과.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)

### 닫기 차단 사유

개념:
- 닫기 차단 사유는 정직한 닫기 준비 상태를 막는 닫기 관련 이유입니다.

같은 것이 아님:
- `WriteDecisionReason`.
- 그 자체의 저장 효과 증명.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)

### `CloseReadinessBlocker`

개념:
- `CloseReadinessBlocker`는 닫기 차단 사유를 나타내는 API 데이터 표현입니다.

같은 것이 아님:
- 닫기 준비 상태 개념 전체.
- `prepare_write` 판단 사유.
- 그 자체의 지속 저장 증명.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)
- [API 오류](api/errors.md)

### 면제

개념:
- 면제는 담당 문서가 허용할 때 이름 붙은 요구사항 하나에 대한 범위 있는 예외입니다.

허용되는 효과:
- 이름 붙인 요구사항 하나만, 그리고 그 요구사항의 담당 경로가 허용하는 범위에서만 차단을 풀 수 있습니다.

같은 것이 아님:
- 판단 유예.
- 범위 생성, 민감 동작 승인, 필수 증거, 최종 수락, 잔여 위험 표시.
- QA 증거, QA 통과, 검증, 보증 수준 향상.

담당 문서 링크:
- [범위 참조](scope.md)

## 12. 잔여 위험

개념:
- 잔여 위험은 닫기에 의미가 있는 알려진 남은 불확실성, 확인하지 못한 조건, 한계, 절충점입니다.

입력:
- 보이는 이름 붙은 위험.
- 요청한 닫기와 보이는 닫기 근거.
- 관련 증거, 아티팩트, 차단 사유, Run 참조.
- 닫기가 잔여 위험 수락에 의존할 때 호환되는 `residual_risk_acceptance`.

필요한 순서:
- 닫기에 영향을 주는 알려진 잔여 위험은 성공 닫기 전에 보여야 합니다.
- 사용자는 판단할 만큼 보이지 않은 위험을 수락할 수 없습니다.

범위:
- 수락은 요청한 닫기에 대해 이름 붙은 보이는 위험에 적용되며 모든 미지의 사항에 적용되지 않습니다.

같은 것이 아님:
- 검증, 증거 충분성, QA, 민감 동작 승인, 최종 수락, 무위험 결과.
- 면제 또는 자동 성공.

현재 MVP 경로:
- 담당 문서가 더 많은 것을 승격하기 전까지 현재 경로는 간결한 잔여 위험 요약, 차단 사유, 증거 참조, `user_judgment` 참조입니다.
- 풍부한 위험 흐름은 이후 자료입니다.

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)
- [범위 참조](scope.md)

## 13. 관련 담당 문서

| 주제 | 상세 |
|---|---|
| API 메서드와 요청 래퍼 | [API 메서드와 요청 래퍼](#core-owner-api-methods-envelopes)를 참고합니다. |
| 상태 형태 API 데이터 | [상태 형태 API 데이터](#core-owner-state-shaped-api-data)를 참고합니다. |
| 사용자 판단 스키마 | [사용자 판단 스키마](#core-owner-user-judgment-schemas)를 참고합니다. |
| 아티팩트 스키마와 생명주기 | [아티팩트 스키마와 생명주기](#core-owner-artifact-schemas-lifecycle)를 참고합니다. |
| 공개 오류 | [공개 오류](#core-owner-public-errors)를 참고합니다. |
| 저장소 기록과 효과 | [저장소 기록과 효과](#core-owner-storage-records-effects)를 참고합니다. |
| 상태 보기 권한 | [상태 보기 권한](#core-owner-projection-authority)을 참고합니다. |
| 템플릿 본문 | [템플릿 본문](#core-owner-template-bodies)을 참고합니다. |
| 보안 표현 | [보안 표현](#core-owner-security-wording)을 참고합니다. |
| 런타임 경계 | [런타임 경계](#core-owner-runtime-boundaries)를 참고합니다. |
| 설계 품질 | [설계 품질](#core-owner-design-quality)을 참고합니다. |
| 에이전트 통합 | [에이전트 통합](#core-owner-agent-integration)을 참고합니다. |
| 이후 후보 | [이후 후보](#core-owner-out-of-scopes)를 참고합니다. |

<a id="core-owner-api-methods-envelopes"></a>
### API 메서드와 요청 래퍼

적용 대상:
- API 메서드 동작.
- 요청/응답 형태.
- 공통 요청 래퍼.
- `dry_run`/거절 분기.
- 메서드 효과.

담당 문서 링크:
- [API 메서드](api/methods.md)와 그 문서가 나열하는 메서드 담당 문서.
- [API 코어 스키마](api/schema-core.md).

<a id="core-owner-state-shaped-api-data"></a>
### 상태 형태 API 데이터

적용 대상:
- `ShapingReadiness`.
- `CloseReadinessBlocker`.
- `ValidatorResult`.
- 공개 상태 필드.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md).
- [API 값 집합](api/schema-value-sets.md).

<a id="core-owner-user-judgment-schemas"></a>
### 사용자 판단 스키마

적용 대상:
- 사용자 판단 스키마.
- `SensitiveActionScope`.
- 수락된 위험 입력 형태.

담당 문서 링크:
- [API 판단 스키마](api/schema-judgment.md).

<a id="core-owner-artifact-schemas-lifecycle"></a>
### 아티팩트 스키마와 생명주기

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md).
- [아티팩트 저장소](storage-artifacts.md).

<a id="core-owner-public-errors"></a>
### 공개 오류

적용 대상:
- 공개 오류 코드.
- 오류 라우팅.
- 오류 우선순위.

담당 문서 링크:
- [API 오류](api/errors.md).

<a id="core-owner-storage-records-effects"></a>
### 저장소 기록과 효과

담당 문서 링크:
- [저장소 기록](storage-records.md).
- [저장 효과](storage-effects.md).
- [저장소 버전 관리](storage-versioning.md).

<a id="core-owner-projection-authority"></a>
### 상태 보기 권한

적용 대상:
- 상태 보기 권한.
- 읽기 전용 표시 경계.

담당 문서 링크:
- [상태 보기 권한 참조](projection-and-templates.md).

<a id="core-owner-template-bodies"></a>
### 템플릿 본문

적용 대상:
- 상태 카드 본문.
- 판단 요청 본문.
- 실행/증거 요약 본문.
- 닫기 결과 본문.
- 에이전트 맥락 패킷 본문.

담당 문서 링크:
- [템플릿 본문](template-bodies.md).

<a id="core-owner-security-wording"></a>
### 보안 표현

적용 대상:
- 보안 보장 문구.
- 협력형, 탐지형, 예방형 주장.
- 로컬 접근 태세.

담당 문서 링크:
- [보안 참조](security.md).

<a id="core-owner-runtime-boundaries"></a>
### 런타임 경계

적용 대상:
- Product Repository 분리.
- Harness Server 분리.
- Harness Runtime Home 분리.

담당 문서 링크:
- [런타임 경계 참조](runtime-boundaries.md).

<a id="core-owner-design-quality"></a>
### 설계 품질

적용 대상:
- 설계 품질 경계.
- 비관문 라우팅.

담당 문서 링크:
- [설계 품질](design-quality.md).

<a id="core-owner-agent-integration"></a>
### 에이전트 통합

적용 대상:
- 커넥터 동작.
- 접점 역량 태세.

담당 문서 링크:
- [에이전트 통합 참조](agent-integration.md).

<a id="core-owner-out-of-scopes"></a>
### 이후 후보

적용 대상:
- 이후 후보.
- 향후 보증, 면제, QA, 검증, 픽스처 자료.

담당 문서 링크:
- [범위 참조](scope.md).

다른 문서가 정확한 스키마, DDL, 렌더링된 템플릿 문구, 공개 오류 코드, 이후 후보 목록을 필요로 하면 여기서 다시 정의하지 말고 담당 문서로 연결해야 합니다.
