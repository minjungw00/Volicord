# 런타임 경계 참조

이 참조 문서는 향후 Harness Server 계획에서 사용하는 활성 런타임 경계를 작게 정의합니다. 어느 공간이 제품 파일을 소유하는지, 어느 공간이 하네스 권한 확인을 실행하는지, 어느 공간이 Core가 소유한 기준 상태를 지속 보관하는지, 무엇이 파생 표시나 아티팩트 보조 자료로 남는지 설명합니다.

런타임 경계는 권한 경계와 저장 위치 경계이지 OS 수준 격리 경계가 아닙니다. 이 경계는 누가 하네스 권한을 만들 수 있는지, Core가 소유한 기록과 아티팩트를 어디에 보관하는지, 무엇이 파생 표시로 남는지를 나눕니다. 프로세스 격리, 샌드박스, 권한 강제, 임의 도구 통제, 변조 방지 저장소, 보안 격리를 뜻하지 않습니다.

이 문서는 원천 문서입니다. 지금 이 저장소에는 Harness Server/runtime 구현, Harness Runtime Home, 생성된 Projection 시스템, 적합성 실행기, 런타임 데이터가 없습니다. 현재 저장소 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

정확한 계약은 [Core Model 참조](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Projection과 Template 참조](projection-and-templates.md), [보안 참조](security.md), [Agent 통합 참조](agent-integration.md)를 사용합니다. 이 문서는 작은 경계 모델만 담당합니다.

## 1. Product Repository

Product Repository는 사용자의 실제 제품 작업 공간입니다. 제품 소스 파일, 테스트, 저장소 수준 에이전트 규칙, 제품 문서가 여기에 있습니다. 제품 작업은 사용자가 선택한 일반 도구와 에이전트 행동을 통해 이 공간에서 일어납니다.

Product Repository의 파일은 하네스 기준 상태가 아닙니다. 제품 파일은 입력이거나 변경 대상이거나 제품 내용에 대해 제품 저장소가 소유하는 사실일 수 있습니다. 하지만 하네스 근처에 있다는 이유만으로 하네스 운영 권한이 되지는 않습니다.

활성 담당 경로가 지원하면 Product Repository에는 생성된 읽기용 출력이 있을 수 있습니다. Projection, template, status card, 작은 증거 요약, 닫기 준비 상태 보기, managed Markdown block 같은 출력입니다. 이 파일은 사람과 에이전트가 작업을 읽도록 돕습니다. Core가 소유한 상태가 아니라 파생 표시입니다. 사람이 편집할 수 있는 제안 영역은 Core 상태 변경 action이 수락하기 전까지 입력일 뿐입니다.

이 문서 저장소도 사용자의 Product Repository가 아닙니다. 이 저장소는 문서 전용 계획 저장소입니다. 문서 수락과 별도의 구현 계획 준비 결정 이후에 향후 Harness Server source repository가 되는 것을 목표로 합니다.

<a id="2-harness-server--installation"></a>

## 2. Harness Server / Installation

Harness Server / Installation은 향후 로컬 하네스 프로그램 경계입니다. 로컬 tool/resource 호출을 받고, Core가 소유한 권한 확인을 실행하며, Core를 통해 상태 변경 action을 기록합니다. 활성 담당 경로가 요구할 때 validator를 호출하고, artifact를 등록하며, Projection 지원이 범위에 있을 때 파생 표시를 렌더링합니다.

MVP 경계는 여러 서비스나 자세한 프로세스 분리를 요구하지 않습니다. 하나의 로컬 프로세스와도 호환됩니다. 중요한 것은 권한 경계와 저장 위치 경계를 분명히 유지하는 것입니다. 호출자는 요청하고, Core는 호환되는 상태 변경을 평가하고 기록하며, 저장소는 지속 보관하고, 표시는 기록된 상태에서 파생됩니다.

