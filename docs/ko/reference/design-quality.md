# 설계 품질

## 1. 담당하는 것 / 담당하지 않는 것

이 참조 문서는 기준 범위의 설계 품질 라우팅 경계를 담당합니다.

역할: 설계 품질 관찰 사항은 아래 항목을 식별할 때 판단, 증거, 범위 담당 문서로 이어집니다.

- 제품 판단
- 기술 판단
- 범위 판단
- 증거 공백
- 잔여 위험 표시 문제
- 이미 활성 Core/API 범주가 담당하는 닫기 차단 사유

이 문서는 독립적인 활성 관문, 설계 품질 전용 활성 닫기 차단 범주, 활성 검증기 계열, 품질 면제 경로, 심각도 기반 차단 정책, 증거 기록, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 권한을 정의하지 않습니다.

이 문서가 담당합니다.

- 기준 범위에서 판단 라우팅, 증거 참조, 범위 참조로 쓰이는 설계 품질 역할
- 설계 품질 관찰 사항을 `judgment_kind=product_decision`, `judgment_kind=technical_decision`, `judgment_kind=scope_decision`으로 보내는 기준
- 설계 품질 관찰 사항을 `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, `surface_capability` 같은 기존 활성 차단 사유 범주로 연결하는 기준
- 활성 설계 품질 심각도 경계. 심각도 형태의 문구는 활성 담당 경로가 별도 행동을 요구하지 않는 한 조언성 우선순위입니다.
- 설계 품질 관찰 사항, 활성 `ValidatorResult.validator_id` 값, 지원 범위 밖 품질 정책 자료 사이의 경계

이 문서는 담당하지 않습니다.

- Core 생명주기, 관문, 차단 사유, `prepare_write`, `close_task`, Write Authorization, 최종 수락, 잔여 위험 수락, 대체 불가능 규칙. [Core 모델 참조](core-model.md)를 봅니다.
- MCP 요청/응답 스키마, `ValidatorResult`, `UserJudgment`, `AcceptedRiskInput`, 공개 오류, 기준 범위/지원 범위 밖 스키마 값. [API 메서드](api/methods.md), 메서드 담당 문서, [API 코어 스키마](api/schema-core.md), [API 판단 스키마](api/schema-judgment.md), [API 오류](api/errors.md)를 봅니다.
- SQLite DDL과 지속 테이블. [저장소 기록](storage-records.md)을 봅니다.
- 검증기 실행 저장 효과. [저장 효과](storage-effects.md)를 봅니다.
- 아티팩트 저장소. [아티팩트 저장소](storage-artifacts.md)를 봅니다.
- 상태 보기 권한. [상태 보기 권한 참조](projection-and-templates.md)를 봅니다.
- 템플릿 본문, 상태 카드, 렌더링된 보고서. [템플릿 본문](template-bodies.md)을 봅니다.
- 지원 범위 밖 설계 품질 정책 체계, 넓은 검토 절차, 운영/보고 자료, 적합성 목록.

설계 품질 발견 사항이 다른 계약과 만날 때는 다음 담당 문서를 사용합니다.

| 질문 | 담당 문서 |
|---|---|
| Core 대체 금지, 닫기 준비 상태, 면제, 수락된 위험, 잔여 위험의 의미 | [Core 모델 참조](core-model.md) |
| `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput` 구조 | [API 판단 스키마](api/schema-judgment.md) |
| 사용자 판단 메서드 동작 | [사용자 판단 메서드](api/method-user-judgment.md) |
| 상태 메서드 동작 | [상태 메서드](api/method-status.md) |
| 닫기 메서드 동작 | [Task 닫기 메서드](api/method-close-task.md) |
| 활성 API 메서드 분기의 메서드별 저장 효과 | [저장 효과](storage-effects.md) |
| 지원 범위 밖 설계 품질 정책 계열 | [범위 참조](scope.md) |

이 저장소의 문서는 계획용 자료입니다. 지금 이 저장소에 하네스 서버, 런타임 상태, 생성된 증거, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 기록이 있다는 뜻이 아닙니다.

## 2. 기준 범위 설계 품질 역할

기준 범위에서 설계 품질은 판단 라우팅을 하고 증거와 범위를 참조하는 좁은 층입니다. 품질 관련 우려를 이해할 수 있게 만들고, 그 우려를 기존 활성 담당 경로로 보냅니다.

설계 품질 발견 사항이 기준 범위에서 할 수 있는 일은 아래뿐입니다.

| 발견 유형 | 상세 |
|---|---|
| 제품 판단 필요 | [제품 판단 필요](#design-quality-product-decision-needed) 참조 |
| 기술 판단 필요 | [기술 판단 필요](#design-quality-technical-decision-needed) 참조 |
| 범위 경계 변경 | [범위 경계 변경](#design-quality-scope-boundary-change) 참조 |
| 닫기 관련 뒷받침 부족 | [닫기 관련 뒷받침 부족](#design-quality-missing-close-relevant-support) 참조 |
| 잔여 위험 표시 | [잔여 위험 표시](#design-quality-residual-risk-visibility) 참조 |
| 접점 역량 공백 | [접점 역량 공백](#design-quality-surface-capability-gap) 참조 |
| 조언성 심각도 | [조언성 심각도](#design-quality-advisory-severity) 참조 |
| 집중된 다음 행동 | [집중된 다음 행동](#design-quality-focused-next-action) 참조 |
| 활성 담당 경로 없음 | [활성 담당 경로 없음](#design-quality-no-active-owner-path) 참조 |

<a id="design-quality-product-decision-needed"></a>
### 제품 판단 필요

조건:
- 우려가 사용자 판단이 필요한 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치 선택입니다.

라우팅:
- `judgment_kind=product_decision`을 사용합니다.

닫기 영향:
- 활성 닫기 경로가 이미 `CloseReadinessBlocker.category=user_judgment`를 요구할 때만 닫기를 차단합니다.

허용되지 않는 것:
- 설계 품질 라벨 자체를 독립적인 닫기 차단 사유로 취급하지 않습니다.

<a id="design-quality-technical-decision-needed"></a>
### 기술 판단 필요

조건:
- 우려가 사용자 판단이 필요한 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향 선택입니다.

라우팅:
- `judgment_kind=technical_decision`을 사용합니다.

닫기 영향:
- 활성 닫기 경로가 이미 `CloseReadinessBlocker.category=user_judgment`를 요구할 때만 닫기를 차단합니다.

허용되지 않는 것:
- 설계 품질 라벨 자체를 독립적인 닫기 차단 사유로 취급하지 않습니다.

<a id="design-quality-scope-boundary-change"></a>
### 범위 경계 변경

조건:
- 우려가 범위 확장, 비목표 제거, Change Unit 경계 변경, Autonomy Boundary 변경입니다.

라우팅:
- 담당 경로에 따라 `judgment_kind=scope_decision` 또는 `CloseReadinessBlocker.category=scope`를 사용합니다.

닫기 영향:
- 활성 범위 또는 사용자 판단 담당 경로를 통해서만 닫기를 차단합니다.

허용되지 않는 것:
- 설계 품질 라벨을 범위 판단을 우회하는 근거로 취급하지 않습니다.

<a id="design-quality-missing-close-relevant-support"></a>
### 닫기 관련 뒷받침 부족

조건:
- 닫기와 관련된 주장을 뒷받침하는 자료가 부족합니다.

라우팅:
- Core 증거 담당 경로에서 증거를 요청합니다.
- `CloseReadinessBlocker.category=evidence` 또는 `CloseReadinessBlocker.category=artifact_availability`는 그 담당 경로를 통해서만 사용합니다.

닫기 영향:
- 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다.

허용되지 않는 것:
- 그 담당 경로 밖에서 설계 품질 전용 증거 규칙을 만들지 않습니다.

<a id="design-quality-residual-risk-visibility"></a>
### 잔여 위험 표시

조건:
- 알려진 한계, 확인하지 못한 조건, 절충점이 닫기에 중요합니다.

라우팅:
- 잔여 위험 표시를 사용합니다.
- 활성 닫기 경로가 수락을 요구할 때만 `CloseReadinessBlocker.category=residual_risk_acceptance`를 사용합니다.

닫기 영향:
- 잔여 위험 표시 또는 잔여 위험 수락 담당 경로를 통해서만 닫기에 영향을 줍니다.

허용되지 않는 것:
- 수락된 위험을 성공 증명이나 위험 제거로 취급하지 않습니다.

<a id="design-quality-surface-capability-gap"></a>
### 접점 역량 공백

조건:
- 연결된 접점이 주장한 동작이나 보장을 정직하게 지원하지 못합니다.

라우팅:
- 역량 담당 경로에서 `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다.

