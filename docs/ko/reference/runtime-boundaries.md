# 런타임 경계 참조

이 참조 문서는 향후 Harness Server 계획에서 사용하는 활성 런타임 경계를 작게 정의합니다. 어느 공간이 제품 파일을 소유하는지, 어느 공간이 하네스 권한 확인을 실행하는지, 어느 공간이 Core가 소유한 기준 상태를 지속 보관하는지, 무엇이 파생 표시나 아티팩트 보조 자료로 남는지 설명합니다.

이 문서는 source 문서입니다. 지금 이 저장소에는 Harness Server/runtime 구현, Harness Runtime Home, 생성된 projection 시스템, conformance runner, 런타임 데이터가 없습니다. 현재 저장소 단계와 handoff 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

정확한 계약은 [Core Model 참조](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Projection과 Template 참조](projection-and-templates.md), [보안 참조](security.md), [Agent 통합 참조](agent-integration.md)를 사용합니다. 이 문서는 작은 경계 모델만 담당합니다.

## 1. Product Repository

Product Repository는 사용자의 실제 제품 작업 공간입니다. 제품 소스 파일, tests, repository-level agent rules, 제품 문서가 여기에 있습니다. 제품 작업은 사용자가 선택한 일반 도구와 agent 행동을 통해 이 공간에서 일어납니다.

Product Repository의 파일은 하네스 기준 상태가 아닙니다. 제품 파일은 입력이거나 변경 대상이거나 제품 내용에 대해 제품 저장소가 소유하는 사실일 수 있습니다. 하지만 하네스 근처에 있다는 이유만으로 하네스 운영 권한이 되지는 않습니다.

활성 profile이 지원하면 Product Repository에는 생성된 읽기용 출력이 있을 수 있습니다. Projection, template, status card, 작은 증거 요약, 닫기 준비 상태 보기, managed Markdown block 같은 것입니다. 이 파일은 사람과 agent가 작업을 읽도록 돕습니다. Core가 소유한 상태가 아니라 파생 표시입니다. 사람이 편집할 수 있는 proposal 영역은 Core state-changing action이 수락하기 전까지 입력일 뿐입니다.

이 문서 저장소도 사용자의 Product Repository가 아닙니다. 이 저장소는 문서 전용 계획 저장소입니다. 문서 수락과 별도의 implementation-planning readiness 결정 이후에 향후 Harness Server source repository가 되는 것을 목표로 합니다.

<a id="2-harness-server--installation"></a>

## 2. Harness Server / Installation

Harness Server / Installation은 향후 로컬 하네스 프로그램 경계입니다. 로컬 tool/resource 호출을 받고, Core가 소유한 권한 확인을 실행하며, Core를 통해 상태 변경 action을 기록합니다. 활성 profile이 요구할 때 validator를 호출하고, artifact를 등록하며, projection support가 범위에 있을 때 파생 표시를 렌더링합니다.

MVP 경계는 여러 서비스나 자세한 process 분리를 요구하지 않습니다. 하나의 로컬 프로세스와도 호환됩니다. 중요한 것은 권한 경계입니다. 호출자는 요청하고, Core는 판단하고 기록하며, 저장소는 지속 보관하고, 표시는 기록된 상태에서 파생됩니다.

Harness Server / Installation은 Product Repository도 아니고 Harness Runtime Home도 아닙니다. 제품 파일을 읽을 수 있고, 사용자가 선택한 작업 접점과 문서화된 협력형 하네스 확인을 통해서만 제품 파일을 쓸 수 있습니다. 하네스 기록은 [Storage](storage.md)가 담당하는 Runtime Home storage path를 통해서만 지속 보관합니다.

## 3. Harness Runtime Home

Harness Runtime Home은 사용자별 또는 설치별 운영 데이터 공간입니다. 짧게 Runtime Home이라고도 부릅니다. Reference location과 정확한 layout은 [Storage](storage.md)가 담당합니다. 향후 일반적인 내용에는 project registration data, project configuration, `state.sqlite`, artifact storage가 포함됩니다.

하네스 기준 상태는 Runtime Home storage에 지속 보관되는 Core-owned current records에 있습니다. `state.sqlite.task_events`는 state store 안의 audit와 ordering history를 기록합니다. 별도의 display log도 아니고 current records를 대체하지도 않습니다.

Harness Runtime Home은 대화 기록이 사라지거나 Product Repository의 projection이 오래되어도 하네스 운영 의미를 복구할 수 있을 만큼 충분해야 합니다. Projection support가 있으면 Product Repository의 표시는 state records와 artifact refs에서 다시 만들 수 있습니다. 표시는 그 기록을 대체할 수 없습니다.

Runtime Home의 파일은 private local control data로 취급해야 합니다. 하지만 하네스는 operating-system permission을 강제하거나, 파일을 tamper-proof로 만들거나, 임의의 local tool로부터 파일을 스스로 격리한다고 주장하지 않습니다.

## 4. Core 변경 권한

하네스 기준 상태 변경은 Core 상태 변경 경로에서만 일어납니다. Core는 범위, 사용자 소유 판단, 증거와 아티팩트 참조, 검증과 QA 기대, 최종 수락, 잔여 위험 상태, 닫기 준비 상태에 대한 하네스 기록을 만들거나 업데이트할 권한을 소유합니다.

