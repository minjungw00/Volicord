# 설계 품질

<a id="1-owns--does-not-own"></a>
## 1. 담당 경계

이 참조 문서는 기준 범위의 설계 품질 담당 경계를 정의합니다. 설계 품질 발견
사항은 우려를 분명하게 드러내고 기존 판단, 증거, 범위, 잔여 위험, 연결 역량,
닫기 준비 상태 담당 문서로 보냅니다. 별도 권한을 만들지는 않습니다.

이 문서가 담당합니다.

- 기준 범위에서 설계 품질 발견 사항이 맡는 역할
- 발견 사항을 지원되는 판단 종류, 차단 사유 범주, 증거 또는 범위 담당 문서로
  보내는 경로
- 심각도 형태 문구의 조언성 경계
- 발견 사항, 지원되는 `ValidatorResult.validator_id`, 지원 범위 밖 품질 정책
  사이의 경계

인접 계약은 각 담당 문서에 남습니다.

| 질문 | 담당 문서 |
|---|---|
| Core 대체 금지, 닫기 준비 상태, 면제, 수락된 위험, 잔여 위험의 의미 | [Core 모델 참조](core-model.md) |
| 판단 형태와 값 | [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md) |
| 차단 사유 형태와 범주 값 | [API 상태 스키마](api/schema-state.md), [API 값 집합](api/schema-value-sets.md) |
| 사용자 소유 판단 요청과 기록 동작 | [사용자 소유 판단 요청 메서드](api/method-request-user-judgment.md), [사용자 소유 판단 기록 메서드](api/method-record-user-judgment.md) |
| 상태와 닫기 동작 | [상태 메서드](api/method-status.md), [Task 닫기 메서드](api/method-close-task.md) |
| Agent Connection 역량과 공개 역량 오류 | [Agent Connection](agent-connection.md), [API 오류 코드](api/error-codes.md) |
| 메서드별 저장 효과 | [저장 효과](storage-effects.md) |
| 지원 범위 밖 설계 품질 정책 계열 | [범위 참조](scope.md) |

이 문서는 제품 수락, 최종 수락, 잔여 위험 수락, 닫기 권한, 독립 품질 관문,
품질 면제 경로, 심각도 기반 차단 정책, API 동작, 저장 효과, 스키마 필드,
검증기 계열, 증거 권한, QA 결과, 적합성 목록, 상태 보기, 보고서, 템플릿 본문을
정의하지 않습니다.

발견 사항은 Volicord 상태, 사용자 소유 판단, 쓰기 티켓, 민감 동작 승인, 증거,
최종 수락, 잔여 위험 수락, 닫기 준비 상태도 만들지 않습니다.

<a id="2-baseline-design-quality-role"></a>
<a id="3-routing-rules"></a>
## 2. 발견 사항 처리

발견 사항은 관련 담당 문서가 그 효과를 정의할 때만 기준 범위의 제품 효과를
가집니다. 아래에서 적용되는 가장 좁은 경로를 사용합니다.