닫기 영향:
- 역량 담당 경로를 통해서만 닫기에 영향을 줍니다.

허용되지 않는 것:
- 설계 품질 라벨로 보장 수준을 높이지 않습니다.

<a id="design-quality-advisory-severity"></a>
### 조언성 심각도

조건:
- 발견 사항이 우려의 상대적 긴급도나 주의 수준을 설명합니다.

라우팅:
- 활성 담당 경로가 별도 행동을 요구하지 않는 한 심각도 형태의 문구는 조언성 우선순위로 다룹니다.

닫기 영향:
- 심각도 형태의 문구 자체에는 닫기 영향이 없습니다.

허용되지 않는 것:
- 심각도만으로 차단 사유, 검증기 매핑, 면제, 증거 기대치, 닫기 결과를 만들지 않습니다.

<a id="design-quality-focused-next-action"></a>
### 집중된 다음 행동

조건:
- 좁은 행동 하나가 이름 붙은 담당 경로를 풀거나 분명히 할 수 있습니다.

라우팅:
- 집중된 사용자 판단 하나 묻기, 증거 요청, 잔여 위험 표시, 조언성 다음 행동 표시, 아무 행동 없음 중 하나를 사용합니다.

닫기 영향:
- 이름 붙은 담당 경로가 그 행동을 사용할 때만 닫기에 영향을 줄 수 있습니다.