Harness Server / Installation은 Product Repository도 아니고 Harness Runtime Home도 아닙니다. 제품 파일을 읽을 수 있고, 사용자가 선택한 작업 접점과 문서화된 협력형 하네스 확인을 통해서만 제품 파일을 쓸 수 있습니다. 하네스 기록은 [Storage](storage.md)가 담당하는 Runtime Home 저장 경로를 통해서만 지속 보관합니다.

## 3. Harness Runtime Home

Harness Runtime Home은 사용자별 또는 설치별 운영 데이터 공간입니다. 짧게 Runtime Home이라고도 부릅니다. 기준 위치와 정확한 배치는 [Storage](storage.md)가 담당합니다. 향후 일반적인 내용에는 프로젝트 등록 데이터, 프로젝트 설정, `state.sqlite`, 아티팩트 저장소가 포함됩니다.

하네스 기준 상태는 Runtime Home 저장소에 지속 보관되는 Core 소유 현재 기록에 있습니다. `state.sqlite.task_events`는 상태 저장소 안의 감사와 순서 이력을 기록합니다. 별도 표시 로그도 아니고 현재 기록을 대체하지도 않습니다.

Harness Runtime Home은 대화 기록이 사라지거나 Product Repository의 Projection이 오래되어도 하네스 운영 의미를 복구할 수 있을 만큼 충분해야 합니다. Projection 지원이 있으면 Product Repository의 표시는 state records와 artifact refs에서 다시 만들 수 있습니다. 표시는 그 기록을 대체할 수 없습니다.

Runtime Home의 파일은 비공개 로컬 제어 데이터로 취급해야 합니다. 하지만 하네스는 운영체제 권한을 강제하거나, 파일을 변조 방지 상태로 만들거나, 임의의 로컬 도구로부터 파일을 스스로 격리한다고 주장하지 않습니다.

## 4. Core 변경 권한

하네스 기준 상태 변경은 Core 상태 변경 경로에서만 일어납니다. Core는 범위, 사용자 소유 판단, 증거와 아티팩트 참조, 검증과 QA 기대, 최종 수락, 잔여 위험 상태, 닫기 준비 상태에 대한 하네스 기록 권한을 소유합니다.

에이전트, MCP caller, CLI text, operator output, 제품 파일, Projection Markdown, template, status card, artifact bytes, chat transcript는 그 자체로 기준 상태를 변경하지 않습니다. 관련 담당 경로가 받아들일 때만 입력 또는 증거 후보가 될 수 있습니다.

`prepare_write`, Write Authorization, `record_run`, `close_task`는 Core/API가 소유하는 계약입니다. Write Authorization은 협력형 하네스 기록과 확인입니다. OS 권한, 샌드박스 강제, 변조 방지 보호, 실행 전 물리적 차단, 보안 격리 메커니즘이 아닙니다.

정확한 상태 전이, gate 영향, row 경계, idempotency 동작, 응답 모양은 [Core Model 참조](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)에 남습니다.

## 5. Projection 파생 경계

Projection, template, status card, generated Markdown, read-only status resource는 파생 표시입니다. Core가 소유한 state records와 등록된 artifact refs에서 렌더링됩니다. 최신성, 실패, blocker, next-action 정보를 담을 수 있지만, 그 정보도 담당 기록을 보여주는 표시일 뿐 두 번째 권한 근거가 아닙니다.

Projection은 stale, missing, failed 상태일 수 있고 사람이 직접 고쳤을 수도 있습니다. 이런 조건은 그 자체로 하네스 기준 상태를 바꾸지 않습니다. 오래되었거나 실패한 Projection은 보이는 blocker나 freshness warning을 만들 수 있습니다. 하지만 Core state를 되돌리거나, 증거를 충족하거나, 검증 또는 QA를 통과시키거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, Task를 닫지 않습니다.

Managed generated area는 계속 파생 표시입니다. 사람이 편집할 수 있는 영역은 제안 입력입니다. 제안은 Core 소유 경로가 상태 변경 action으로 수락한 뒤에만 하네스 상태에 영향을 줍니다.

