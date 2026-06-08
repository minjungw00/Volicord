# 설계 품질

## 1. 담당하는 것 / 담당하지 않는 것

이 참조 문서는 현재 MVP의 설계 품질 라우팅 경계를 판단 라우팅, 증거 참조, 범위 참조로 담당합니다. 설계 품질에서 보인 관찰 사항이 제품 판단, 기술 판단, 범위 판단, 증거 공백, 잔여 위험 표시 문제, 또는 이미 활성 Core/API 범주가 담당하는 닫기 차단 사유인지 식별하는 방법을 다룹니다.

이 문서는 독립적인 활성 관문, 활성 `CloseBlocker.category`, 활성 validator 계열, 설계 정책 면제 경로, 심각도 기반 차단 정책, 증거 기록, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 권한을 정의하지 않습니다.

이 문서가 담당합니다.

- 현재 MVP에서 판단 라우팅, 증거 참조, 범위 참조로 쓰이는 설계 품질 역할
- 설계 품질 관찰 사항을 `judgment_kind=product_decision`, `judgment_kind=technical_decision`, `judgment_kind=scope_decision`으로 보내는 기준
- 설계 품질 관찰 사항을 `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, `surface_capability` 같은 기존 활성 차단 사유 범주로 연결하는 기준
- 설계 품질 관찰 사항, 활성 `ValidatorResult.validator_id` 값, 이후 설계 정책 후보 사이의 경계

이 문서는 담당하지 않습니다.

- Core 생명주기, 관문, 차단 사유, `prepare_write`, `close_task`, Write Authorization, 최종 수락, 잔여 위험 수락, 대체 불가능 규칙. [Core Model 참조](core-model.md)를 봅니다.
- MCP 요청/응답 스키마, `ValidatorResult`, 공개 오류, active/later 스키마 값. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- SQLite DDL, 지속 테이블, validator-run 저장소, 아티팩트 저장소. [Storage](storage.md)를 봅니다.
- 상태 보기 템플릿 본문, 상태 카드, 렌더링된 보고서. [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- 넓은 설계 정책 validator, 설계 정책 waiver, 심각도 기반 활성 차단 정책, steward 정책, 전체 검토 절차, 운영/보고 후보, 향후 적합성 목록.

이 저장소의 문서는 계획용 자료입니다. 지금 이 저장소에 하네스 서버, 런타임 상태, 생성된 증거, QA 기록, 수락 기록, 잔여 위험 기록, 닫기 기록이 있다는 뜻이 아닙니다.

## 2. 현재 MVP 설계 품질 역할

현재 MVP에서 설계 품질은 판단 라우팅을 하고 증거와 범위를 참조하는 좁은 층입니다. 품질 관련 우려를 이해할 수 있게 만들고, 그 우려를 기존 활성 담당 경로로 보냅니다. 새 Core 상태, `StateSummary.gates.design_gate`, `CloseBlocker.category=design_policy`, 새 스키마, 새 `ValidatorResult` 필드, 활성 설계 정책 validator, 설계 정책 waiver, 별도 설계 검토 권한을 만들지 않습니다.

활성 역할은 아래 영향으로 제한됩니다.

- 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치 선택을 `judgment_kind=product_decision`으로 식별합니다.
- 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향 선택을 `judgment_kind=technical_decision`으로 식별합니다.
- 범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary 변경을 `judgment_kind=scope_decision`으로 식별합니다.
- 해당 활성 담당 경로가 이미 차단을 요구할 때만 `CloseBlocker.category=scope`, `CloseBlocker.category=user_judgment`, `CloseBlocker.category=evidence`, `CloseBlocker.category=artifact_availability`를 가리킵니다.
- 해당 담당 경로가 실제로 맞을 때만 `CloseBlocker.category=residual_risk_visibility`, `CloseBlocker.category=residual_risk_acceptance`, `CloseBlocker.category=surface_capability`, 또는 다른 이미 활성화된 범주를 가리킵니다.
- 집중된 사용자 판단 하나 묻기, 증거 요청, 잔여 위험 표시, 조언성 다음 행동 표시, 또는 아무 행동 없음 중 하나로 라우팅합니다.
- 사용자 소유 제품 판단, 중요한 기술 판단, 범위 판단, 최종 수락, 잔여 위험 판단, 취소 판단을 구분합니다.
- 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 닫기 준비 상태를 구분합니다. 검증과 수동 QA는 현재 MVP의 활성 관문이 아닙니다.

설계 품질은 평범한 작업을 끝없는 계획 반복으로 만들면 안 됩니다. 전체 도메인 언어 점검, 전체 모듈/인터페이스 검토, 전체 TDD 추적, 전체 피드백 루프 감사, 전체 `codebase_stewardship` 검토, 자세한 수동 QA 정책, 분리형 검증, 두 단계 검토 표시, steward 정책은 다른 활성 담당 경로가 좁은 일부를 명시적으로 요구하지 않는 한 현재 MVP 차단 사유가 아닙니다.

## 3. 라우팅 규칙

설계 품질 관찰 사항은 활성 담당 경로를 통해서만 현재 MVP 상태에 영향을 줍니다. 관찰 사항은 자신이 의존하는 활성 경로를 이름 붙여야 합니다.

| 우려 | 현재 MVP 활성 경로 |
|---|---|
| 제품 동작, UX, 문구, 릴리스 약속, 사용자 가치가 정해지지 않았습니다. | `judgment_kind=product_decision`. 활성 닫기 경로가 그 판단을 요구할 때만 `CloseBlocker.category=user_judgment`를 사용합니다. |
| 아키텍처, 의존성, 마이그레이션, 공개 인터페이스, 호환성, 보안/개인정보, 중요한 기술 방향이 정해지지 않았습니다. | `judgment_kind=technical_decision`. 활성 닫기 경로가 그 판단을 요구할 때만 `CloseBlocker.category=user_judgment`를 사용합니다. |
| 범위 확장, 비목표 제거, Change Unit 경계, Autonomy Boundary 변경이 필요합니다. | 담당 경로에 따라 `judgment_kind=scope_decision` 또는 `CloseBlocker.category=scope`를 사용합니다. |
| 닫기와 관련된 주장을 뒷받침하는 자료가 부족합니다. | Core 증거 담당 경로에서 `CloseBlocker.category=evidence`, `CloseBlocker.category=artifact_availability`, 또는 증거 요청을 사용합니다. |
| 알려진 한계나 확인하지 못한 조건이 닫기에 중요합니다. | `CloseBlocker.category=residual_risk_visibility`로 잔여 위험을 보이게 하고, 활성 닫기 경로가 수락을 요구할 때만 `CloseBlocker.category=residual_risk_acceptance`를 사용합니다. |
| 연결된 접점이 주장한 동작이나 보장을 정직하게 지원하지 못합니다. | 역량 담당 경로에서 `CloseBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, 또는 낮아진 보장 표시를 사용합니다. |

