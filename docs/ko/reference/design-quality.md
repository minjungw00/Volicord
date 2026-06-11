# 설계 품질

## 1. 담당하는 것 / 담당하지 않는 것

이 참조 문서는 현재 MVP의 설계 품질 라우팅 경계를 판단 라우팅, 증거 참조, 범위 참조로 담당합니다. 설계 품질에서 보인 관찰 사항이 제품 판단, 기술 판단, 범위 판단, 증거 공백, 잔여 위험 표시 문제, 또는 이미 활성 Core/API 범주가 담당하는 닫기 차단 사유인지 식별하는 방법을 다룹니다.

이 문서는 독립적인 활성 관문, 설계 품질 전용 활성 `CloseReadinessBlocker.category`, 활성 검증기 계열, 설계 정책 면제 경로, 심각도 기반 차단 정책, 증거 기록, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 권한을 정의하지 않습니다.

이 문서가 담당합니다.

- 현재 MVP에서 판단 라우팅, 증거 참조, 범위 참조로 쓰이는 설계 품질 역할
- 설계 품질 관찰 사항을 `judgment_kind=product_decision`, `judgment_kind=technical_decision`, `judgment_kind=scope_decision`으로 보내는 기준
- 설계 품질 관찰 사항을 `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, `surface_capability` 같은 기존 활성 차단 사유 범주로 연결하는 기준
- 활성 설계 품질 심각도 경계. 심각도 형태의 문구는 활성 담당 경로가 별도 행동을 요구하지 않는 한 조언성 우선순위입니다.
- 설계 품질 관찰 사항, 활성 `ValidatorResult.validator_id` 값, 이후 설계 정책 후보 사이의 경계

이 문서는 담당하지 않습니다.

- Core 생명주기, 관문, 차단 사유, `prepare_write`, `close_task`, Write Authorization, 최종 수락, 잔여 위험 수락, 대체 불가능 규칙. [Core 모델 참조](core-model.md)를 봅니다.
- MCP 요청/응답 스키마, `ValidatorResult`, `UserJudgment`, `AcceptedRiskInput`, 공개 오류, 현재/이후 스키마 값. [MVP API](api/mvp-api.md), [API 코어 스키마](api/schema-core.md), [API 판단 스키마](api/schema-judgment.md), [API 오류](api/errors.md)를 봅니다.
- SQLite DDL과 지속 테이블. [저장소 기록](storage-records.md)을 봅니다.
- 검증기 실행 저장 효과. [저장 효과](storage-effects.md)를 봅니다.
- 아티팩트 저장소. [아티팩트 저장소](storage-artifacts.md)를 봅니다.
- 상태 보기 권한. [상태 보기 권한 참조](projection-and-templates.md)를 봅니다.
- 템플릿 본문, 상태 카드, 렌더링된 보고서. [템플릿 본문](template-bodies.md)을 봅니다.
- 넓은 설계 정책 검증기, 설계 정책 면제, 심각도 기반 활성 차단 정책, steward 정책, 전체 검토 절차, 운영/보고 후보, 향후 적합성 목록.

설계 품질 발견 사항이 다른 계약과 만날 때는 다음 담당 문서를 사용합니다.

| 질문 | 담당 문서 |
|---|---|
| Core 대체 금지, 닫기 준비 상태, 면제, 수락된 위험, 잔여 위험의 의미 | [Core 모델 참조](core-model.md) |
| `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, `AcceptedRiskInput` 구조 | [API 판단 스키마](api/schema-judgment.md) |
| 사용자 판단 요청/기록, 상태 보고, Task 닫기의 활성 메서드 동작 | [MVP API](api/mvp-api.md) |
| 활성 API 메서드 분기의 메서드별 저장 효과 | [저장 효과](storage-effects.md) |
| 이후 설계 관문, 정책 차단 사유, 넓은 검증기, 면제 후보, 정책 목록 | [이후 후보 색인](../later/index.md) |

이 저장소의 문서는 계획용 자료입니다. 지금 이 저장소에 하네스 서버, 런타임 상태, 생성된 증거, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 기록이 있다는 뜻이 아닙니다.

## 2. 현재 MVP 설계 품질 역할

현재 MVP에서 설계 품질은 판단 라우팅을 하고 증거와 범위를 참조하는 좁은 층입니다. 품질 관련 우려를 이해할 수 있게 만들고, 그 우려를 기존 활성 담당 경로로 보냅니다.

설계 품질 발견 사항이 현재 MVP에서 할 수 있는 일은 아래뿐입니다.

