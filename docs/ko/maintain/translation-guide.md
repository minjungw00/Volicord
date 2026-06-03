# 번역 가이드

## 이 문서로 할 수 있는 일

영어와 한국어 하네스 문서를 함께 고칠 때 이 가이드를 사용합니다.

이 문서는 이중 언어 문서 유지보수를 위한 Maintain 문서입니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 런타임/서버 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, 런타임 데이터, 제품 상태 변경을 허가하지 않으며, conformance pass/fail, 근거, QA, 작업 수락, 닫기 준비 상태, 구현 준비 상태를 정의하지 않습니다. 첫 실행 목표는 코어 권한 스모크(v0.1 Core Authority Smoke)이며, 커널 스모크(Kernel Smoke)는 좁은 미래 스모크 확인 작성 라벨입니다. 첫 사용자 가치 목표는 첫 사용자 가치 조각(v0.2 First User-Value Slice)입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 단계 분류의 정식 단계/팩 라벨입니다. v0.2 이후 이 단계들의 담당 문서 정의 profile을 완료하면 용어집이 정의하는 강화된 로컬 기준 목표(hardened local reference target)에 도달합니다. 이 강화된 로컬 기준 목표 자체는 상위 목표일 뿐, 추가 단계나 fixture profile, suite name이 아닙니다. v1+ Expansion은 담당 문서가 승격하고 증명하기 전까지 로드맵 범위에 남습니다.

## 이런 때 읽기

- 영어 또는 한국어 문서의 의미를 바꿀 때.
- 영어/한국어 의미 일치를 검토할 때.
- 한국어 문장이 영어 식별자를 그대로 유지해야 하는지, 자연스러운 한국어 문장으로 풀어야 하는지 판단할 때.

## 먼저 읽기

Owner boundary, 문서 유지보수 점검, 엄격한 계약이 참조 문서에 머문다는 규칙은 [문서 작성 가이드](authoring-guide.md)를 봅니다.

## 핵심 생각

영어 문서는 이중 언어 문서 세트의 기준 의미를 정의합니다. 한국어 문서는 그 의미를 보존하되, 영어 문장을 줄 단위로 따라 하지 않습니다.

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어 문서는 한국어 기술 문서답게 자연스럽게 읽혀야 하며, 공식 식별자(official identifier), 정확한 계약, 코드처럼 쓰이는 이름, 안정적인 제품 용어(product term)는 흔들리지 않아야 합니다.

사용자용 한국어에서는 자연스러운 공개 표현을 먼저 두고, 정확한 하네스 라벨이 도움이 될 때만 뒤에 붙입니다. 평소 설명에서는 `범위`, `범위 밖`, `판단 요청`, `근거`, `확인`, `검증`, `수동 QA`, `작업 수락`, `잔여 위험`, `닫기 막힘`, `다음 안전한 행동`을 우선합니다. `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Residual Risk`, `Manual QA`, `detached verification`, `Acceptance` 같은 라벨은 독자 친화 표현과 하네스 라벨이 모두 필요할 때만 괄호에 둡니다. 참고 문서의 한국어에서는 정확한 스키마 식별자, enum 값, field 이름, API 용어를 정밀도가 필요할 때 그대로 둘 수 있습니다.

## 사용자용 어휘 규칙

한국어 사용자용 문서는 자연스러운 공개 용어인 `작업`, `범위`, `범위 밖`, `판단`, `판단 요청`, `근거`, `확인`, `검증`, `수동 QA`, `작업 수락`, `잔여 위험`, `닫기 막힘`, `닫기 준비 상태`, `다음 안전한 행동`을 우선합니다. 안정적인 English identifier는 주로 참조 문서, schema/API 문맥, 정확한 record name, code-like string, anchor, 내부 구현 용어를 의도적으로 설명하는 표에서 보존합니다.

사용자용 문서에서 내부 구현 용어가 필요하면 쉬운 개념을 먼저 설명하고, 실제 경계, 막힘, 출처 참조, 참조 링크를 분명히 하는 경우에만 정확한 용어를 괄호로 덧붙입니다. 한국어 문장이 영어 명사 여러 개에 조사만 붙인 형태가 되지 않게 합니다.