설계 품질 라벨, 정책 이름, 심각도 값, validator ID, 검토 문구는 그 자체로 경로를 만들지 않습니다. 적용되는 활성 담당 경로가 없으면 현재 MVP 결과는 조언 문구이거나 아무 행동 없음입니다.

<a id="when-a-finding-blocks-close"></a>
## 4. 닫기 차단 사유가 되는 조건

설계 품질 관찰 사항은 아래 조건을 모두 만족할 때만 닫기를 차단합니다.

- 활성 Task 또는 Change Unit과 시도 중인 닫기에 연결되어 있습니다.
- 활성 닫기 차단 집합 안의 기존 활성 `CloseBlocker.category`, `judgment_kind`, API 오류, 담당 경로를 이름 붙입니다.
- 이름 붙인 담당 경로가 설계 품질 라벨 없이도 닫기를 차단할 조건입니다.
- 차단 해소, 담당 경로를 통한 유예, 필요한 증거 요청, 잔여 위험 표시 중 하나로 이어지는 다음 행동을 정확히 하나 제공합니다.
- `design_gate`, `CloseBlocker.category=design_policy`, 설계 정책 waiver, 넓은 정책 목록, 심각도 값만으로 차단하지 않습니다.

발견 사항이 도메인 언어, 세로 조각 형태, TDD, 모듈/인터페이스 검토, stewardship, 수동 QA, 분리형 검증, 검토 단계, 향후 정책 후보를 언급한다는 이유만으로 닫기를 차단하지 않습니다. 활성 담당 경로가 좁은 행동을 필요로 할 때만 조언성 다음 행동, 증거 요청, 집중된 사용자 판단, 잔여 위험 표시로 이어질 수 있습니다.

