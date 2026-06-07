# 설계 품질

## 1. 담당하는 것 / 담당하지 않는 것

이 참조 문서는 현재 활성 MVP의 설계 품질 경계를 담당합니다. 설계 품질 발견 사항이 gate, 닫기 차단 사유, 면제, 증거 기대치에 영향을 줄 수 있는 시점을 설명하되, 이를 Core 불변 규칙처럼 만들지 않습니다.

이 문서가 담당합니다.

- 현재 활성 MVP 닫기 동작에서 설계 품질의 역할
- 보이는 차단 사유나 다음 행동에 영향을 주는 발견 사항 심각도 해석
- 설계 품질 발견 사항이 Core가 뒷받침하는 닫기 차단 사유가 될 수 있는 조건
- 설계 품질 기대치에 대한 면제 경계
- 설계 품질 발견 사항의 증거 기대치
- validator ID, 활성 닫기 영향, later 정책 후보 사이의 경계

이 문서는 담당하지 않습니다.

- Core 생명주기, gate, blocker, `prepare_write`, `close_task`, Write Authorization, 최종 수락, 잔여 위험 수락, 대체 불가능 규칙. [Core Model 참조](core-model.md)를 봅니다.
- MCP 요청/응답 스키마, `ValidatorResult`, 공개 오류, active/later 스키마 값. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- SQLite DDL, 지속 테이블, validator-run 저장소, 아티팩트 저장소. [Storage](storage.md)를 봅니다.
- 상태 보기 템플릿 본문, 상태 카드, 렌더링된 보고서. [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- 전체 정책-검증기 매핑, steward 정책, 전체 검토 절차, 운영/보고 자료, 향후 적합성 목록.

이 저장소의 문서는 계획용 자료입니다. 지금 이 저장소에 하네스 서버, 런타임 상태, 생성된 증거, QA 기록, 수락 기록, residual-risk 기록, 닫기 기록이 있다는 뜻이 아닙니다.

## 2. 현재 활성 MVP 설계 품질 역할

현재 활성 MVP에서 설계 품질은 좁은 라우팅 층입니다. 닫기에 영향을 주는 품질 발견 사항을 보이게 하고, 각 발견 사항을 기존 담당 경로로 보냅니다. 새 Core 상태, 새 gate, 새 스키마, 새 `ValidatorResult` 필드, 별도 설계 검토 권한을 만들지 않습니다.

활성 역할은 아래 영향으로 제한됩니다.

- 범위, 사용자 소유 판단, 필요한 증거, 최신이 아닌 닫기/쓰기 맥락, 접점 역량, 정직한 보장 표시, 보이는 잔여 위험에 영향을 주는 발견 사항을 표시합니다.
- `ask one focused user judgment`, `request evidence`, `mark residual risk`, `show advisory next action`, `no action` 중 집중된 다음 행동 하나로 라우팅합니다.
- Core 담당 경로가 해당 차단 사유를 이미 뒷받침할 때만 `block write` 또는 `block close`로 라우팅합니다. 현재 MVP에는 별도 `design_policy` `CloseBlocker.category`가 없습니다.
- 사용자 소유 제품 판단, 중요한 기술 판단, 최종 수락, 잔여 위험 판단, later/reserved QA 면제 또는 검증 위험 판단을 구분합니다.
- 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 닫기 준비 상태를 구분합니다.

설계 품질은 평범한 작업을 끝없는 계획 반복으로 만들면 안 됩니다. 전체 도메인 언어 점검, 전체 모듈/인터페이스 검토, 전체 TDD 추적, 전체 피드백 루프 감사, 전체 `codebase_stewardship` 검토, 자세한 수동 QA 정책, 분리형 검증, 두 단계 검토 표시, steward 정책은 다른 활성 담당 경로가 좁은 일부를 명시적으로 요구하지 않는 한 현재 활성 MVP 차단 사유가 아닙니다.

## 3. 발견 사항 심각도

`ValidatorResult.findings.severity`는 [API Schema Core](api/schema-core.md#validatorresult)가 담당합니다. 설계 품질은 보이는 다음 행동과 가능한 닫기 영향에 필요한 범위에서만 심각도를 해석합니다.

| 심각도 | 현재 활성 MVP 의미 |
|---|---|
| `info` | 유용한 맥락입니다. 쓰기나 닫기를 차단하지 않습니다. |
| `warning` | 에이전트가 보여 주거나 제한된 다음 행동 하나로 라우팅해야 하는 관심 사항입니다. 그 자체로 쓰기나 닫기를 차단하지 않습니다. |
| `error` | 품질 기대치가 충족되지 않았습니다. 증거 요청, 집중된 사용자 판단 하나, 잔여 위험 표시, 조언으로 보는 다음 행동으로 이어질 수 있습니다. [닫기 차단 사유가 되는 조건](#when-a-finding-blocks-close)이 적용될 때만 닫기를 차단합니다. |
| `blocker` | 주장된 blocker는 활성 Core가 뒷받침하는 blocker, gate, API 오류 경로를 이름 붙여야 합니다. 그런 담당 경로가 없으면 닫기 차단 사유로 표시하면 안 됩니다. |

같은 영향 대상에서는 유효한 활성 동작 중 가장 강한 동작을 보여 주고 더 약한 발견 사항도 숨기지 않습니다. 서로 다른 영향 대상은 서로 분리합니다. later 후보 `warning`은 다른 Core가 뒷받침하는 관심 사항이 차단한다고 해서 `blocker` 상태를 상속하면 안 됩니다.

<a id="when-a-finding-blocks-close"></a>
## 4. 닫기 차단 사유가 되는 조건

설계 품질 발견 사항은 아래 조건을 모두 만족할 때만 닫기를 차단합니다.

- 활성 Task 또는 Change Unit과 시도 중인 닫기에 연결되어 있습니다.
- 현재 활성 닫기 차단 집합 안에서 기존 Core가 뒷받침하는 닫기 차단 사유 category, gate, API 오류, 담당 경로를 이름 붙입니다.
- 해결, 연기, 허용된 면제, 잔여 위험 표시 중 하나로 이어지는 다음 행동을 정확히 하나 제공합니다.
- 아래 현재 활성 MVP 차단 조건 중 하나에 해당합니다.

현재 활성 MVP 차단 조건은 다음뿐입니다.

| 조건 | 담당 경로 |
|---|---|
| 필요한 사용자 소유 판단이 해결되지 않았습니다. | `decision_gate`, `user_judgment`, Core 닫기 의미. |
| 닫기에 영향을 주는 작업에 필요한 활성 범위가 없거나 맞지 않거나 Autonomy Boundary를 넘었습니다. | Scope Gate, Change Unit, Autonomy Boundary, `prepare_write`, 닫기 차단 사유. |
| 필요한 증거가 없거나, 사용할 수 없거나, 오래됐거나, blocked 상태입니다. | Core 증거 요약, 아티팩트 가용성, `EVIDENCE_INSUFFICIENT` 경로. |
| 최신이 아닌 맥락 때문에 닫기 근거를 안전하게 믿을 수 없습니다. | Core freshness, 보이는 닫기 근거에 쓰이는 Projection/source ref, reconcile/recovery 담당 경로. |
| 접점(surface)이 주장한 동작 또는 보장을 지원하지 못합니다. | 역량 경계, `CAPABILITY_INSUFFICIENT`, 정직한 보장 표시 담당 문서. |

발견 사항이 도메인 언어, 세로 조각 형태, TDD, 모듈/인터페이스 검토, stewardship, 수동 QA, 분리형 검증, 검토 단계, 향후 정책 후보를 언급한다는 이유만으로 닫기를 차단하지 않습니다. 활성 담당 경로가 좁은 행동을 필요로 할 때만 조언성 다음 행동, 증거 요청, 집중된 사용자 판단, 잔여 위험 표시로 이어질 수 있습니다.

설계 품질 발견 사항이 닫기에 영향을 주더라도 차단 사유는 [API Schema Core](api/schema-core.md#current-mvp-value-sets)가 담당하는 활성 `CloseBlocker.category` 값 중 하나를 사용해야 합니다. 예를 들면 `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, `surface_capability`, `baseline`, `recovery`, `cancellation`, `supersession`입니다.

## 5. 면제 경계

설계 품질 면제는 활성 담당 경로가 면제를 허용하는 설계 품질 기대치에만 영향을 줄 수 있습니다. 면제는 명시적이어야 하고 affected Task/Change Unit 또는 발견 사항에 범위가 정해져야 하며, 판단이 사용자에게 속하면 관련 user-judgment 또는 담당 경로로 기록해야 합니다.

설계 품질 면제는 아래 항목을 면제하지 않습니다.

- 활성 범위 누락 또는 맞지 않는 Write Authorization
- 민감 동작 승인
- 필요한 증거 범위 또는 아티팩트 가용성
- 필요한 최종 수락
- 필요한 잔여 위험 표시, 그리고 활성 닫기 경로가 요구할 때의 잔여 위험 수락
- 검증 독립성
- Core가 소유한 닫기 차단 사유

판단 경로는 서로 구분합니다.

- `qa_waiver`는 later/reserved 값입니다. 승격되면 QA 담당 경로가 허용하는 범위 있는 QA 요구사항만 면제합니다. QA 증거나 QA 통과 결과가 아닙니다.
- `verification_risk_acceptance`는 later/reserved 값입니다. 승격되면 빠졌거나 면제된 검증의 위험을 수락합니다. 분리 검증(detached verification)을 만들지 않습니다.
- `final_acceptance`는 닫기 근거가 보인 뒤 사용자가 결과를 판단하는 것입니다. 증거를 만들거나 잔여 위험을 수락하지 않습니다.
- `residual_risk_acceptance`는 이름 붙은 보이는 잔여 위험을 수락합니다. 정확성을 증명하거나 최종 수락을 대신하지 않습니다.

넓은 승인, "looks good" 같은 말, 일반적인 go-ahead는 활성 담당 경로가 그 특정 판단을 물은 경우가 아니라면 위 판단으로 취급하면 안 됩니다.

## 6. 증거 기대치

설계 품질의 증거 기대치는 좁고 닫기에 관련됩니다. 발견 사항은 활성 담당 경로가 쓰기 안전성, 닫기 준비 상태, 사용자 판단, 잔여 위험, 정직한 guarantee 표시에 영향을 주는 주장을 뒷받침해야 할 때만 증거를 요청해야 합니다.

유용한 증거 참조는 다음을 포함할 수 있습니다.

- 등록된 `ArtifactRef` 값, Run ref, command/check summary, source ref
- 최신이 아닌 맥락이 닫기 근거에 영향을 줄 때 current state/version/freshness ref
- 제품, 기술, 범위, 최종 수락, 잔여 위험 판단, 그리고 승격된 later/reserved QA 면제와 검증 위험 판단에 대한 user-judgment ref
- 알려진 한계가 닫기에서 보일 때 residual-risk ref
- 해당 담당 경로가 active이거나 명시적으로 요구할 때만 수동 QA 또는 verification ref

채팅 주장, 일반 요약, 렌더링된 Projection 문장, 등록되지 않은 파일, 담당 경로 없는 screenshot, 테스트 통과만 있는 상태, later QA 면제, 최종 수락, 잔여 위험 수락은 필요한 증거를 자동으로 충족하지 않습니다. 필요한 증거는 Core evidence 담당 경로를 통해서만 닫기를 차단할 수 있습니다. 필수가 아닌 증거 공백은 상황에 맞게 `request evidence`, `show advisory next action`, 또는 residual-risk visibility로 라우팅해야 합니다.

## 7. Validator ID 경계

Validator ID는 보고용 라벨입니다. Core 불변조건, gate, 닫기 차단 사유, 면제, 증거 기록, 사용자 판단을 만들지 않습니다.

`ValidatorResult` 형태와 `severity` 값은 [API Schema Core](api/schema-core.md#validatorresult)가 담당합니다. later 안정 `validator_id` 집합은 담당 문서가 좁은 활성 계약으로 승격하기 전까지 [Later 후보 색인: Later schema 후보](../later/index.md#later-schema-candidates)의 후보로 남습니다.

이 문서는 전체 정책-검증기 매핑을 제공하지 않습니다. 현재 또는 향후 `ValidatorResult`가 설계 품질 발견 사항을 보고하더라도 닫기 영향은 validator ID 자체가 아니라 [닫기 차단 사유가 되는 조건](#when-a-finding-blocks-close)과 관련 Core/API 담당 경로에서 옵니다.

## 8. Later 정책 후보 경계

전체 설계 품질 later 정책 후보 목록은 현재 활성 MVP 범위가 아닙니다. 향후 정책 후보, steward 정책, 자세한 검토 표시, 운영/보고 후보, 전체 검증기 매핑, 향후 적합성 fixture는 이름 있는 담당 문서가 범위, 대체 동작, 정확한 계약, 증명 기대치와 함께 좁은 동작을 승격하기 전까지 [Later 후보 색인](../later/index.md)에 남습니다.

Later 후보는 이름만 유지할 수 있습니다. 이를 현재 활성 MVP 요구사항, 차단 사유, 면제 규칙, 증거 기대치, 검증기 매핑, fixture 요구사항, 운영 보고, 구현 작업처럼 제시하면 안 됩니다.