| 발견 사항이 할 수 있는 일 | 활성 경로 또는 결과 | 닫기 준비 상태 경계 |
|---|---|---|
| 사용자 판단이 필요한 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치 선택을 식별합니다. | `judgment_kind=product_decision`으로 보냅니다. | 활성 닫기 경로가 이미 `CloseReadinessBlocker.category=user_judgment`를 요구할 때만 닫기를 차단합니다. |
| 사용자 판단이 필요한 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향 선택을 식별합니다. | `judgment_kind=technical_decision`으로 보냅니다. | 활성 닫기 경로가 이미 `CloseReadinessBlocker.category=user_judgment`를 요구할 때만 닫기를 차단합니다. |
| 범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary 변경을 식별합니다. | 담당 경로에 따라 `judgment_kind=scope_decision` 또는 `CloseReadinessBlocker.category=scope`를 사용합니다. | 활성 범위 또는 사용자 판단 담당 경로를 통해서만 닫기를 차단합니다. |
| 닫기와 관련된 주장을 뒷받침하는 자료가 부족하다고 지적합니다. | Core 증거 담당 경로에서 증거를 요청하거나 `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`를 사용합니다. | 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다. |
| 알려진 한계, 확인하지 못한 조건, 절충점을 보이게 합니다. | 잔여 위험 표시를 사용하고, 활성 닫기 경로가 수락을 요구할 때만 `CloseReadinessBlocker.category=residual_risk_acceptance`를 사용합니다. | 수락된 위험은 보이는 위험에 대한 사용자 판단을 기록할 뿐, 성공을 증명하거나 위험을 지우지 않습니다. |
| 연결된 접점이 주장한 동작이나 보장을 정직하게 지원하지 못한다고 알립니다. | 역량 담당 경로에서 `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다. | 설계 품질 라벨이 보장 수준을 높이지 않습니다. |
| 우려의 상대적 긴급도나 주의 수준을 설명합니다. | 활성 담당 경로가 별도 행동을 요구하지 않는 한 조언성 우선순위입니다. | 심각도만으로 차단 사유, 검증기 매핑, 면제, 증거 기대치, 닫기 결과를 만들지 않습니다. |
| 집중된 다음 행동 하나를 고릅니다. | 집중된 사용자 판단 하나 묻기, 증거 요청, 잔여 위험 표시, 조언성 다음 행동 표시, 또는 아무 행동 없음 중 하나를 사용합니다. | 다음 행동은 이름 붙은 담당 경로를 풀거나 분명히 하는 데 필요한 만큼만 좁아야 합니다. |
| 활성 담당 경로가 없으면 조언으로 남깁니다. | 조언 문구 또는 아무 행동 없음입니다. | 새 관문, 차단 사유, 검증기 매핑, 면제 경로, 증거 규칙, 닫기 권한을 만들지 않습니다. |

활성 설계 품질은 새 Core 상태, `StateSummary.gates.design_gate`, `CloseReadinessBlocker.category=design_policy`, 새 스키마, 새 `ValidatorResult` 필드, 활성 설계 정책 검증기, 설계 정책 면제, 별도 설계 검토 권한을 만들지 않습니다.

설계 품질은 평범한 작업을 끝없는 계획 반복으로 만들면 안 됩니다. 전체 도메인 언어 점검, 전체 모듈/인터페이스 검토, 전체 TDD 추적, 전체 피드백 루프 감사, 전체 `codebase_stewardship` 검토, 자세한 수동 QA 정책, 분리형 검증, 두 단계 검토 표시, steward 정책은 다른 활성 담당 경로가 좁은 일부를 명시적으로 요구하지 않는 한 현재 MVP 차단 사유가 아닙니다.

## 3. 라우팅 규칙

설계 품질 관찰 사항은 활성 담당 경로를 통해서만 현재 MVP 상태에 영향을 줍니다. 관찰 사항은 자신이 의존하는 활성 경로를 이름 붙여야 합니다.

| 우려 | 현재 MVP 활성 경로 |
|---|---|
| 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치가 정해지지 않았습니다. | `judgment_kind=product_decision`. 활성 닫기 경로가 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다. |
| 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향이 정해지지 않았습니다. | `judgment_kind=technical_decision`. 활성 닫기 경로가 그 판단을 요구할 때만 `CloseReadinessBlocker.category=user_judgment`를 사용합니다. |
| 범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary 변경이 필요합니다. | 담당 경로에 따라 `judgment_kind=scope_decision` 또는 `CloseReadinessBlocker.category=scope`를 사용합니다. |
| 닫기와 관련된 주장을 뒷받침하는 자료가 부족합니다. | Core 증거 담당 경로에서 `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`, 또는 증거 요청을 사용합니다. |
| 알려진 한계나 확인하지 못한 조건이 닫기에 중요합니다. | `CloseReadinessBlocker.category=residual_risk_visibility`로 잔여 위험을 보이게 하고, 활성 닫기 경로가 수락을 요구할 때만 `CloseReadinessBlocker.category=residual_risk_acceptance`를 사용합니다. |
| 연결된 접점이 주장한 동작이나 보장을 정직하게 지원하지 못합니다. | 역량 담당 경로에서 `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다. |