설계 품질 관찰 사항이 닫기에 영향을 주더라도 차단 사유는 [API Schema Core](api/schema-core.md#current-mvp-value-sets)가 담당하는 활성 `CloseBlocker.category` 값 중 하나를 사용해야 합니다. 예를 들면 `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, `residual_risk_acceptance`, `surface_capability`, `baseline`, `recovery`, `cancellation`, `supersession`입니다.

## 5. 현재 설계 정책 waiver 없음

현재 MVP에는 활성 설계 품질 waiver나 설계 정책 waiver 경로가 없습니다. 어떤 요구사항을 유예하거나, 위험으로 수락하거나, 사용자 판단으로 해결할 수 있는지는 해당 활성 담당 경로가 정합니다. 그 경로의 정확한 `judgment_kind`, 차단 사유 범주, 증거 동작을 사용해야 합니다.

판단 경로는 계속 서로 구분합니다.

- `final_acceptance`는 닫기 근거가 보인 뒤 사용자가 결과를 판단하는 것입니다. 증거를 만들거나 잔여 위험을 수락하지 않습니다.
- `residual_risk_acceptance`는 이름 붙은 보이는 잔여 위험을 수락합니다. 정확성을 증명하거나 최종 수락을 대신하지 않습니다.
- 현재 MVP의 활성 `UserJudgment.judgment_kind` 값은 [API Schema Core](api/schema-core.md#current-mvp-value-sets)에 있는 일곱 값뿐입니다. 그 밖의 향후 후보는 승격 전까지 [이후 후보 색인](../later/index.md)에 남습니다.

넓은 승인, "looks good" 같은 말, 일반적인 진행 승인은 활성 담당 경로가 그 특정 판단을 물은 경우가 아니라면 위 판단으로 취급하면 안 됩니다.

## 6. 증거 기대치

설계 품질 관찰 사항은 증거 공백을 식별할 수 있지만, 필요한 증거는 Core 증거 담당 경로에 속합니다. 발견 사항은 활성 담당 경로가 쓰기 안전성, 닫기 준비 상태, 사용자 판단, 잔여 위험, 정직한 보장 표시에 영향을 주는 주장을 뒷받침해야 할 때만 증거를 요청해야 합니다.

유용한 증거 참조는 다음을 포함할 수 있습니다.

- 지속 `ArtifactRef` 값, Run 참조, 명령/확인 요약, 출처 참조
- 최신이 아닌 맥락이 닫기 근거에 영향을 줄 때 현재 상태/버전/최신성 참조
- 제품, 기술, 범위, 최종 수락, 잔여 위험 판단에 대한 사용자 판단 참조
- 알려진 한계가 닫기에서 보일 때 잔여 위험 참조
- 향후 수동 QA 또는 검증 참조는 해당 이후 후보 담당 경로가 승격된 뒤에만 사용

채팅 주장, 일반 요약, 렌더링된 Projection 문장, 등록되지 않은 파일, 담당 경로 없는 화면 캡처, 테스트 통과만 있는 상태, 향후 면제 후보, 최종 수락, 잔여 위험 수락은 필요한 증거를 자동으로 충족하지 않습니다. 필요한 증거는 Core 증거 담당 경로를 통해서만 닫기를 차단할 수 있습니다. 필수가 아닌 증거 공백은 상황에 맞게 `request evidence`, `show advisory next action`, 또는 잔여 위험 표시로 라우팅해야 합니다.

## 7. Validator ID 경계

Validator ID는 보고용 라벨입니다. Core 불변조건, gate, 닫기 차단 사유, 면제, 증거 기록, 사용자 판단을 만들지 않습니다.

`ValidatorResult` 형태와 `severity` 값은 [API Schema Core](api/schema-core.md#validatorresult)가 담당합니다. 활성 안정 `ValidatorResult.validator_id` 집합은 [API Schema Core: 현재 MVP 값 집합](api/schema-core.md#current-mvp-value-sets)에 있는 값으로 제한됩니다. 그 표에 `surface_capability_check`만 있다면 그것이 유일한 활성 안정 validator ID입니다.

이 문서는 활성 설계 정책 validator ID나 정책-검증기 매핑을 제공하지 않습니다. 이후 안정 validator ID 집합은 담당 문서가 좁은 활성 계약으로 승격하기 전까지 [이후 후보 색인: 이후 스키마 후보](../later/index.md#later-schema-candidates)의 후보로 남습니다.

## 8. 이후 정책 후보 경계

전체 설계 품질 이후 정책 후보 목록은 현재 MVP 범위가 아닙니다. 넓은 설계 정책 validator, 설계 정책 waiver, 심각도 기반 활성 차단 정책, 더 풍부한 정책 후보, steward 정책, 자세한 검토 표시, 운영/보고 후보, 전체 검증기 매핑, 향후 적합성 fixture는 이름 있는 담당 문서가 범위, 대체 동작, 정확한 계약, 증명 기대치와 함께 좁은 동작을 승격하기 전까지 [이후 후보 색인](../later/index.md)에 남습니다.

이후 후보는 이름만 유지할 수 있습니다. 이를 현재 MVP 요구사항, 차단 사유, 면제 규칙, 증거 기대치, 검증기 매핑, fixture 요구사항, 운영 보고, 구현 작업처럼 제시하면 안 됩니다.
