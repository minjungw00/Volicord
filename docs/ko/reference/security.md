# 보안 참조

이 참조 문서는 활성 하네스 MVP 계획의 보안 경계 표현을 담당합니다. 이 저장소는 아직 문서 전용입니다. 지금 이곳에는 Harness Server/runtime 구현, Harness Runtime Home, 실행 가능한 적합성 실행기, 런타임 보안 증명이 없습니다. 이 문서는 향후 구현이 지켜야 할 경계를 설명할 뿐, 통제가 이미 구현되었다는 증거가 아닙니다.

보안 문구, 로컬 접근 태세, 위협/통제 요약, 보장 라벨을 정직하게 유지해야 할 때 이 문서를 사용합니다. 정확한 동작은 각 담당 문서를 사용합니다. [Core Model 참조](core-model.md), [런타임 경계 참조](runtime-boundaries.md), [Storage](storage.md), [Agent 통합 참조](agent-integration.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [적합성 참조](conformance.md)가 해당 담당 문서입니다. 향후 운영 후보는 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다. 이 문서에 언급되었다는 이유만으로 현재 MVP 보안 보장이 되지 않습니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 문서가 담당하는 것:

- 보안 자산 범주와 신뢰 경계 범주
- `cooperative`, `detective`, profile-gated `preventive` / `isolated` 보안 보장 라벨의 의미
- 보안 표시가 입증된 통제와 일치해야 한다는 규칙
- 현재 MVP의 명시적 비보장
- Core 권한, 사용자 소유 판단, 증거, 저장소, 커넥터, Projection을 구분하는 위협/통제 요약
- 보안 주장에 대한 담당 문서 간 검토 확인

이 문서가 담당하지 않는 것:

- Core 상태 전이, gate, `prepare_write`, Write Authorization, `record_run`, `close_task`, 사용자 판단, 최종 수락, 잔여 위험 수락. [Core Model 참조](core-model.md)를 봅니다.
- MCP method 계약, 공유 스키마, 공개 오류, 멱등성, 재실행, `allowed` / `blocked` 응답 형태. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- SQLite DDL, Runtime Home 배치, 저장소 잠금, artifact row, hash, 마이그레이션 규칙, Storage가 담당하는 JSON. [Storage](storage.md)를 봅니다.
- Product Repository / Harness Server / Harness Runtime Home 분리, Projection 권한, 아티팩트 경계, 복구 경계. [런타임 경계 참조](runtime-boundaries.md)를 봅니다.
- 커넥터 `capability_profile` 필드, 생성된 manifest, 대체 동작, 접점별 메모. [Agent 통합 참조](agent-integration.md)를 봅니다.
- 활성 참조 범위로서의 운영자 명령 의미 또는 진단 출력. 향후 운영 후보는 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다.
- 실행 가능한 증명, fixture assertion, runner 동작, 적합성 통과/실패. [적합성 참조](conformance.md)를 봅니다.

## 2. 현재 MVP 보장 수준

<a id="정직한-guarantee-display"></a>

현재 MVP 보장 수준은 기본적으로 협력형입니다. 활성 기준 접점이 관련 사실을 정직하게 관찰할 수 있는 곳에서만 제한된 탐지적 동작을 말할 수 있습니다. 활성 기준 접점은 등록된 `capability_profile`로 표현됩니다. 이 프로필은 보장 수준 표시와 역량 차단 사유를 제한하지만 쓰기 호환성이나 Write Authorization을 만들지는 않습니다.

현재 MVP 값 집합에서 `GuaranteeDisplay.level`의 기본 값은 `cooperative`와 `detective`뿐입니다. `preventive`와 `isolated`는 profile-gated 표시 값 이름일 뿐입니다. 대상 동작이나 경계에 대한 승격된 프로필과 증명이 있어야 사용할 수 있으며, 현재 MVP의 기본 보장이 아닙니다.

`allowed`는 현재 하네스 상태, 담당 기록, 활성 접점 역량과 호환된다는 뜻입니다. 운영체제가 그 동작을 허용한다는 뜻이 아닙니다. `blocked`는 하네스 프로토콜, 상태, 담당 기록, 역량 확인상 그 경로가 진행되면 안 된다는 뜻입니다. 실행 전에 프로세스가 물리적으로 멈췄다는 뜻이 아닙니다.

기준 `capability_profile`에는 기본 예방형 또는 격리형 태세가 없습니다. 프로필 지원은 명시되어야 합니다. 에이전트와 커넥터는 사용자 의도에서 더 강한 보장 라벨을 추론하면 안 됩니다. `pre_tool_blocking_supported=false`이면 MVP는 제품/런타임/코드 쓰기에 `preventive`를 표시하면 안 됩니다. `isolation_supported=false`이면 MVP는 `isolated`를 표시하거나 보안 격리 경계를 암시하면 안 됩니다. 사용자가 더 강한 안전을 요청했거나 guard/freeze/careful-mode 라벨이 있거나 향후 프로필 아이디어가 있다는 이유만으로 프로필이 승격되지 않습니다.

Write Authorization은 호환되는 non-dry-run `prepare_write` 경로만 만들고 compatible `record_run`이 소비하는 한 번만 쓰는 협력형 하네스 기록입니다. 하네스 기록/확인일 뿐이며 운영체제 권한, 샌드박스, 변조 방지 강제, 물리적 도구 실행 전 차단, 격리가 아닙니다.

기준 `reference-local-mcp` 프로필에서 Write Authorization과 제품 쓰기 Run 호환성은 경로 수준입니다. 이 프로필은 기본적으로 협력형이고, 관찰된 변경 경로에 대해서만 제한된 탐지형 지원을 가집니다. 명령 관찰, 네트워크 관찰, 비밀값 접근 관찰, 접점 자체 아티팩트 캡처, 도구 실행 전 차단, 격리는 없습니다. 담당 경로가 승인한 아티팩트 등록을 통해 수동 아티팩트 첨부가 가능할 수는 있지만, 그것은 커넥터 아티팩트 캡처가 아니며 아티팩트가 어떻게 만들어졌는지 검증하지 않습니다.

로컬 접근 태세도 하네스 호환성 사실입니다. `registered_local`은 API 담당 문서가 요청한 접근 분류에 대해 호출자/전송 경로를 등록된 로컬 접점과 맞는 것으로 다룰 수 있다는 뜻입니다. OS 계정, 편집기, shell, package manager, 임의 로컬 프로세스가 제한된다는 뜻이 아닙니다. `unavailable`, `mismatch`, `revoked` 태세는 공개 API 오류와 표시해도 안전한 진단으로 라우팅됩니다. 더 강한 보안 경계의 증거가 아닙니다.

문서 점검, fixture 초안, 예시, 적합성 계획은 런타임 보안 동작을 증명하지 않습니다. 이런 자료는 문구와 향후 계약 의도를 확인할 수 있을 뿐입니다. 예방형 주장이나 격리 주장은 대상 동작 또는 경계에 대해 구현된 메커니즘과 증명이 있어야 합니다.

## 3. 명시적 비보장

현재 MVP는 다음을 제공하지 않습니다.

- 운영체제 수준 권한 제어(`OS-level permission control`)
- 임의 도구 샌드박스(`arbitrary-tool sandboxing`)
- 변조 방지 또는 변조 불가능 저장소(`tamper-proof storage`)
- 기본 도구 실행 전 차단(`default pre-tool blocking`)
- 기준 참조 프로필의 접점 자체 아티팩트 캡처
- 보안 격리(`security isolation`)

하네스가 blocker를 반환하거나, Write Authorization을 기록하거나, artifact hash를 확인하거나, 최신이 아닌 맥락을 탐지하거나, 역량 불일치를 보고하거나, Projection을 stale로 표시해도 이 명시적 비보장은 유지됩니다. 그런 결과는 협력형 또는 탐지적 동작일 수 있습니다. 다른 담당 문서가 정확한 메커니즘과 정확한 동작을 문서화하고 증명하지 않는 한 예방적 또는 격리형이 아닙니다.

MVP는 로컬 파일이 로컬이라는 이유만으로 신뢰 가능하다고 주장하지 않습니다. 런타임 경계가 OS 수준 격리 경계라고 주장하지 않습니다. MCP 도달 가능성을 권한으로 취급하지 않습니다. 대화나 생성된 Markdown이 권한을 만들 수 있다고 주장하지 않습니다. 구현 전 적합성 fixture 문구가 런타임 보안 동작을 증명한다고 주장하지 않습니다.

## 4. 자산

보안에 민감한 자산은 다음과 같습니다.

| 자산 | 중요한 이유 | 담당 경계 |
|---|---|---|
| Core가 소유한 상태 | 작업 범위, 사용자 소유 판단, 증거 참조, 쓰기 호환성, 닫기 준비 상태, 잔여 위험 상태에 대한 하네스 권한을 정의합니다. | 의미는 [Core Model 참조](core-model.md)가 담당하고, 지속 보관은 [Storage](storage.md)가 담당합니다. |
| `state.sqlite`와 Runtime Home 메타데이터 | 프로젝트 등록, 현재 상태, 이벤트 이력, 접점, Write Authorization, 아티팩트 메타데이터를 지속 보관합니다. | [Storage](storage.md)가 배치와 방어적 확인을 담당합니다. 저장소는 변조 방지 저장소가 아닙니다. |
| Write Authorization과 `AuthorizedAttemptScope` | 한 번의 호환된 쓰기 시도와 한 번의 호환된 소비를 기록합니다. | 정확한 동작은 [Core Model 참조](core-model.md#write-authorization), [MVP API](api/mvp-api.md), [Storage](storage.md)가 소유합니다. |
| `user_judgment` 기록 | 사용자 소유의 제품, 기술, 범위, 민감 동작, 최종 수락, 잔여 위험, 취소 판단을 보존하고, later/reserved QA/검증 위험 경로가 승격되더라도 서로 구분합니다. | 정확한 경로는 Core/API 담당 문서가 정합니다. 대화 문구는 담당 경로로 기록되기 전까지 입력입니다. |
| 아티팩트 참조와 증거 메타데이터 | 원시 경로나 등록되지 않은 바이트를 신뢰하지 않고 증거와 닫기 준비 상태 주장을 뒷받침합니다. | 정확한 처리는 [API Schema Core](api/schema-core.md), [Storage](storage.md), [런타임 경계 참조](runtime-boundaries.md)가 소유합니다. |
| 커넥터 `capability_profile` | 활성 접점의 보장 수준 표시, 역량 차단 사유, 대체 동작을 제한합니다. | 필드와 갱신 규칙은 [Agent 통합 참조](agent-integration.md)가 담당합니다. |
| Product Repository 파일과 생성된 Projection | 에이전트와 사용자에게 영향을 줄 수 있지만, 하네스 관점에서는 입력 또는 파생 표시입니다. | 표시 경계는 [런타임 경계 참조](runtime-boundaries.md)와 [Projection과 Template 참조](projection-and-templates.md)가 담당합니다. |
| Secret, token, PII, 표시해도 안전한 핸들 | 아티팩트, 로그, prompt, Projection, manifest, export를 통해 누출될 수 있습니다. | 담당 경로는 redaction, omission, blocked-payload metadata, 표시해도 안전한 핸들을 우선해야 합니다. |

## 5. 신뢰 경계

| 경계 | 보안 태세 |
|---|---|
| 사용자 대화와 에이전트 접점 | 대화, 기억, 붙여넣은 텍스트, 승인처럼 보이는 문구는 입력으로 취급합니다. 사용자 소유 판단은 문서화된 `user_judgment` / 담당 경로를 통해서만 권한이 됩니다. |
| Product Repository | 제품 파일, 저장소 규칙, 생성된 Markdown, Projection은 제품 작업, 입력, 파생 표시입니다. 가까이에 있거나 repo에 있다는 이유로 하네스 운영 권한이 되지 않습니다. |
| Harness Server / Installation | 향후 로컬 제어 프로그램이 하네스 권한 확인을 실행합니다. 일반 운영체제 샌드박스나 임의 도구 권한 시스템이 아닙니다. |
| Harness Runtime Home | Runtime Home은 향후 동작을 위해 Core가 소유한 기록과 artifact를 저장합니다. 넓은 로컬 읽기/쓰기 접근은 변조와 기밀성 위험으로 취급합니다. 변조 불가능 저장소를 주장하지 않습니다. |
| MCP / 로컬 API 접점 | 도달 가능성은 권한 부여가 아닙니다. Core/API 검증, project/task/surface 호환성, idempotency, expected state version, 로컬 접근 태세, 접점 상태, active capability가 계속 적용됩니다. |
| 커넥터가 생성한 파일 | 생성된 manifest, snippet, prompt, adapter file은 drift되거나 편집될 수 있습니다. 담당 경로와 현재 `capability_profile` 없이는 권한을 만들지 않습니다. |
| 아티팩트 저장소 | 아티팩트 바이트는 등록되고, 담당 기록과 연결되고, 필요한 integrity/redaction metadata가 확인되기 전까지 신뢰하지 않습니다. |
| 외부 도구, 명령, 네트워크 호출 | 로컬 실행은 파일을 바꾸거나 데이터를 누출하거나 외부 시스템에 영향을 줄 수 있습니다. 협력형 하네스 확인은 기본적으로 그런 도구를 물리적으로 제한하지 않습니다. |

## 6. 위협/통제 요약

이 요약은 활성 위협 범주만 이름 붙입니다. MVP 문서를 전체 향후 위협 목록으로 만들지 않습니다.

| 위협 범주 | 흔한 경로 | MVP 통제 태세 |
|---|---|---|
| 권한 위조 | 대화, 생성된 Markdown, 호출자 주장, 오래된 Projection이 작업을 민감 동작 승인, 검증, 최종 수락, 닫기한 것처럼 꾸밉니다. | 권한은 Core가 소유한 기록으로 라우팅합니다. MCP/Core 권한을 사용할 수 없으면 실패하거나 보류합니다. |
| 범위 밖 쓰기 | 제품 파일 경로나 민감 범주가 active Change Unit, 사용자 판단, 민감 동작 승인, 저장된 `AuthorizedAttemptScope`를 벗어납니다. 명령, 네트워크, 비밀값 효과는 향후 프로필이 관찰을 승격하기 전까지 별도의 역량과 민감 동작 문제입니다. | 협력형 `prepare_write`, 한 번만 쓰는 Write Authorization, compatible `record_run`, 접점이 관찰할 수 있는 변경 경로 탐지를 사용합니다. 관찰할 수 없는 명령, 네트워크, 비밀값 보장을 요구하는 요청은 거절하거나 차단합니다. |
| 최신이 아닌 맥락 또는 replay | 오래된 상태 문구, 승인, Projection, 기준선, evaluator bundle, 캐시된 상태가 현재 작업을 이끕니다. | 입력에 의존하기 전에 현재 state version, idempotency, freshness, owner-record compatibility를 확인합니다. |
| 아티팩트 또는 증거 변조 | 바이트, 경로, hash, metadata가 바뀌었거나 stale, missing, redacted, blocked, unrelated 상태입니다. | 등록, 무결성, redaction, 담당 관계 확인이 통과할 때까지 evidence를 insufficient 또는 blocked로 취급합니다. |
| Secret 또는 PII 노출 | Log, screenshot, trace, prompt, artifact, Projection, manifest, export가 sensitive value를 담습니다. | Redaction, omission, blocked-payload notice, 표시해도 안전한 핸들, 담당 경로가 승인한 증거 요약을 우선합니다. |
| 역량 과장 주장 | 접점이 실제 `capability_profile`보다 강한 차단, 캡처, 격리, MCP 도달 가능성을 주장합니다. | 표시 보장 수준을 낮추고, 주장을 unverified로 표시하고, capability blocker/error를 반환하거나, 지시로 보류합니다. |

## 7. 협력형 동작

협력형 동작은 연결된 에이전트나 접점이 문서화된 절차를 따를 때 하네스가 안내, 기록, 비교, 또는 하네스 상태 변경 경로 거부를 할 수 있다는 뜻입니다. 강한 보안 경계가 아닙니다.

현재 MVP 계획의 협력형 동작 예시는 다음과 같습니다.

- 접점이 제품 파일 쓰기 전에 `prepare_write`를 호출합니다.
- 범위, 판단, 민감 동작 승인, state version, capability가 호환되지 않으면 Core가 Write Authorization 생성을 거부합니다.
- 호환되는 non-dry-run `prepare_write`가 소비 가능한 Write Authorization 하나를 만듭니다.
- `record_run`은 접점이 정직하게 관찰할 수 있는 범위에서 관찰된 변경 경로가 호환될 때만 그 Write Authorization을 소비합니다.
- MCP/Core 권한이나 필요한 역량을 사용할 수 없으면 에이전트가 제품/런타임/코드 쓰기를 지시로 보류합니다.
- 생성된 상태 텍스트는 하네스가 확인할 수 있는 것과 확인할 수 없는 것을 사용자에게 말합니다.

협력형 동작은 정직한 에이전트를 하네스와 맞출 수 있습니다. 하지만 임의 로컬 프로세스, 편집기, shell, package manager, 네트워크 사용 도구를 기본적으로 멈추지는 않습니다.

## 8. 탐지적 동작

탐지적 동작은 동작 뒤 또는 관련 사실을 관찰할 수 있게 된 뒤 하네스가 불일치를 감지, 기록, 보고할 수 있다는 뜻입니다. 사후 확인이지 예방이 아닙니다.

현재 MVP 계획의 탐지적 동작 예시는 다음과 같습니다.

- 접점이 지원할 때 run 이후 변경 경로 비교
- 담당 경로가 요구하는 등록된 `ArtifactRef`의 `sha256`, `size_bytes`, `content_type`, ownership, availability, redaction, omission, blocked-payload check. 이런 확인은 접점 자체 아티팩트 캡처가 아닙니다.
- 최신이 아닌 state, Projection, 커넥터 프로필, baseline, retrieved-context 보고
- 기능 불일치 또는 지원되지 않는 접점 보고
- 담당 경로가 지원하는 generated-file 또는 managed-block drift 보고

탐지적 동작은 무엇을 관찰했고 무엇이 아직 미확인인지 말해야 합니다. 기준 제품 쓰기 호환성에서 `detective` 라벨은 변경 경로 관찰이 있을 때만 정당화됩니다. 지원되지 않는 명령, 네트워크, 비밀값, 아티팩트 캡처, 차단, 격리, 외부 시스템 효과는 근처의 하네스 확인이 성공했다는 이유만으로 통과로 보고하면 안 됩니다.

## 9. 예방형 주장 규칙

예방형 주장은 아래 조건이 모두 참일 때만 허용됩니다.

- 정확한 대상 동작을 이름 붙입니다.
- 후크, 래퍼, 권한 계층, 정책 엔진, 사이드카 또는 이에 준하는 실행 전 차단 메커니즘을 이름 붙입니다.
- 그 동작의 담당 문서가 동작과 대체 동작을 정의합니다.
- 정확한 경로에 대한 실행 가능한 증명이 있습니다.
- 표시되는 `capability_profile`이 그 동작에 대해 `preventive`를 지원합니다.

현재 MVP에는 기본 예방형 주장이 없습니다. 위의 정확한 예방형 경로가 없다면 `prepare_write`, Write Authorization, `allowed`, `blocked`, 파일 잠금, hash, 상태 카드, Projection, 문서 점검, fixture 초안, guard 문구, freeze 문구, careful-mode 문구를 실행 전 차단으로 설명하지 않습니다.

## 10. 격리 주장 규칙

격리 주장은 그 주장에 맞는 보안 경계를 이름 붙이고 증명했을 때만 허용됩니다. 유효한 주장은 무엇을 무엇으로부터 격리하는지, 어떤 메커니즘을 쓰는지, 어떤 동작에 적용되는지, 어떤 담당 문서/증명 경로 아래에 있는지 말해야 합니다.

분리된 worktree, 새로운 세션, 새로운 evaluator bundle, 별도 프로세스는 최신성, 검증 독립성, 영향 범위 축소를 도울 수 있습니다. 하지만 자동으로 운영체제 샌드박싱, 권한 격리, 변조 불가능 저장소, 보안 격리가 되지는 않습니다. 현재 MVP에는 기본 보안 격리 주장이 없습니다.

파일이 로컬이라는 이유, bundle이 최신이라는 이유, 커넥터에 친근한 mode name이 있다는 이유, tool이 다른 directory에서 실행된다는 이유, 문서가 조심하라고 말한다는 이유만으로 `isolated`를 쓰지 않습니다.

## 11. 담당 문서 간 확인

보안 주장을 추가하거나 받아들이기 전에 관련 담당 문서를 확인합니다.

| 질문 | 확인할 담당 문서 |
|---|---|
| 하네스 상태 전이, gate, judgment, write, run, close, waiver, residual-risk rule인가요? | [Core Model 참조](core-model.md) |
| 공개 API method, 응답 필드, 오류 코드, 멱등성, 재실행, state-version, `allowed`, `blocked` 동작인가요? | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| Runtime Home 배치, `state.sqlite`, artifact row, 잠금, hash, 마이그레이션, 저장소 검증인가요? | [Storage](storage.md) |
| Product Repository / Harness Server / Harness Runtime Home 분리, Projection 권한, 아티팩트 경계, 복구 경계인가요? | [런타임 경계 참조](runtime-boundaries.md) |
| 접점 `capability_profile`, 접점의 MCP 사용 가능성, 생성된 manifest, 대체 동작, 맥락 주입/가져오기, 보장 수준 표시인가요? | [Agent 통합 참조](agent-integration.md) |
| 운영자 진단, recovery, export, artifact check, conformance entrypoint 후보인가요? | [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)를 봅니다. Runtime conformance 증명은 [적합성 참조](conformance.md)에 남습니다. |
| 런타임 증명, fixture assertion 동작, 통과/실패 표현인가요? | [적합성 참조](conformance.md) |

담당 문서가 더 강한 통제를 정의하고 증명하지 않으면 협력형 또는 탐지적 표현을 사용합니다. 또는 주장을 지원되지 않음으로 표시하거나 명시적 비보장을 적습니다. 향후 통제, later 운영 아이디어, 문서 점검, 적합성 계획 언어를 활성 MVP 보안 보장으로 바꾸지 않습니다.