Agent, MCP caller, CLI text, operator output, 제품 파일, projection Markdown, template, status card, artifact bytes, chat transcript는 그 자체로 기준 상태를 변경하지 않습니다. 관련 owner 경로가 받아들일 때만 입력 또는 증거 후보가 될 수 있습니다.

`prepare_write`, Write Authorization, `record_run`, `close_task`는 Core/API가 소유하는 계약입니다. Write Authorization은 협력형 하네스 기록과 확인입니다. OS permission, sandbox enforcement, tamper-proof protection, 실행 전 물리적 차단, security-isolation mechanism이 아닙니다.

정확한 상태 전이, gate 영향, row 경계, idempotency 동작, response shape는 [Core Model 참조](core-model.md), [Storage](storage.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)에 남습니다.

## 5. Projection 파생 경계

Projection, template, status card, generated Markdown, read-only status resource는 파생 표시입니다. Core가 소유한 state records와 등록된 artifact refs에서 렌더링됩니다. 최신성, 실패, blocker, next-action 정보를 담을 수 있지만, 그 정보도 owner record를 보여주는 표시일 뿐 두 번째 권한 근거가 아닙니다.

Projection은 stale, missing, failed 상태일 수 있고 사람이 직접 고쳤을 수도 있습니다. 이런 조건은 그 자체로 하네스 기준 상태를 바꾸지 않습니다. 오래되었거나 실패한 projection은 보이는 blocker나 freshness warning을 만들 수 있습니다. 하지만 Core state를 roll back하거나, evidence를 충족하거나, verification 또는 QA를 통과시키거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, task를 close하지 않습니다.

Managed generated area는 계속 파생 표시입니다. Human-editable area는 proposal input입니다. Proposal은 Core-owned path가 상태 변경 action으로 수락한 뒤에만 하네스 상태에 영향을 줍니다.

## 6. Artifact 저장 경계

아티팩트 경계는 지속 보관되는 증거 보조 자료와 기준 상태를 분리합니다. Artifact store는 등록된 evidence bytes 또는 safe metadata notice를 보관할 수 있습니다. 하네스에서 권한을 갖는 의미는 등록된 `ArtifactRef`, owner relation, integrity metadata, redaction/availability state, 관련 Core records에서 나옵니다.

Raw path, caller claim, chat text, Markdown prose, 등록되지 않은 file, owner relation이 없는 artifact bytes는 그 자체로 충분한 증거가 아닙니다. Required artifact metadata가 missing, stale, redacted, unavailable, blocked 상태이거나 integrity check에 실패하면 Core-owned evidence와 닫기 준비 상태 기록은 그 조건을 반영해야 합니다.

Artifact는 evidence, verification, QA, acceptance review, residual-risk visibility, close-readiness display를 뒷받침할 수 있습니다. 그러나 Core가 요구하는 별도 owner records와 사용자 소유 판단 없이 성공을 증명하거나, 작업을 승인하거나, 위험을 수락하거나, 작업을 닫지 않습니다.

## 7. Recovery 경계

Recovery는 같은 권한 모델 안에 머뭅니다. Recovery는 Runtime Home state records, `state.sqlite.task_events`, artifact refs, integrity metadata, projection freshness fact를 사용해 무엇이 stale, interrupted, missing, inconsistent인지 분류할 수 있습니다.

Recovery는 파생 표시를 다시 만들거나, owner 경로를 통해 artifact를 다시 scan하거나 등록하거나, dependent evidence나 view를 stale 또는 blocked로 표시하거나, owner contract가 허용하는 경우 최신이 아닌 work record를 interrupted로 처리할 수 있습니다. 필요한 사용자 판단이나 Core action으로 라우팅할 수도 있습니다. 두 번째 상태 모델을 만들면 안 됩니다.

Recovery는 chat, generated Markdown, stale projection, export text, operator console output, staging path, recovery artifact에서 성공한 구현을 추론할 수 없습니다. Recovery 자체로 evidence를 충족하거나, verification 또는 QA를 통과시키거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, task를 close하지 않습니다.

## 8. 현재 MVP가 격리하지 않는 것

현재 MVP 경계는 future owner가 이름 붙인 operation에 대해 더 강한 profile을 승격하고 증명하기 전까지 cooperative와 detective 수준입니다. OS-level permission, arbitrary-tool sandboxing, permission enforcement, tamper-proof storage, universal pre-tool blocking, security isolation을 주장하지 않습니다.

Local-only MCP reachability는 authorization이 아닙니다. 도달 가능한 caller도 valid Core/API state, project/task/surface compatibility, state-version compatibility, active surface capability profile이 필요합니다. `allowed`는 하네스 상태와 활성 surface capability에 맞는다는 뜻입니다. `blocked`는 하네스 권한 경로나 capability check상 진행하면 안 된다는 뜻입니다. 증명된 preventive profile이 정확한 covered operation을 이름 붙이지 않는 한 두 단어 모두 물리적 차단을 뜻하지 않습니다.

Surface name, connector recipe, friendly mode label, projection, template, status card, artifact, documentation check는 guarantee level을 올려 주지 않습니다. 더 강한 preventive 또는 isolated claim은 관련 Reference owner에서 문서화한 mechanism, covered operation, owner, proof path가 필요합니다.