설계 품질 라벨, 정책 이름, 심각도 값, 검증기 ID, 검토 문구는 그 자체로 경로를 만들지 않습니다. 적용되는 활성 담당 경로가 없으면 현재 MVP 결과는 조언 문구이거나 아무 행동 없음입니다.

<a id="when-a-finding-blocks-close"></a>
## 4. 닫기 차단 사유가 되는 조건

설계 품질 관찰 사항은 아래 조건을 모두 만족할 때만 닫기를 차단합니다.

- 활성 Task 또는 Change Unit과 시도 중인 닫기에 연결되어 있습니다.
- 활성 닫기 차단 집합 안의 기존 활성 `CloseReadinessBlocker.category`, `judgment_kind`, API 오류, 담당 경로를 이름 붙입니다.
- 이름 붙인 담당 경로가 설계 품질 라벨 없이도 닫기를 차단할 조건입니다.
- 차단 해소, 담당 경로를 통한 유예, 필요한 증거 요청, 잔여 위험 표시 중 하나로 이어지는 다음 행동을 정확히 하나 제공합니다.
- `design_gate`, `CloseReadinessBlocker.category=design_policy`, 설계 정책 면제, 넓은 정책 목록, 심각도 값만으로 차단하지 않습니다.

발견 사항이 도메인 언어, 세로 조각 형태, TDD, 모듈/인터페이스 검토, stewardship, 수동 QA, 분리형 검증, 검토 단계, 향후 정책 후보를 언급한다는 이유만으로 닫기를 차단하지 않습니다. 활성 담당 경로가 좁은 행동을 필요로 할 때만 조언성 다음 행동, 증거 요청, 집중된 사용자 판단, 잔여 위험 표시로 이어질 수 있습니다.

설계 품질 관찰 사항이 닫기에 영향을 주더라도 닫기 준비 상태 평가의 닫기 차단 사유는 [API 값 집합](api/schema-value-sets.md)이 담당하는 활성 `CloseReadinessBlocker.category` 값을 사용해야 합니다.

## 5. 현재 설계 정책 면제 없음

현재 MVP에는 활성 설계 품질 면제나 설계 정책 면제 경로가 없습니다. 어떤 요구사항을 유예하거나, 위험으로 수락하거나, 사용자 판단으로 해결할 수 있는지는 해당 활성 담당 경로가 정합니다. 그 경로의 정확한 `judgment_kind`, 차단 사유 범주, 증거 동작을 사용해야 합니다.

면제에 가까운 결정이나 수락된 위험 답변은 이름 붙은 요구사항 또는 이름 붙은 보이는 위험에 대한 책임 있는 사용자 판단을 기록합니다. 사실을 지우거나, 닫기 근거에서 남은 한계를 제거하거나, 증거를 만들거나, 검증을 증명하거나, QA를 통과시키거나, 최종 수락을 대신하거나, 닫기를 자동으로 성공시키지 않습니다.

판단 경로는 계속 서로 구분합니다.

| 경로 | 기록하는 것 | 취급하면 안 되는 것 |
|---|---|---|
| `final_acceptance` | 닫기 근거가 보인 뒤 사용자가 결과를 판단한 내용입니다. | 증거 생성, 잔여 위험 수락, QA, 검증, 차단 사유 우회. |
| `residual_risk_acceptance` | 요청한 닫기를 위해 이름 붙은 보이는 잔여 위험을 사용자가 수락한 내용입니다. | 정확성 증명, 증거 충분성, 최종 수락, 무위험 결과, 자동 성공. |
| 현재 MVP의 활성 `UserJudgment.judgment_kind` 값 | [API 값 집합](api/schema-value-sets.md)이 담당하는 집중된 사용자 소유 결정입니다. | 설계 정책 면제, 포괄 승인, 이후 QA 면제, 이후 검증 위험 수락, 승격되지 않은 향후 후보. |
| 향후 설계 품질 면제 후보 | [이후 후보 색인](../later/index.md)에 남아 있는 이후 전용 자료입니다. | 현재 요구사항, 닫기 차단 사유, 검증기 동작, 증거 규칙. |

넓은 승인, "looks good" 같은 말, 일반적인 진행 승인은 활성 담당 경로가 그 특정 판단을 물은 경우가 아니라면 위 판단으로 취급하면 안 됩니다.