- 사용자가 이해해야 하는 설명은 자연스러운 한국어를 우선합니다. 안정적인 English identifier는 필요할 때만 씁니다.
- 한국어 문장이 대부분 영어 명사이고 끝에 조사만 붙은 형태라면 다시 씁니다.
- 첫 사용에서 식별자 병기가 도움이 되면 한국어 표현 뒤에 괄호로 둡니다. 이후 같은 단락이나 사용자용 흐름에서는 한국어 표현만으로 충분한지 먼저 봅니다.
- 사용자에게 보이는 목록이나 카드에서는 `현재 근거 참조`, `빠진 근거`, `오래된 근거`, `이미 실행한 확인`처럼 자연스러운 한국어 묶음을 씁니다.
- Code block 안의 code, API method name, enum value, field name, file path, schema identifier, stable identifier는 정확히 유지합니다. 단, code block 안의 설명용 prose나 사용자 예시는 자연스러운 한국어로 고칠 수 있습니다.
- 사용자 예시를 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events` 같은 내부 용어로 시작하지 않습니다.
- 사용자가 `Discovery`, `Change Unit`, `Decision Packet`을 말해야만 동작이 시작되는 것처럼 쓰지 않습니다. `구현 전에 계획을 구체화해줘`, `범위가 커지면 먼저 알려줘`, `무엇을 결정해야 하는지 보여줘`처럼 평소 말을 예시로 둡니다.
- 사용자에게 판단을 물을 때는 `판단 요청` 또는 자연스러운 질문을 씁니다. `Decision Packet`은 선택적/내부 라벨로만 나중에 소개합니다.
- 어색한 혼합어 표현은 피합니다. 영어 명사에 조사만 붙인 문장보다 짧고 분명한 한국어 문장을 우선합니다.

## 그대로 유지할 것

다음 항목은 영어와 한국어 문서에서 그대로 유지합니다.

- API names
- schema names
- enum values
- DDL names
- code identifiers
- field names
- file names and path names
- stable identifiers
- error codes and validator IDs

Code block 안의 code, API method name, enum value, field name, file path, stable identifier, 그 밖의 정확한 문자열은 번역하지 않습니다.

다음 이름은 literal identifier, schema/API value, file/template name, heading anchor, code-like reference처럼 정확한 문자열을 가리킬 때 그대로 씁니다. 일반 한국어 prose에서 개념을 설명할 때는 아래 [이중 언어 용어표](#한국어-기준-용어)를 우선합니다.

- Task
- Change Unit
- Decision Packet
- Write Authorization
- Evidence Manifest
- Eval
- Gate
- ProjectionKind
- MCP
- Core
- state.sqlite
- task_events
- judgment_category
- judgment_route
- display_depth
- judgment_domain
- decision_kind
- decision_profile
- prepare_write
- record_run
- close_task

`HARNESS:BEGIN` 같은 marker, `ArtifactRef`, `ProjectionKind`, `decision_kind=approval`, `approval_gate`, `ResidualRiskSummary.status=none`, validator ID, error code, file path, API/tool/schema 이름은 번역하지 않습니다.

단계 라벨을 한국어 prose에서 쓸 때는 한국어 설명을 먼저 두고 표준 영어 라벨을 괄호에 둡니다. 예를 들어 `코어 권한 스모크(v0.1 Core Authority Smoke)`, `커널 스모크(Kernel Smoke)`, `첫 사용자 가치 조각(v0.2 First User-Value Slice)`, `에이전시 보증 팩(v0.3 Agency Assurance Pack)`, `운영과 인계 팩(v0.4 Operations & Handoff Pack)`, `v1+ Expansion`을 사용합니다. `강화된 로컬 기준 목표(hardened local reference target)`는 단계 라벨이 아닙니다. v0.2 이후 정식 단계/팩인 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)의 담당 문서 정의 profile을 완료해 도달하는 용어집 정의 상위 목표를 가리킬 때만 쓰고, 추가 단계나 fixture profile, suite name으로 쓰지 않습니다. 세 공간 모델을 한국어 prose로 설명할 때는 `제품 저장소`, 이 저장소의 향후 source 역할을 가리킬 때는 `하네스 서버 소스 저장소`, 운영 데이터 공간은 `하네스 런타임 홈`을 사용합니다. Architecture term을 구분해야 할 때만 영어 라벨을 괄호로 덧붙입니다.

Lookup anchor로 쓰이는 Reference heading은 전용 link/anchor migration으로 모든 link를 함께 고치지 않는 한 안정적으로 유지합니다. 사용자용 prose에서는 자연스러운 한국어를 우선합니다. 안정적인 Reference heading 아래에는 한국어 alias line으로 자연스러운 표현을 제공할 수 있습니다.

<a id="한국어-기준-용어"></a>

## 이중 언어 용어표

아래 용어는 한국어 prose에서 우선 사용하는 기준 표현입니다. 정확한 identifier나 계약 값이 필요한 곳에서는 영어 문자열을 그대로 유지하고, 독자에게 도움이 될 때 한국어 표현 뒤에 괄호로 붙입니다.

| English term | 한국어 기준 표현 | 사용 메모 |
|---|---|---|
| Harness | 하네스 | 제품명을 일반 prose에서 쓸 때 사용합니다. `HARNESS:BEGIN` 같은 marker나 literal string은 유지합니다. |
| Harness Server | 하네스 서버 | 세 공간 모델에서 로컬 하네스 프로그램/설치를 가리킵니다. 사용자의 제품 저장소나 런타임 데이터 홈을 가리키지 않습니다. |
| Harness Server source repository | 하네스 서버 소스 저장소 | 이 저장소의 future source-code 역할을 가리킵니다. 서버/런타임 구현 시작에는 문서 수락과 별도의 구현 계획 준비 결정이 필요합니다. |
| Product Repository | 제품 저장소 | 사용자의 제품 작업 공간을 가리킵니다. 세 공간 모델을 구분해야 할 때만 영어 라벨을 병기합니다. |
| Harness Runtime Home | 하네스 런타임 홈 | 사용자별/설치별 운영 데이터 공간을 가리킵니다. 독자에게 도움이 될 때만 영어 라벨을 병기합니다. |
| Core-owned state | Core가 소유한 상태 | Core 기록이 운영 권한임을 강조할 때 씁니다. Core 경계가 이미 분명한 사용자용 문맥에서는 `운영 기준 상태`가 더 자연스러울 수 있습니다. |
| durable local state | 지속 로컬 상태 | 첫 사용에서 `지속 로컬 상태(durable local state)`처럼 병기할 수 있습니다. |
| work | 작업 | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일을 가리킵니다. Mode value나 code-like reference로 쓰는 `work`는 그대로 둡니다. |
| scope | 범위 | 무엇이 바뀔 수 있고 무엇이 범위 밖인지 설명할 때 씁니다. 내부 scoped-write record를 이름 붙일 때만 `Change Unit`을 붙입니다. |
| out of scope | 범위 밖 | 제외되는 동작, 파일, 판단, 완료 주장을 말할 때 씁니다. Identifier를 인용하는 경우가 아니라면 `out-of-scope한` 같은 혼합어는 피합니다. |
| Discovery | 요구사항 구체화 | 구현 계획 전에 에이전트가 요구사항을 구체화하는 자세로 설명합니다. 명령 이름처럼만 다루지 않습니다. Reference/schema 문맥에서는 `Discovery`를 유지합니다. |
| Change Unit | 범위 / Change Unit | 사용자용 prose에서는 제한된 작업 경계를 먼저 `범위`로 설명합니다. Record나 reference term을 이름 붙일 때는 `Change Unit`을 유지합니다. |
| judgment | 판단 | 사용자가 소유하는 선택을 가리킬 때 씁니다. 기록되는 구현 경로를 이름 붙일 때만 `Decision Packet`을 붙입니다. |
| judgment request | 판단 요청 | 사용자가 보는 평소 질문 표현입니다. 문맥에 따라 `무엇을 결정해야 하나요?`처럼 더 자연스러운 질문을 써도 됩니다. |
| user-owned judgment | 사용자 소유 판단 | 사용자의 판단권을 보존한다는 넓은 원칙을 말할 때 씁니다. 이를 전체 문서에서 `사용자 결정`으로 바꾸지 않습니다. |
| User Decisions display group | 사용자 결정 | 대기 중인 결정 항목을 나열하는 사용자 표시 그룹에만 씁니다. Schema field, `gate`, 경로, 권한 경로처럼 다루지 않습니다. |
| decision type | 결정 유형 | 구체적인 대기 결정 항목에 이름을 붙일 때 씁니다. |
| Product/UX judgment | 제품/UX 판단 | 제품 동작, 문구, flow, UX 선택처럼 사용자가 소유하는 결정을 가리킵니다. |
| technical architecture judgment | 기술 구조 판단 | 아키텍처, 의존성, 마이그레이션, 인터페이스, 중요한 기술 방향처럼 사용자가 소유하는 결정을 가리킵니다. |
| security/privacy judgment | 보안/개인정보 판단 | 보안, 개인정보, 보관, 가림 처리, 감사, 노출 범위처럼 사용자가 소유하는 결정을 가리킵니다. |
| scope/autonomy judgment | 범위/자율성 판단 | Scope expansion, work split, Autonomy Boundary 선택처럼 사용자가 소유하는 결정을 가리킵니다. |
| QA waiver | QA 면제 판단 | 필수 QA 경로를 면제할지에 대한 사용자 결정을 가리킵니다. 일반적인 `승인`으로 옮기지 않습니다. |
| verification waiver | 검증 면제 판단 | 필수 검증 경로를 면제할지에 대한 사용자 결정을 가리킵니다. 일반적인 `승인`으로 옮기지 않습니다. |
| `judgment_category` | 판단 묶음 / 판단 유형 | Schema/API/reference 문맥에서는 field name과 enum value를 그대로 둡니다. 사용자 표시에서는 `제품/UX 판단`, `기술 구조 판단`, `보안/개인정보 판단`, `QA/검증`, `작업 수락`, `잔여 위험`처럼 보여줍니다. |
| `judgment_route` | 판단 경로 | Schema/API/reference 문맥에서는 field name과 enum value를 그대로 둡니다. 사용자 표시에서는 실제 동사로 설명합니다. 선택, 미루기, 면제, 결과 수락, 위험 수용, 민감 단계 승인, 조정처럼 씁니다. |
| `display_depth` | 표시 깊이 | Schema/API/reference 문맥에서는 field name을 그대로 둡니다. 일반 사용자가 배워야 하는 개념처럼 만들지 않습니다. 짧은 질문, 장단점 질문, 위험이 큰 질문, 닫기에 영향을 주는 질문처럼 실제 표시 결과로 풀어 씁니다. |
| `judgment_domain` | 판단 영역 | `judgment_category`로 mapping되는 compatibility alias입니다. 오래된 schema/API/reference 문맥에서는 field name을 그대로 둡니다. 새 예시에서 정확한 field가 필요하면 `judgment_category`를 우선합니다. |
| `decision_kind` | 결정 경로 | `judgment_route`로 mapping되는 compatibility alias입니다. 오래된 schema/API/reference 문맥에서는 field name과 enum value를 유지합니다. |
| `decision_profile` | 결정 표시 깊이 | `display_depth`로 mapping되는 compatibility alias입니다. 오래된 schema/API/reference 문맥에서는 field name을 유지합니다. |
| Decision Packet | Decision Packet / 판단 요청 기록 | Record ID, API/schema 이름, heading anchor 등 literal context에서는 `Decision Packet`을 유지합니다. 사용자용 prose에서는 정확한 라벨 전에 `판단 요청`을 쓰거나, 도움이 되지 않으면 라벨을 생략합니다. |
| Write Authorization | 쓰기 허가 기록 | `prepare_write` 결과나 record를 설명하는 prose에서 사용합니다. API/tool 이름과 exact field는 유지합니다. |
| evidence | 근거 | 주장을 뒷받침하는 자료를 사용자용 prose에서 설명할 때 씁니다. Record나 API를 이름 붙일 때는 `Evidence`, `Evidence Manifest`, schema field를 정확히 유지합니다. |
| Evidence Manifest | 근거 매니페스트 | Prose에서 도움이 될 때만 한국어 표현을 씁니다. Record/template/schema 문맥에서는 `Evidence Manifest`를 유지합니다. |
| check | 확인 | 테스트, 변경 차이 검토, 조사, 출처 확인 같은 일반 확인에 씁니다. 공식 Verification 경로를 뜻할 때만 `검증`을 씁니다. |
| Verification | 검증 | 기록된 정확성 확인을 가리킬 때 씁니다. 공식 Verification 개념이 아니라 일반 확인을 뜻하면 문맥에 따라 `확인`을 씁니다. |
| Manual QA | 수동 QA | 사용자용 prose에서는 `수동 QA`를 기준으로 씁니다. "사람이 직접 확인해야 하는 품질"처럼 풀어 설명할 수 있지만 별도 기준 용어로 `사람의 QA`를 만들지 않습니다. Exact template/schema/API context에서는 `Manual QA`를 유지합니다. |
| final acceptance / Acceptance | 작업 수락 | 작업 경로가 요구할 때 사용자가 결과를 받아들이는 판단을 가리킵니다. 민감 동작 승인에는 쓰지 않습니다. 영어의 finality는 새 기준어를 만들지 말고 문장 안에서 풀어 설명합니다. 예: `작업 수락은 완료 결과를 사용자가 받아들이는 최종 판단입니다.` |
| Approval | 민감 동작 승인 | Canonical Approval 개념의 사용자용 표현입니다. `허가`는 permission을 설명하는 prose에서만 쓸 수 있고 두 번째 기준 용어가 아닙니다. 일반적인 `승인`은 작업 수락, 제품 판단, QA 면제 판단, 잔여 위험 수용, 쓰기 허가 기록을 뜻하면 안 됩니다. Reference/schema 문맥에서는 `Approval`을 유지합니다. |
| Residual Risk | 잔여 위험 | 사용자용 prose도 `잔여 위험`을 기준으로 맞춥니다. 쉬운 설명이 필요하면 `남은 불확실성`처럼 풀어 쓸 수 있습니다. |
| residual-risk acceptance | 잔여 위험 수용 | 이름 붙은 잔여 위험을 사용자가 명시적으로 받아들이는 판단입니다. `작업 수락(Acceptance)`과 구분합니다. |
| close readiness | 닫기 준비 상태 | 완료나 닫기 전에 아직 확인하거나 처리해야 하는 것을 보여주는 공개 요약 표현으로 일관되게 씁니다. 영어 표시 그룹 라벨이나 정확한 문서 heading을 맞출 때만 `Close Readiness`를 유지합니다. |
| close blocker | 닫기 막힘 | 닫기가 진행될 수 없는 구체적인 이유를 가리킵니다. API/reference 문맥에서는 `close blocker`나 정확한 schema name을 유지할 수 있습니다. |
| next safe action | 다음 안전한 행동 | 해결되지 않은 판단, 범위, 근거, QA, 검증, 작업 수락, 위험을 숨기지 않고 진행할 수 있는 다음 행동을 말합니다. |
| blocker | 막힘 | 사용자용 prose에서는 진행이나 닫기를 막는 것을 `막힘`으로 쓸 수 있습니다. API/reference 문맥에서는 `blocker`를 유지하거나, 독자에게 도움이 될 때 `차단 조건(blocker)`으로 설명합니다. `blockers`, `CloseBlockerCategory` 같은 field name, template key, enum-like value, schema name은 번역하지 않습니다. |
| ArtifactRef | `ArtifactRef` / 아티팩트 참조 | Schema name은 정확히 유지합니다. Prose에서는 `아티팩트 참조`를 씁니다. Evidence 문맥에서는 `근거 아티팩트 참조`도 가능합니다. |
| artifact ref | 아티팩트 참조 | Evidence 문맥에서는 `근거 아티팩트 참조`도 가능합니다. `ArtifactRef` schema name은 유지합니다. |
| projection / Projection | 읽기용 요약 | 사용자용 첫 설명에서는 `읽기용 요약(Projection)` 또는 `읽기용 요약`을 씁니다. 이후에는 독자 문맥에 따라 `읽기용 요약`을 우선하고, 정확한 API/schema 개념을 가리킬 때만 `Projection`을 유지합니다. Markdown projection은 `Markdown 읽기용 요약` 또는 `Markdown으로 렌더링된 읽기용 요약`으로 옮깁니다. Projection은 운영 권한이 아닙니다. Reference/schema 문맥에서는 `Projection`, `ProjectionKind`, `projection freshness`, API field, template kind 또는 `projection view`를 유지합니다. |
| kernel | 커널 | 공식 heading이나 owner link가 아니라면 `커널`을 씁니다. |
| gate | 관문 | Use/Learn 문서에서는 `관문`을 우선합니다. 참조 문서에서 kernel field나 value를 가리킬 때는 `gate`를 유지할 수 있습니다. |
| detached verification | 분리 검증 | 보장 수준이나 보증 설명에서 `detached verification`을 병기할 수 있습니다. |
| cooperative | 협력형 | Guarantee-level table에서는 영어 라벨도 유지합니다. |
| detective | 탐지형 | Guarantee-level table에서는 영어 라벨도 유지합니다. |
| preventive | 예방형 | Guarantee-level table에서는 영어 라벨도 유지합니다. |
| isolated | 격리형 | Guarantee-level table에서는 영어 라벨도 유지합니다. |

## 자연스럽게 옮길 것

문장과 독자 맥락에 맞는 말을 고릅니다.

| English term | 한국어 문서에서의 원칙 |
|---|---|
| context | 식별자처럼 쓰이거나 AI 세션 맥락을 가리키면 `context`를 유지할 수 있습니다. 일반 문장에서는 `맥락`을 씁니다. |
| boundary | 코드나 식별자 맥락에서는 `boundary`를 유지합니다. 일반 문장에서는 `경계`를 씁니다. |
| authority | 운영 권한은 보통 `권한`으로 옮깁니다. 권한의 출처를 강조해야 하면 `기준 권한`을 씁니다. |
| canonical | 식별자 맥락에서는 `canonical`을 유지합니다. 일반 문장에서는 `기준` 또는 `기준 기록`을 씁니다. |
| change / modify | 상태나 기록을 바꾸는 의미입니다. 한국어에서는 `변경하다`를 씁니다. |
| surface | 문맥에 맞게 `interface`, `view`, `entrypoint`, `display area`나 그에 맞는 한국어 표현을 고릅니다. 사용자용 한국어에서는 보통 `접점`, `화면`, `표시 영역`이 자연스럽습니다. |
| evidence | 제품 용어로 필요할 때만 `evidence`를 유지합니다. 일반 문장에서는 `근거` 또는 `증거`를 씁니다. |
| acceptance / final acceptance | 사용자가 결과를 받아들인다는 판단이면 `작업 수락`을 우선합니다. 영어 문장이 finality를 강조하면 별도 기준어로 바꾸기보다 주변 문장에서 그 의미를 풀어 씁니다. |
| acceptance criteria | 공식 acceptance criteria를 가리키면 `수용 기준`을 씁니다. 정식 기준보다 작업 완료 조건을 말하는 문맥이면 `완료 기준`도 가능합니다. `수락 기준`은 쓰지 않습니다. |
| residual-risk acceptance / accepted risk | 기준 route 표현은 `잔여 위험 수용`입니다. 설명 문장에서는 `잔여 위험을 받아들이는 판단`, `잔여 위험을 받아들이다`도 가능합니다. Schema/reference 문맥에서는 정확한 enum/field name을 유지합니다. 이 개념은 generic `수락` 표현으로 옮기지 않습니다. `작업 수락`과 다른 결정임을 분명히 합니다. |
| Acceptance Gate / acceptance_gate | 필요한 경우 `Acceptance Gate` 또는 `acceptance_gate` 같은 식별자를 그대로 유지합니다. 불안정한 새 번역어를 만들지 말고 한국어 문장으로 뜻을 설명합니다. |
| residual risk | 기준 표현은 `잔여 위험`입니다. 쉬운 설명에서 남는 불확실성을 풀어 말할 수 있지만, 문서 전체 용어는 `잔여 위험`으로 맞춥니다. |
| approval / Approval | 사용자용 prose에서는 민감 동작 permission을 `민감 동작 승인`으로 씁니다. 기준 하네스 status, `gate`, record, schema, exact reference term을 이름 붙일 때는 `Approval`을 유지합니다. 일반적인 `승인`이 작업 수락, 제품 판단, QA 면제 판단, 잔여 위험 수용, 쓰기 허가 기록을 뜻하게 하면 안 됩니다. |
| write authority | 일반 문장에서는 `쓰기 권한`을 쓸 수 있습니다. `prepare_write`가 남기는 하네스 기록을 말할 때는 `쓰기 허가 기록(Write Authorization)`을 씁니다. |
| gate | 사용자용 흐름에서는 `관문`, `확인`, `닫기 확인`, `막힘`처럼 문맥에 맞는 말을 우선합니다. 참조 문서에서 kernel field나 strict contract를 가리킬 때는 `gate`를 유지할 수 있습니다. |
| prompt | 사용자용 prose에서는 보통 `질문`, `표시 질문`, `모델에 전달되는 맥락`처럼 문맥에 맞게 풉니다. `prompt injection`, exact prompt template, schema/code context에서는 영어를 유지할 수 있습니다. |
| profile | 사용자용 prose에서는 `프로필`을 씁니다. `decision_profile`, DDL profile, fixture profile name 같은 식별자나 정확한 라벨은 유지합니다. |
| sandbox | 보안 경계 설명에서는 문맥에 따라 `샌드박스` 또는 `격리 환경`을 씁니다. `OS sandbox`처럼 정확한 메커니즘을 이름 붙일 때만 영어 병기를 유지할 수 있습니다. |
| preventive control | 보장 수준 설명에서는 `사전 차단 통제` 또는 `사전 차단 장치`를 씁니다. `preventive` 라벨 자체를 소개할 때만 영어를 유지합니다. |
| pros/cons, recommendation, uncertainty, deferral analysis | 결정 질문 설명에서는 `장단점`, `추천`, `불확실성`, `미루면 생기는 일`을 씁니다. 필드 이름, enum 값, 스키마 식별자는 유지합니다. |
| trade-off | 사용자 선택의 비교라면 `장단점 비교`를 우선하고, 기술적 균형점을 말할 때는 `절충`을 씁니다. `_tradeoff` enum 값과 field 이름은 유지합니다. |
| reconcile | 사용자용 prose에서는 `조정`을 먼저 씁니다. API action, enum value, heading, exact record label은 `reconcile` 또는 `Reconcile`을 유지하거나 `조정(reconcile)`처럼 병기합니다. |
| unblocker | 사용자에게는 `막힘을 푸는 최소 조치` 또는 `해소 방법`으로 씁니다. Schema/API field나 exact label이 아니면 `unblocker`를 그대로 두지 않습니다. |
| agent / Agent | 일반 prose에서는 `에이전트`를 씁니다. `Agent Integration` 같은 문서명, product/interface label, file path, code identifier에서는 영어를 유지합니다. |

대소문자 규칙: `Approval`은 민감한 행동에 대한 기준 하네스 permission 개념입니다. 소문자 `approval`은 `approval_gate`, `decision_kind=approval`, `approval_request_candidate`, `approval_scope`, `approval-shaped`, approval drift처럼 stable identifier, enum value, schema name, 의도적으로 고정된 표현, quoted legacy/user wording에서만 유지할 수 있습니다.

## 피할 표현

영어 기술어를 그대로 끼워 넣었지만 독자에게 더 선명해지지 않는 표현은 피합니다.

- `상태 변경을 영어 동사와 섞어 쓴다`
- `authority boundary 유지라고 쓴다`
- `surface 표시라고 쓴다`
- `projection freshness를 report한다`
- `acceptance를 complete한다`
- `risk를 accept한다`
- `acceptance criteria를 수락 기준으로 쓴다`
- `Harness 상태를 local state와 artifact ref에 둔다`
- `detached verification을 독립 검증이라고만 쓴다`

## 더 나은 표현

의미는 보존하되 한국어 문장으로 자연스럽게 씁니다.

- `상태를 변경한다`
- `권한 경계를 유지한다`
- `화면에 보여준다`
- `projection이 최신인지 표시한다`
- `결과를 수락한다`
- `잔여 위험을 받아들인다`
- `수용 기준을 확인한다`
- `하네스 상태를 지속 로컬 상태와 아티팩트 참조에 둔다`
- `분리 검증(detached verification)을 기록한다`

## Before / After 예시

| Before | After |
|---|---|
| `Core가 state 변경을 수행한다.` | `Core가 상태를 변경한다.` |
| `Agent는 authority boundary 유지가 필요하다.` | `Agent는 권한 경계를 유지해야 한다.` |
| `이 surface는 blocker 표시를 담당한다.` | `이 화면은 blocker를 보여준다.` |
| `Operations는 projection freshness를 report한다.` | `Operations는 projection이 최신인지 표시한다.` |
| `canonical source를 update한다.` | `기준 기록을 업데이트한다.` |
| `context를 잃지 않도록 한다.` | 일반 문장에서는 `맥락을 잃지 않도록 한다.` |
| `acceptance가 필요하다.` | `작업 수락이 필요하다.` |
| `risk를 accept한다.` | `잔여 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 작업 수락처럼 쓴다.` | `잔여 위험 수용`처럼 작업 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |
| `Harness 상태는 local state와 artifact ref에 있다.` | `하네스 상태는 지속 로컬 상태와 아티팩트 참조에 있다.` |
| `detached verification을 독립 검증으로 표시한다.` | `분리 검증(detached verification)으로 표시한다.` |
| `close-relevant check gap이 있다.` | `닫기에 영향을 주는 확인 공백이 있다.` |
| `Waiver prompt and summary를 보여준다.` | `면제 질문과 요약을 보여준다.` |
| `valid independence와 current reviewed inputs가 있다.` | `유효한 독립성과 현재 검토된 입력이 있다.` |

## 한국어 제목 원칙

한국어 제목은 영어 제목을 기계적으로 옮기지 않습니다.

Official identifier 자체를 설명하는 제목이라면 identifier를 정확히 유지합니다. 그렇지 않다면 한국어 기술 독자가 자연스럽게 이해할 제목을 고릅니다.

문서의 heading order와 scope는 영어 문서와 맞춥니다. 하지만 heading text가 단어 단위로 일치할 필요는 없습니다.

## 이중 언어 리뷰 체크리스트

```text
[ ] 한국어 페이지가 영어 페이지와 같은 의미를 보존하는가?
[ ] Paired file이 같은 active file path, reader purpose, semantic section coverage, owner link, 계약 세부사항을 유지하는가?
[ ] 한국어 문장이 한국어 기술 독자에게 자연스럽게 읽히는가?
[ ] API name, schema name, enum value, DDL name, identifier, path, error code, validator ID가 정확한가?
[ ] 일반 prose의 하네스 기준 용어가 일관되고, exact identifier는 그대로 유지되었는가?
[ ] Source-of-truth phrase와 owner link가 owner Reference docs와 맞는가?
[ ] Owner가 아닌 중복 contract는 전체 contract 번역이 아니라 요약과 owner link로 처리되었는가?
[ ] 불필요한 혼합어 표현을 가능한 한 자연스러운 한국어로 바꾸었는가?
[ ] 사용자용 문서에서 한국어 표현과 하네스 라벨이 모두 필요할 때 자연스러운 한국어가 먼저 나오는가?
[ ] 제목이 자연스러우면서도 같은 문서 구조와 범위를 유지하는가?
[ ] 영어와 한국어 링크 변경이 같은 batch에 들어갔는가?
[ ] Review가 translation drift를 runtime state, evidence, QA, 작업 수락, 닫기 준비 상태, implementation readiness로 취급하지 않는가?
```