| 우려 | 담당 문서가 정의한 처리 경로 | 닫기 영향 |
|---|---|---|
| <a id="design-quality-product-decision-needed"></a><a id="design-quality-route-product-direction"></a>제품 동작, UX, 문구, 릴리스 약속, 사용자 가치에 판단이 필요합니다. | `judgment_kind=product_decision`을 사용합니다. | 적용되는 닫기 준비 상태 계약이 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다. |
| <a id="design-quality-technical-decision-needed"></a><a id="design-quality-route-technical-direction"></a>아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안·개인정보 또는 중요한 기술 방향에 판단이 필요합니다. | `judgment_kind=technical_decision`을 사용합니다. | 적용되는 닫기 준비 상태 계약이 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다. |
| <a id="design-quality-scope-boundary-change"></a><a id="design-quality-route-scope-boundary"></a>범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary를 바꿔야 합니다. | 영향을 받는 범위 또는 판단 계약에 따라 `judgment_kind=scope_decision`이나 `CloseReadinessBlocker.category=scope`를 사용합니다. | 해당 계약이 의존성을 정의할 때만 닫기에 영향을 줍니다. |
| <a id="design-quality-missing-close-relevant-support"></a><a id="design-quality-route-evidence"></a>닫기 관련 주장에 필요한 뒷받침이 없습니다. | Core 증거 권한에 따라 증거를 요청합니다. 증거와 닫기 준비 상태 담당 문서가 허용할 때만 `CloseReadinessBlocker.category=evidence_claim`, `CloseReadinessBlocker.category=evidence_provenance`, `CloseReadinessBlocker.category=artifact_availability`를 사용합니다. | 담당 문서가 요구할 때만 증거 부족으로 닫기를 차단합니다. |
| <a id="design-quality-residual-risk-visibility"></a><a id="design-quality-route-residual-risk"></a>알려진 한계, 확인하지 못한 조건, 절충점이 닫기에 중요합니다. | 위험을 보이게 합니다. 적용되는 담당 문서에 따라 `CloseReadinessBlocker.category=residual_risk_visibility`를 사용하고, 수락이 필요하면 `CloseReadinessBlocker.category=residual_risk_acceptance`를 사용합니다. | 적용되는 잔여 위험 계약을 통해서만 닫기에 영향을 줍니다. |
| <a id="design-quality-connection-capability-gap"></a><a id="design-quality-route-connection-capability"></a>Agent Connection이 주장한 동작이나 보장을 지원할 수 없습니다. | 연결 역량과 API 오류 담당 문서에 따라 `CloseReadinessBlocker.category=connection_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다. | 해당 담당 문서가 효과를 정의할 때만 닫기에 영향을 줍니다. |
| <a id="design-quality-advisory-severity"></a>발견 사항이 상대적 긴급도를 설명합니다. | 담당 문서가 별도 행동을 요구하지 않으면 심각도 형태 문구를 조언성 우선순위로 다룹니다. | 심각도 자체에는 닫기 영향이 없습니다. |
| <a id="design-quality-focused-next-action"></a>좁은 행동 하나로 담당 요구사항을 해결하거나 분명히 할 수 있습니다. | 집중된 사용자 판단 하나를 묻거나, 증거를 요청하거나, 잔여 위험을 보이거나, 다음 행동을 조언하거나, 별도 행동을 하지 않습니다. | 담당 문서가 닫기 관련 행동으로 정할 때만 영향을 줍니다. |
| <a id="design-quality-no-applicable-owner-path"></a>필요한 담당 문서가 없거나, 불분명하거나, 제품 효과를 정의하기에 너무 넓습니다. | 공백을 밝히고 가장 가까운 담당 문서로 연결합니다. 설계 품질 문구로 공백을 메우지 않습니다. | 조언 문구로 남거나 별도 행동이 없습니다. 공백 자체는 닫기를 차단하지 않습니다. |

모든 경로에 아래 규칙을 적용합니다.

- 발견 사항은 별도 닫기 차단 사유, 수락 관문, 범위 재정의, 증거 규칙, 보장이
  되지 않습니다.
- 발견 사항은 사용자 소유 판단, 쓰기 티켓, 민감 동작 승인, 증거, 최종 수락,
  잔여 위험 수락을 대신하지 않습니다.
- 다음 행동은 적용되는 담당 계약의 범위 안에 둡니다. 문서 안내 편의를 위해
  범위를 넓히면 안 됩니다.
- 정책 라벨, 심각도 값, 검증기 ID, 검토 문구만으로 처리 경로가 생기지 않습니다.
- 설계 품질 검토가 평범한 작업을 끝없는 계획 반복으로 만들면 안 됩니다.

<a id="when-a-finding-blocks-close"></a>
<a id="4-close-dependency-boundary"></a>
## 3. 닫기 의존성 경계

설계 품질에는 별도 차단 장치가 없습니다. 발견 사항은 닫기 준비 상태, 범위,
판단, 증거, 연결 역량, 메서드 담당 문서가 정의한 의존성을 통해서만 닫기에
영향을 줍니다.

- <a id="design-quality-close-applicable-dependency"></a>**적용되는 의존성:**
  발견 사항이 현재 `Task` 또는 Change Unit과 연결되고, 지원되는 차단 사유 범주,
  판단 종류, API 오류 또는 다른 닫기 의존성을 이름 붙입니다. 그 의존성만 닫기를
  차단할 수 있습니다.
- <a id="design-quality-close-focused-unblock-path"></a>**집중된 차단 해소
  경로:** 관련 담당 문서가 요구하는 다음 행동 하나를 보여 줍니다. 이 행동은
  요구사항을 해결하거나, 담당 문서가 허용한 유예를 기록하거나, 필요한 증거를
  제공하거나, 잔여 위험을 보이게 할 수 있습니다.
- <a id="design-quality-close-unsupported-policy-basis"></a>**지원되지 않는 정책
  근거:** 지원 범위 밖 정책이나 심각도만으로는 닫기를 차단하지 않습니다.
- <a id="design-quality-close-advisory-only-policy-phrase"></a>**조언에 그치는 정책
  문구:** 지원 범위 밖 품질 정책 계열을 이름만 언급해도 닫기 영향은 생기지
  않습니다.
- <a id="design-quality-close-supported-category"></a>**지원되는 범주:** 발견
  사항이 닫기에 영향을 준다면 [API 값 집합](api/schema-value-sets.md)이 담당하는
  지원 `CloseReadinessBlocker.category` 값을 사용합니다.

<a id="5-no-separate-quality-waiver"></a>
## 4. 별도 품질 면제 없음

기준 범위에는 품질 면제 경로가 없습니다. 담당 문서가 요구사항의 유예, 위험
수락, 사용자 판단 해결을 허용하면 그 담당 문서의 정확한 판단 종류, 차단 사유
범주, 증거 동작을 사용합니다.

면제에 가까운 결정은 사실을 지우거나, 닫기 근거의 한계를 제거하거나, 증거를
만들거나, 검증을 증명하거나, QA를 통과시키거나, 최종 수락을 대신하거나,
닫기를 자동으로 성공시키지 않습니다.

| 경로 | 의미와 경계 |
|---|---|
| <a id="design-quality-route-final-acceptance"></a>`final_acceptance` | 닫기 근거가 보인 뒤 사용자의 결과 판단을 기록합니다. 증거, 잔여 위험 수락, QA, 검증, 차단 사유 우회가 아닙니다. |
| <a id="design-quality-route-residual-risk-acceptance"></a>`residual_risk_acceptance` | 요청한 닫기에 대해 이름 붙은 보이는 위험 하나의 수락을 기록합니다. 잔여 위험 담당 문서를 통해서만 닫기에 영향을 주며 정확성 증명, 증거 충분성, 최종 수락, 무위험 결과가 아닙니다. |
| <a id="design-quality-route-supported-user-judgment-values"></a>지원되는 `UserJudgment.judgment_kind` 값 | 집중된 사용자 소유 판단을 기록합니다. 값은 [API 값 집합](api/schema-value-sets.md)이 담당합니다. 관련 계약이 특정 질문을 했을 때만 포괄적 승인을 그 판단으로 볼 수 있습니다. |

<a id="6-evidence-routing-boundary"></a>
## 5. 증거 경계

발견 사항은 증거 공백을 식별할 수 있지만 증거 요구사항을 만들지는 않습니다.

| 질문 | 경계 |
|---|---|
| <a id="design-quality-evidence-gap-request"></a>언제 증거를 요청할 수 있습니까? | 적용되는 담당 문서가 쓰기 안전성, 닫기 준비 상태, 사용자 판단, 잔여 위험, 정직한 보장 표시에 영향을 주는 주장에 뒷받침을 요구할 때입니다. Core 증거 권한을 통해 요청합니다. |
| <a id="design-quality-useful-evidence-references"></a>어떤 참조가 유용할 수 있습니까? | 담당 문서가 관련성을 부여한 영속 `ArtifactRef`, 실행 참조, 명령 또는 확인 요약, 출처 참조, 현재 상태·버전·최신성 참조, 사용자 판단 참조, 잔여 위험 참조입니다. |
| <a id="design-quality-evidence-non-satisfying-references"></a>무엇이 증거를 자동으로 충족하지 않습니까? | 채팅 주장, 일반 요약, 렌더링된 상태 보기 문구, 등록되지 않은 파일, 기록된 담당 관계가 없는 화면 캡처, 테스트 통과 상태 자체, 최종 수락, 잔여 위험 수락입니다. |
| <a id="design-quality-non-required-evidence-gaps"></a>필수 증거가 아니라면 어떻게 합니까? | 다음 행동을 조언하거나, 선택적 뒷받침을 요청하거나, 필요에 따라 잔여 위험을 보이게 합니다. 이 공백은 필수 증거로서 닫기를 차단하지 않습니다. |

<a id="7-validator-id-boundary"></a>
## 6. 검증기 ID 경계

검증기 ID는 보고용 라벨입니다. Core 불변조건, 제품 관문, 닫기 차단 사유,
면제, 증거 기록, 사용자 판단, 쓰기 티켓, 최종 수락, 잔여 위험 수락을 만들지
않습니다.

`ValidatorResult` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다.
심각도 형태 값과 지원되는 안정 `ValidatorResult.validator_id` 값의 경계는
[API 값 집합](api/schema-value-sets.md)이 담당합니다. 이 문서는 설계 정책 검증기
ID나 정책-검증기 매핑을 공개하지 않습니다.

그 밖의 검증기 ID는 [범위](scope.md)와 관련 담당 문서가 좁은 지원 계약을
정의하지 않는 한 기준 범위 효과가 없습니다.

<a id="8-out-of-scope-policy-material"></a>
## 7. 지원 범위 밖 정책 자료

이 담당 경계 밖의 설계 품질 정책은 기준 범위에 포함되지 않습니다. 이 문서는
지원되지 않는 관문 이름, 차단 사유 범주, 면제 분기, 검증기 계열, 작업 흐름
분기, 승격 체크리스트를 공개하지 않습니다.

지원 범위 밖 품질 자료를 기준 범위 요구사항, 차단 사유, 면제 규칙, 증거
요구사항, 검증 기준, 검증기 매핑, 적합성 시나리오, 운영 보고, 구현 작업으로
제시하면 안 됩니다. 범주 수준의 제외 항목은 [범위](scope.md)를 사용합니다.