## 6. 증거 기대치

설계 품질 관찰 사항은 증거 공백을 식별할 수 있지만, 필요한 증거는 Core 증거 담당 경로에 속합니다. 발견 사항은 활성 담당 경로가 쓰기 안전성, 닫기 준비 상태, 사용자 판단, 잔여 위험, 정직한 보장 표시에 영향을 주는 주장을 뒷받침해야 할 때만 증거를 요청해야 합니다.

유용한 증거 참조는 다음을 포함할 수 있습니다.

- 지속 `ArtifactRef` 값, Run 참조, 명령/확인 요약, 출처 참조
- 최신이 아닌 맥락이 닫기 근거에 영향을 줄 때 현재 상태/버전/최신성 참조
- 제품, 기술, 범위, 최종 수락, 잔여 위험 판단에 대한 사용자 판단 참조
- 알려진 한계가 닫기에서 보일 때 잔여 위험 참조
- 향후 수동 QA 또는 검증 참조는 해당 이후 후보 담당 경로가 승격된 뒤에만 사용

채팅 주장, 일반 요약, 렌더링된 상태 보기 문장, 등록되지 않은 파일, 담당 경로 없는 화면 캡처, 테스트 통과만 있는 상태, 향후 면제 후보, 최종 수락, 잔여 위험 수락은 필요한 증거를 자동으로 충족하지 않습니다. 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다. 필수가 아닌 증거 공백은 상황에 맞게 `request evidence`, `show advisory next action`, 또는 잔여 위험 표시로 라우팅해야 합니다.

## 7. 검증기 ID 경계

검증기 ID는 보고용 라벨입니다. Core 불변조건, 관문, 닫기 차단 사유, 면제, 증거 기록, 사용자 판단을 만들지 않습니다.

`ValidatorResult` 형태는 [API 상태 스키마](api/schema-state.md)가 담당합니다. `severity` 형태 값과 활성 안정 `ValidatorResult.validator_id` 집합은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.

이 문서는 활성 설계 정책 검증기 ID나 정책-검증기 매핑을 제공하지 않습니다. 이후 안정 검증기 ID 집합은 담당 문서가 좁은 활성 계약으로 승격하기 전까지 [정책과 적합성: `ValidatorResult` 안정 ID와 정책 계열](../later/policy-and-conformance.md#validatorresult-stable-ids-and-policy-families)의 후보로 남습니다.

## 8. 이후 정책 후보 경계

전체 설계 품질 정책 목록은 현재 MVP 범위가 아닙니다. 아래 아이디어는 이름 있는 담당 문서가 범위, 대체 동작, 정확한 계약, 증명 기대치와 함께 좁은 동작을 승격하기 전까지 이후 전용입니다.

| 이후 전용 아이디어 | 현재 MVP에서 하지 않는 일 | 승격에 필요한 것 |
|---|---|---|
| `design_gate`와 `CloseReadinessBlocker.category=design_policy` | 활성 관문, 활성 닫기 차단 사유, 닫기 준비 상태 범주를 만들지 않습니다. | Core/API 담당 문서 변경과 값 집합, 스키마, 닫기 준비 상태, 저장 효과 담당 경계. |
| 설계 정책 면제 | 활성 면제 경로나 자동 성공 경로를 만들지 않습니다. | 이름 붙은 담당 경로, 대체 금지 규칙, 사용자 판단 동작, 닫기 준비 상태 효과. |
| 넓은 설계 검증기와 심각도 기반 차단 | 활성 검증기 ID, 심각도 의미, 정책-검증기 매핑, 심각도만으로 생기는 차단 사유를 만들지 않습니다. | 안정적인 검증기 집합 담당 문서, 심각도 의미, API/스키마 경계, 대체 동작. |
| 전체 설계 품질 정책 계열과 steward 정책 | 활성 정책 목록, stewardship 관문, 전체 검토 절차를 만들지 않습니다. | 범위 있는 정책 담당 문서, 독자에게 보이는 동작, 증명 기대치, 활성/이후 전환 경로. |
| 자세한 검토 표시, 운영/보고 후보, 전체 검증기 매핑, 향후 적합성 fixture | 활성 운영 보고, fixture 요구사항, 구현 작업, 적합성 의무를 만들지 않습니다. | [이후 후보 색인](../later/index.md)을 통한 승격, 활성 담당 문서 갱신, 구현 작업 전 문서 전용 수락. |

이후 후보는 이름만 유지할 수 있습니다. 이를 현재 MVP 요구사항, 차단 사유, 면제 규칙, 증거 기대치, 검증기 매핑, fixture 요구사항, 운영 보고, 구현 작업처럼 제시하면 안 됩니다.