허용되지 않는 것:
- 다음 행동을 이름 붙은 담당 경로가 요구하는 범위보다 넓히지 않습니다.

<a id="design-quality-no-active-owner-path"></a>
### 활성 담당 경로 없음

조건:
- 적용되는 활성 담당 경로가 없습니다.

닫기 영향:
- 기준 범위 결과는 조언 문구이거나 아무 행동 없음입니다.

허용되지 않는 것:
- 새 관문, 차단 사유, 검증기 매핑, 면제 경로, 증거 규칙, 닫기 권한을 만들지 않습니다.

활성 설계 품질은 아래 항목을 만들지 않습니다.

- 새 Core 상태나 스키마
- 새 `ValidatorResult` 필드
- 활성 정책 검증기
- 품질 면제 경로
- 별도 설계 검토 권한

설계 품질은 평범한 작업을 끝없는 계획 반복으로 만들면 안 됩니다.

다른 활성 담당 경로가 좁은 일부를 명시적으로 요구하지 않는 한, 아래 항목은 기준 범위 차단 사유가 아닙니다.

- 전체 도메인 언어 점검
- 전체 모듈/인터페이스 검토
- 전체 TDD 추적
- 전체 피드백 루프 감사
- 전체 `codebase_stewardship` 검토
- 넓은 검토 목록
- 지원 범위 밖 품질 절차

## 3. 라우팅 규칙

설계 품질 관찰 사항은 활성 담당 경로를 통해서만 기준 범위 상태에 영향을 줍니다. 관찰 사항은 자신이 의존하는 활성 경로를 이름 붙여야 합니다.