## 6. Artifact 저장 경계

아티팩트 경계는 지속 보관되는 증거 보조 자료와 기준 상태를 분리합니다. 아티팩트 저장소는 등록된 증거 바이트 또는 안전한 메타데이터 알림을 보관할 수 있습니다. 하네스에서 권한을 갖는 의미는 등록된 `ArtifactRef`, 담당 관계, 무결성 metadata, redaction/availability 상태, 관련 Core 기록에서 나옵니다.

원시 경로, 호출자 주장, 대화 텍스트, Markdown 문장, 등록되지 않은 파일, 담당 관계가 없는 아티팩트 바이트는 그 자체로 충분한 증거가 아닙니다. 필요한 artifact metadata가 missing, stale, redacted, unavailable, blocked 상태이거나 무결성 확인에 실패하면 Core 소유 evidence와 닫기 준비 상태 기록은 그 조건을 반영해야 합니다.

Artifact는 evidence, verification, QA, 최종 수락 표시, residual-risk visibility, close-readiness display를 뒷받침할 수 있습니다. 그러나 Core가 요구하는 별도 담당 기록과 사용자 소유 판단 없이 성공을 증명하거나, 작업을 승인하거나, 위험을 수락하거나, 작업을 닫지 않습니다.

## 7. Recovery 경계

Recovery는 같은 권한 모델 안에 머뭅니다. Recovery는 Runtime Home state records, `state.sqlite.task_events`, artifact refs, integrity metadata, Projection freshness fact를 사용해 무엇이 stale, interrupted, missing, inconsistent인지 분류할 수 있습니다.

Recovery는 파생 표시를 다시 만들거나, 담당 경로를 통해 artifact를 다시 scan하거나 등록하거나, dependent evidence나 view를 stale 또는 blocked로 표시하거나, 담당 계약이 허용하는 경우 최신이 아닌 work record를 interrupted로 처리할 수 있습니다. 필요한 사용자 판단이나 Core action으로 라우팅할 수도 있습니다. 두 번째 상태 모델을 만들면 안 됩니다.

Recovery는 chat, generated Markdown, stale Projection, export text, operator console output, staging path, recovery artifact에서 성공한 구현을 추론할 수 없습니다. Recovery 자체로 evidence를 충족하거나, verification 또는 QA를 통과시키거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, Task를 닫지 않습니다.

## 8. 현재 MVP가 격리하지 않는 것

현재 MVP 경계는 향후 담당 문서가 이름 붙인 동작에 대해 더 강한 메커니즘을 승격하고 증명하기 전까지 협력형과 탐지형 수준입니다. 운영체제 수준 권한, 임의 도구 샌드박스, 권한 강제, 변조 방지 저장소, 보편적 도구 실행 전 차단, 보안 격리를 주장하지 않습니다. `preventive`와 `isolated`는 현재 MVP 기본값이 아니며, 관련 Reference 담당 문서가 관리하는 profile-gated 표시 값으로 남습니다.

로컬 전용 MCP 도달 가능성은 권한 부여가 아닙니다. 닿을 수 있는 caller도 valid Core/API state, project/task/surface compatibility, state-version compatibility, active surface capability가 필요합니다. `allowed`는 하네스 상태와 활성 surface capability에 맞는다는 뜻입니다. `blocked`는 하네스 담당 경로 또는 capability check상 진행하면 안 된다는 뜻입니다. 증명된 preventive mechanism이 정확한 대상 동작을 이름 붙이지 않는 한 두 단어 모두 물리적 차단을 뜻하지 않습니다.

Surface name, connector recipe, friendly mode label, Projection, template, status card, artifact, documentation check는 보장 수준을 올려 주지 않습니다. 더 강한 preventive 또는 isolated claim은 관련 Reference 담당 문서에서 문서화한 mechanism, 대상 동작, 담당 문서, proof path가 필요합니다.