| 우려 | 상세 |
|---|---|
| 제품 방향 미결정 | [제품 방향 미결정](#design-quality-route-product-direction) 참조 |
| 기술 방향 미결정 | [기술 방향 미결정](#design-quality-route-technical-direction) 참조 |
| 범위 경계 변경 필요 | [범위 경계 라우팅](#design-quality-route-scope-boundary) 참조 |
| 닫기 관련 뒷받침 부족 | [증거 라우팅](#design-quality-route-evidence) 참조 |
| 닫기에 중요한 잔여 위험 | [잔여 위험 라우팅](#design-quality-route-residual-risk) 참조 |
| 접점 역량이 주장을 뒷받침하지 못함 | [접점 역량 라우팅](#design-quality-route-surface-capability) 참조 |

<a id="design-quality-route-product-direction"></a>
### 제품 방향 미결정

조건:
- 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치가 정해지지 않았습니다.

라우팅:
- `judgment_kind=product_decision`을 사용합니다.
- 활성 닫기 경로가 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다.

닫기 영향:
- 활성 담당 경로가 해당 사용자 판단을 요구할 때만 닫기를 차단합니다.

<a id="design-quality-route-technical-direction"></a>
### 기술 방향 미결정

조건:
- 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향이 정해지지 않았습니다.

라우팅:
- `judgment_kind=technical_decision`을 사용합니다.
- 활성 닫기 경로가 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다.

닫기 영향:
- 활성 담당 경로가 해당 사용자 판단을 요구할 때만 닫기를 차단합니다.

<a id="design-quality-route-scope-boundary"></a>
### 범위 경계 라우팅

조건:
- 범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary 변경이 필요합니다.

라우팅:
- 담당 경로에 따라 `judgment_kind=scope_decision` 또는 `CloseReadinessBlocker.category=scope`를 사용합니다.

닫기 영향:
- 활성 범위 또는 사용자 판단 담당 경로를 통해서만 닫기를 차단합니다.

<a id="design-quality-route-evidence"></a>
### 증거 라우팅

조건:
- 닫기와 관련된 주장을 뒷받침하는 자료가 부족합니다.

라우팅:
- Core 증거 담당 경로에서 `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`, 또는 증거 요청을 사용합니다.

닫기 영향:
- 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다.

<a id="design-quality-route-residual-risk"></a>
### 잔여 위험 라우팅

조건:
- 알려진 한계나 확인하지 못한 조건이 닫기에 중요합니다.

라우팅:
- `CloseReadinessBlocker.category=residual_risk_visibility`로 잔여 위험을 보이게 합니다.
- 활성 닫기 경로가 수락을 요구할 때만 `CloseReadinessBlocker.category=residual_risk_acceptance`를 사용합니다.

닫기 영향:
- 활성 잔여 위험 담당 경로를 통해서만 닫기에 영향을 줍니다.

<a id="design-quality-route-surface-capability"></a>
### 접점 역량 라우팅

조건:
- 연결된 접점이 주장한 동작이나 보장을 정직하게 지원하지 못합니다.

라우팅:
- 역량 담당 경로에서 `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다.

닫기 영향:
- 활성 역량 담당 경로를 통해서만 닫기에 영향을 줍니다.

설계 품질 라벨, 정책 이름, 심각도 값, 검증기 ID, 검토 문구는 그 자체로 경로를 만들지 않습니다. 적용되는 활성 담당 경로가 없으면 기준 범위 결과는 조언 문구이거나 아무 행동 없음입니다.

<a id="when-a-finding-blocks-close"></a>
## 4. 닫기 차단 사유가 되는 조건

설계 품질 관찰 사항은 활성 담당 경로를 통해서만 닫기를 차단합니다.

| 닫기 차단 질문 | 상세 |
|---|---|
| 활성 닫기 의존성 | [활성 닫기 의존성](#design-quality-close-active-dependency) 참조 |
| 집중된 차단 해소 경로 | [집중된 차단 해소 경로](#design-quality-close-focused-unblock-path) 참조 |
| 지원되지 않는 정책 근거 | [지원되지 않는 정책 근거](#design-quality-close-unsupported-policy-basis) 참조 |
| 조언에 그치는 정책 문구 | [조언에 그치는 정책 문구](#design-quality-close-advisory-only-policy-phrase) 참조 |
| 활성 닫기 차단 범주 | [활성 닫기 차단 범주](#design-quality-close-active-category) 참조 |

<a id="design-quality-close-active-dependency"></a>
### 활성 닫기 의존성

조건:
- 관찰 사항이 활성 Task 또는 Change Unit과 시도 중인 닫기에 연결되어 있습니다.
- 관찰 사항이 활성 닫기 차단 집합 안의 기존 활성 `CloseReadinessBlocker.category`, `judgment_kind`, API 오류, 담당 경로를 이름 붙입니다.

닫기 영향:
- 이름 붙인 담당 경로가 설계 품질 라벨 없이도 닫기를 차단할 때만 닫기를 차단할 수 있습니다.

허용되지 않는 것:
- 설계 품질 라벨을 독립적인 닫기 권한으로 취급하지 않습니다.

<a id="design-quality-close-focused-unblock-path"></a>
### 집중된 차단 해소 경로

조건:
- 이름 붙은 담당 경로 하나에서 차단 사유를 해소하거나, 그 담당 경로로 유예하거나, 필요한 증거로 뒷받침하거나, 보이는 잔여 위험으로 표시할 수 있습니다.

닫기 영향:
- 닫기에 영향을 주려면 그 담당 경로를 위한 다음 행동을 정확히 하나만 제공해야 합니다.

허용되지 않는 것:
- 다음 행동을 넓은 설계 검토나 끝없는 계획 반복으로 넓히지 않습니다.

<a id="design-quality-close-unsupported-policy-basis"></a>
### 지원되지 않는 정책 근거

조건:
- 관찰 사항이 지원되지 않는 품질 정책 경로, 넓은 정책 목록, 심각도 값만을 근거로 삼습니다.

닫기 영향:
- 그 근거만으로는 닫기를 차단하지 않습니다.

허용되지 않는 것:
- 지원 범위 밖 품질 정책 자료를 활성 관문, 닫기 차단 사유, 면제 경로, 증거 규칙, 닫기 권한으로 취급하지 않습니다.

<a id="design-quality-close-advisory-only-policy-phrase"></a>
### 조언에 그치는 정책 문구

조건:
- 발견 사항이 도메인 언어, 세로 조각 형태, TDD, 모듈/인터페이스 검토, 관리 책임, 넓은 검토 단계, 기준 범위 밖 정책 계열을 언급합니다.

라우팅:
- 활성 담당 경로가 그 좁은 행동을 필요로 할 때만 조언성 다음 행동, 증거 요청, 집중된 사용자 판단, 잔여 위험 표시를 사용합니다.

닫기 영향:
- 위 주제를 언급했다는 이유만으로는 닫기를 차단하지 않습니다.

허용되지 않는 것:
- 지원 범위 밖 정책 계열을 기준 범위 요구사항처럼 제시하지 않습니다.

<a id="design-quality-close-active-category"></a>
### 활성 닫기 차단 범주

조건:
- 설계 품질 관찰 사항이 닫기에 영향을 줍니다.

라우팅:
- [API 값 집합](api/schema-value-sets.md)이 담당하는 활성 `CloseReadinessBlocker.category` 값을 사용합니다.

닫기 영향:
- 닫기 준비 상태 평가의 닫기 차단 사유는 그 닫기 경로가 담당하는 활성 범주 안에 남습니다.

허용되지 않는 것:
- 기준 범위에서 설계 품질 전용 닫기 차단 범주를 만들지 않습니다.

## 5. 별도 품질 면제 없음

기준 범위에는 별도의 활성 품질 면제 경로가 없습니다. 어떤 요구사항을 유예하거나, 위험으로 수락하거나, 사용자 판단으로 해결할 수 있는지는 해당 활성 담당 경로가 정합니다. 그 경로의 정확한 `judgment_kind`, 차단 사유 범주, 증거 동작을 사용해야 합니다.

면제에 가까운 결정이나 수락된 위험 답변은 이름 붙은 요구사항 또는 이름 붙은 보이는 위험에 대한 책임 있는 사용자 판단을 기록합니다.

그 판단은 아래 항목을 하지 않습니다.

- 사실 지우기
- 닫기 근거에서 남은 한계 제거
- 증거 만들기
- 검증 증명
- QA 통과
- 최종 수락 대체
- 닫기 자동 성공

판단 경로는 계속 서로 구분합니다.

| 경로 | 상세 |
|---|---|
| `final_acceptance` | [`final_acceptance`](#design-quality-route-final-acceptance) 참조 |
| `residual_risk_acceptance` | [`residual_risk_acceptance`](#design-quality-route-residual-risk-acceptance) 참조 |
| 활성 `UserJudgment.judgment_kind` 값 | [활성 사용자 판단 값](#design-quality-route-active-user-judgment-values) 참조 |

<a id="design-quality-route-final-acceptance"></a>
### `final_acceptance`

조건:
- 닫기 근거가 보이고 활성 담당 경로가 사용자의 결과 판단을 요청합니다.

기록 효과:
- 닫기 근거가 보인 뒤 사용자가 결과를 판단한 내용을 기록합니다.

닫기 영향:
- 그 자체로 닫기 차단 사유를 우회하지 않습니다.

허용되지 않는 것:
- 증거 생성, 잔여 위험 수락, QA, 검증, 차단 사유 우회로 취급하지 않습니다.

<a id="design-quality-route-residual-risk-acceptance"></a>
### `residual_risk_acceptance`

조건:
- 요청한 닫기에 이름 붙은 보이는 잔여 위험이 남아 있습니다.

기록 효과:
- 요청한 닫기를 위해 이름 붙은 보이는 잔여 위험을 사용자가 수락한 내용을 기록합니다.

닫기 영향:
- 활성 잔여 위험 담당 경로를 통해서만 닫기에 영향을 줍니다.

허용되지 않는 것:
- 정확성 증명, 증거 충분성, 최종 수락, 무위험 결과, 자동 성공으로 취급하지 않습니다.

<a id="design-quality-route-active-user-judgment-values"></a>
### 활성 사용자 판단 값

조건:
- 집중된 사용자 소유 판단이 필요합니다.

기록 효과:
- 집중된 사용자 소유 결정을 기록합니다.

담당 문서 링크:
- 값은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.

닫기 영향:
- 판단을 요청한 활성 담당 경로를 통해서만 닫기에 영향을 줄 수 있습니다.

허용되지 않는 것:
- 포괄 승인, 별도 품질 면제, 지원되지 않는 판단 범주로 취급하지 않습니다.

넓은 승인, "좋아 보인다" 같은 말, 일반적인 진행 승인은 활성 담당 경로가 그 특정 판단을 물은 경우가 아니라면 위 판단으로 취급하면 안 됩니다.

## 6. 증거 기대치

설계 품질 관찰 사항은 증거 공백을 식별할 수 있지만, 필요한 증거는 Core 증거 담당 경로에 속합니다.

| 증거 질문 | 상세 |
|---|---|
| 요청할 수 있는 증거 공백 | [요청할 수 있는 증거 공백](#design-quality-evidence-gap-request) 참조 |
| 유용한 증거 참조 | [유용한 증거 참조](#design-quality-useful-evidence-references) 참조 |
| 증거를 자동 충족하지 않는 참조 | [증거를 자동 충족하지 않는 참조](#design-quality-evidence-non-satisfying-references) 참조 |
| 필수가 아닌 증거 공백 | [필수가 아닌 증거 공백](#design-quality-non-required-evidence-gaps) 참조 |

<a id="design-quality-evidence-gap-request"></a>
### 요청할 수 있는 증거 공백

조건:
- 활성 담당 경로가 쓰기 안전성, 닫기 준비 상태, 사용자 판단, 잔여 위험, 정직한 보장 표시에 영향을 주는 주장을 뒷받침해야 합니다.

라우팅:
- Core 증거 담당 경로를 통해 증거를 요청합니다.

닫기 영향:
- 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다.

<a id="design-quality-useful-evidence-references"></a>
### 유용한 증거 참조

허용되는 예:

- 지속 `ArtifactRef` 값, Run 참조, 명령/확인 요약, 출처 참조
- 최신이 아닌 맥락이 닫기 근거에 영향을 줄 때 현재 상태/버전/최신성 참조
- 제품, 기술, 범위, 최종 수락, 잔여 위험 판단에 대한 사용자 판단 참조
- 알려진 한계가 닫기에서 보일 때 잔여 위험 참조

<a id="design-quality-evidence-non-satisfying-references"></a>
### 증거를 자동 충족하지 않는 참조

허용되지 않는 것:
- 채팅 주장, 일반 요약, 렌더링된 상태 보기 문장, 등록되지 않은 파일, 담당 경로 없는 화면 캡처, 테스트 통과만 있는 상태, 최종 수락, 잔여 위험 수락을 필요한 증거로 자동 취급하지 않습니다.

닫기 영향:
- 이런 참조만으로 필요한 증거 차단 사유가 해소되지 않습니다.

<a id="design-quality-non-required-evidence-gaps"></a>
### 필수가 아닌 증거 공백

조건:
- 증거 공백이 Core 증거 담당 경로에서 요구한 필수 증거가 아닙니다.

라우팅:
- 상황에 맞게 `request evidence`, `show advisory next action`, 또는 잔여 위험 표시를 사용합니다.

닫기 영향:
- 이 공백은 필요한 증거로서 닫기를 차단하지 않습니다.

## 7. 검증기 ID 경계

검증기 ID는 보고용 라벨입니다. Core 불변조건, 관문, 닫기 차단 사유, 면제, 증거 기록, 사용자 판단을 만들지 않습니다.

`ValidatorResult` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다. `severity` 형태 값과 활성 안정 `ValidatorResult.validator_id` 집합은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.

이 문서는 아래 항목을 제공하지 않습니다.

- 활성 설계 정책 검증기 ID
- 정책-검증기 매핑

[범위 참조](scope.md)와 영향받는 담당 문서가 좁은 활성 계약을 승격하지 않는 한, 활성 집합 밖의 검증기 ID는 기준 범위 효과가 없습니다.

## 8. 지원 범위 밖 정책 자료 경계

전체 설계 품질 정책 목록은 기준 범위가 아닙니다.

이 문서는 지원되지 않는 관문 이름, 차단 사유 범주, 면제 분기, 검증기 계열, 작업 흐름 분기, 승격 체크리스트를 공개하지 않습니다. 기준 범위에서 제외되는 기능 묶음은 [범위 참조](scope.md)에서 범주 수준으로 확인합니다.

지원 범위 밖 품질 자료를 기준 범위 요구사항, 차단 사유, 면제 규칙, 증거 기대치, 검증기 매핑, 픽스처 요구사항, 운영 보고, 구현 작업처럼 제시하면 안 됩니다.
