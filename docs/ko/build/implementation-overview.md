# Build: 구현 개요

## 이 문서가 도와주는 일

이 문서는 구현자가 계획 또는 구현 질문에 필요한 특정 Reference owner 명세를 보기 전에 무엇을 먼저 계획해야 하는지 알려 줍니다. 독자 중심 문서가 kernel, runtime, MCP, storage, 읽기용 요약(Projection), conformance reference와 어떻게 이어지는지 보여 주는 Build 계층입니다.

이 문서는 문서 재설계 / 검토와 유지보수자용 문서 수락 후보 검토를 위한 구현 계획 문서입니다. 이 저장소는 현재 문서 전용이며, 향후 역할은 하네스 서버 소스 저장소입니다. 이 저장소에서 서버/런타임 구현을 시작하려면 문서 수락과 별도의 구현 계획 준비 결정이 모두 필요합니다. 아직 이곳에는 하네스 서버/런타임 구현, 실행 가능한 fixture 파일, 생성된 런타임 기록, 생성된 읽기용 요약, 실행 가능한 하네스 서버 conformance test가 없습니다. 이 리비전은 재설계 이후 검토 상태의 문서 수락 후보이지 구현 시작 허가가 아닙니다. 첫 실행 목표는 v0.1 Core Authority Smoke이며, 커널 스모크(Kernel Smoke)는 가장 작은 로컬 권한 루프를 위한 좁은 future smoke-check 작성 label입니다. 첫 사용자 가치 목표는 v0.2 First User-Value Slice입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 agency assurance, operations, handoff 동작을 단단하게 만듭니다. v1+ Expansion은 담당 문서가 승격하고 증명하기 전까지 로드맵 범위에 둡니다.

이 Build 문서는 상세 단계와 구현 상태 경고를 맡습니다. 그래야 Learn/Use 문서는 사용자 경험에 집중할 수 있습니다. 아래의 현재 검토 기준과 문서 수락 상태가 유지보수자가 상태를 바꿀 때 갱신해야 하는 상세 인계 섹션입니다.

이 문서로 다음을 확인합니다.

- 먼저 필요한 런타임 구성 요소는 무엇인가?
- v0.1 Core Authority Smoke은 어떤 증명을 보여야 하는가?
- 첫 사용자 가치 조각을 완료했다고 말하려면 무엇이 참이어야 하는가?

이 문서는 SQLite DDL, public MCP 스키마, 읽기용 요약(Projection) 템플릿 본문, 명령 문법을 정의하지 않습니다. 그런 세부 계약은 Reference 문서에 둡니다.

## 이런 때 읽기

- 유지보수자 인계가 첫 런타임 배치를 위한 구현 계획 준비 상태를 명시적으로 수락한 뒤 첫 구현 형태를 계획할 때.
- 제안된 staged build가 올바른 범위를 유지하는지 리뷰할 때.
- 엄밀한 Reference 명세를 읽기 전에 짧은 지도가 필요할 때.

## 읽기 전에

Learn 경로에서 하네스의 기본 개념을 먼저 이해해 두는 것이 좋습니다. 정확한 동작은 이 문서 끝에 연결된 Reference 문서들을 봅니다. v1+ Expansion 후보와 승격 기준은 [로드맵](../roadmap.md)을 봅니다.

## 핵심 생각

하네스는 AI 지원 제품 작업을 위한 로컬 작업 장부이자 판단 라우터입니다. 무엇을 바꿀 수 있는지, 누가 판단해야 하는지, 어떤 근거가 있는지, 어떤 위험이 남았는지, 작업을 닫아도 되는지를 기록합니다. 첫 구현 경로는 가장 작은 Core 권한 루프로 그 로컬 장부를 증명한 뒤, 좁은 첫 사용자 가치 조각을 증명해야 합니다.

v0.1 Core Authority Smoke을 먼저 만듭니다. 가장 작은 로컬 Core 권한 경로를 증명하며, 커널 스모크(Kernel Smoke)는 좁은 future smoke-check 작성 label입니다. 이것은 내부 smoke 단계이지 제품 MVP가 아닙니다. 그다음 v0.2 First User-Value Slice를 만들어 ordinary-language start/resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, evidence summary, 닫기 막힘, residual-risk visibility, 민감 동작 승인 / 작업 수락 / 잔여 위험 수용 분리 표시를 사용자가 볼 수 있게 합니다. 근거와 간결한 상태 출력은 이 경험을 지원하지만 full assurance, QA, Eval, reporting, operations, dashboard 시스템이 아닙니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)이 그 경로를 단단하게 만듭니다.

이 Build 경로의 모든 구현 동사는 유지보수자 인계가 그 배치를 위한 구현 계획 준비 상태를 명시적으로 수락한 뒤의 향후 런타임 배치 계획을 설명합니다. [문서 수락 상태](#문서-수락-상태)가 구현 계획 준비 상태를 수락하지 않는 동안에는 이 문서를 범위와 인계 준비 상태를 검토하는 용도로만 사용합니다. 문서 수락만으로 구현이 시작되거나 런타임 conformance가 증명되지는 않습니다.

그 인계 상태가 바뀌면 구현은 이 저장소에서 하네스 서버/설치 프로그램의 소스 코드로 진행될 예정입니다. 그래도 이 저장소는 사용자의 제품 저장소나 하네스 런타임 홈이 아닙니다. 런타임 상태, 아티팩트, 읽기용 요약 출력, 로그는 하네스 런타임 홈에 속합니다.

로컬 커널은 조율과 권한의 기록이지 제품 저장소, 소스 관리, 테스트, 코드 리뷰, 대화, 사용자 소유 제품 판단과 기술 구조 판단을 대체하지 않습니다. 첫 경로는 상태/막힘 출력이 최소 권한 상태와 무엇이 빠졌는지를 설명할 수 있게 계획하되, ordinary-language intake, 닫기 막힘 요약, 작업 수락 display, residual-risk visibility, 사용자용 간결한 설명은 v0.2와 이후 단계에 둡니다.

첫 권한 루프는 좁게 유지합니다. `prepare_write`는 제품 파일 쓰기에 대한 유일한 권한 판단 지점이고, 반환된 쓰기 허가 기록은 지속적이며 한 번만 쓸 수 있으며, `record_run`은 관찰된 변경과 artifact/evidence ref 하나를 기록하면서 하나의 호환되는 direct Run 또는 implementation Run에 대해 이를 소비합니다. v0.1은 blocker를 위해 status나 좁은 close-task smoke를 사용할 수 있지만, 작업 수락이나 잔여 위험 close semantics를 증명하지 않습니다. 정확한 상태 로직은 [커널 참조](../reference/kernel.md#prepare_write)에, public request/response detail은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#public-tools)에 둡니다.

기준 상태, local project registration 하나, active Task 하나, Reference 계약상 필요한 경우에만 Change Unit 소유자 형태로 표현되는 scoped boundary 하나, Write Authorization 경로 하나, 기록된 Run 하나, artifact/evidence link 하나, Core tool 동작, 그리고 그 경로를 실행해 볼 최소 MCP reachability에서 시작합니다. 초기 구현 가정은 분산 platform이 아니라 모듈을 가진 로컬 프로세스 하나입니다. Natural-language intake, full Discovery, full Decision Packet, full Evidence Manifest behavior, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, detached verification, projection rendering, conformance runner, dashboard 또는 hosted workflow UI, index, 넓은 connector ecosystem 또는 marketplace, team workflow, 접점별 connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, operations/export/recover, 넓은 operator entrypoint, broad automation은 이후 단계이거나 그 권한 루프가 존재한 뒤 그것을 읽거나 감싸는 권한 없는 요소로 다룹니다.

구현 계획이 full user-facing system, 에이전시 보증 팩(v0.3 Agency Assurance Pack)이나 운영과 인계 팩(v0.4 Operations & Handoff Pack)의 동작 전체, 읽기용 요약 템플릿 다듬기, dashboard 또는 hosted workflow UI, Context Index, connector marketplace, hook expansion, metrics, parallel orchestration, broad automation lane에서 시작한다면 첫 runnable smoke slice보다 큰 곳에서 시작하는 것입니다.

## 현재 검토 기준

현재 문서 세트는 여전히 문서 전용이며 재설계 이후 검토 상태입니다. 이 저장소의 향후 역할은 하네스 서버 소스 저장소입니다. 런타임/서버 구현은 시작하지 않았으며, 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다. 아래의 유지보수자 갱신 상태 표가 명시적으로 말하지 않는 한 현재 상태는 완전히 수락되었거나, 구현 완료되었거나, 구현 준비가 끝났거나, 서버 코딩을 허가한 상태가 아닙니다.

이 저장소 단계에서는 서버/런타임 구현 결정을 코드 작성용으로 공식 수락하지 않았습니다. [단계별 전달 계획: 서버 코딩 전 필요한 구현 결정](mvp-plan.md#서버-코딩-전-필요한-구현-결정)의 open decision ledger에는 영향을 받는 stage를 코딩하기 전에 수락하거나 명시적으로 미뤄야 하는 pre-coding decision이 기록되어 있습니다.

남은 문서 drift와 검토 위험은 [문서 작성 가이드](../maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)에서 관리합니다. 그 tracker는 현재 문서에서 확인된 drift, 확인 대상 후보, 회귀 방지 점검, 기준 상태 점검을 구분하고, 확인된 finding을 아래 범주로 라우팅합니다. 검토 위험은 기본적으로 열린 구현 결정이 아니지만, 확인 결과 서버 코딩 전 결정이나 단계 blocker가 드러나면 [단계별 전달 계획: 서버 코딩 전 필요한 구현 결정](mvp-plan.md#서버-코딩-전-필요한-구현-결정)에 담당 문서, 영향을 받는 동작 또는 field, 영향을 받는 단계, 선택지, 필요한 결정을 기록합니다.

| 남은 항목 범주 | 의미 | 기록 위치 | 막힘 의미 |
|---|---|---|---|
| 문서 drift | 문구, 소유자 경계, link, TODO, 용어, 영어/한국어 의미 일치 문제입니다. | 문서 작성 가이드 tracker와 영향을 받는 문서. | 문서가 서로 모순되거나 실행하기 어렵게 만들면 문서 수락을 막을 수 있습니다. 그 자체로 런타임 conformance나 서버 코드는 아닙니다. |
| 스키마/설계 결정 | 상태, API, DDL, 보안 보장, fixture 의미, 그 밖의 담당 계약에 관한 실제 선택입니다. | 담당 Reference 문서와, 서버 코딩 전에 결정해야 할 때 단계별 전달 계획의 결정 기록. | 결정되거나 단계 영향과 함께 명시적으로 미뤄지기 전까지 영향을 받는 동작의 구현 계획이나 서버 코딩을 막습니다. |
| 단계 경계 결정 | capability가 v0.1 Core Authority Smoke, v0.2 First User-Value Slice, 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack), v1+ Expansion 중 어디에 속하는지에 대한 선택입니다. | 구현 개요, 단계별 전달 계획, 담당 문서, 필요한 경우 로드맵 승격 항목. | 경계가 수락되기 전까지 영향을 받는 단계 구현을 막습니다. 명시적으로 기록되어 있으면 문서 검토에는 막힘이 아닐 수 있습니다. |
| 구현 준비 조건 | 첫 런타임 배치 계획 전에 유지보수자가 확인해야 하는 조건입니다. | 이 문서의 [하네스 서버 구현 준비 조건](#하네스-서버-구현-준비-조건). | 충족되거나 유지보수자가 다른 범주로 명시적으로 재분류하기 전까지 첫 런타임 배치 계획을 막습니다. |
| 향후 로드맵 항목 | 승격되기 전까지 v0.1부터 v0.4 밖에 있는 유용한 capability입니다. | [로드맵](../roadmap.md)과 승격 뒤 담당 문서. | 담당자가 단계 목표로 승격하지 않는 한 문서 검토, v0.1, v0.2를 막지 않습니다. |

## 문서 수락 상태

이 항목은 유지보수자가 직접 갱신하는 문서 수락 상태 표시입니다. 문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태를 분리합니다. Reference 계약, conformance 결과, 생성된 운영 기록, 생성된 읽기용 요약, 런타임 기록, 런타임 구현 허가로 쓰지 않습니다. 아래 checkpoint에서 수락을 자동 추론하지 않습니다. 유지보수자가 이 표를 명시적으로 바꿔야 합니다.

현재 리비전 상태: 재설계 이후 문서 검토 상태이며 유지보수자 검토를 위한 문서 수락 후보입니다. 유지보수자가 명시적으로 바꾸기 전까지 문서 수락은 여전히 아니오입니다. 이 상태 표시는 런타임/서버 구현, 런타임 conformance, 구현 완료, 구현 준비 상태가 아닙니다.

| 상태 범주 | 현재 상태 | 경계 |
|---|---|---|
| 문서 검토 상태 | 재설계 이후 검토 상태이며 문서 수락 후보입니다. 유지보수자 수락은 아직 남아 있습니다. | 문서가 검토 중인지, 후보인지, 수락되었는지는 이 표가 말할 때만 그렇게 읽습니다. 문서 수락은 런타임 구현을 자동으로 시작하거나 런타임 conformance를 만들지 않습니다. |
| 구현 계획 준비 상태 | 수락되지 않았습니다. 아래 구현 준비 조건이 충족된 뒤 유지보수자가 이 행을 바꾸기 전까지 첫 런타임 배치 계획은 시작할 수 없습니다. | 편집 정리는 스키마/설계 결정, 단계 경계 결정과 별개입니다. 남은 구현 준비 조건은 유지보수자 판단이 필요합니다. |
| 런타임 구현 상태 | 시작하지 않았습니다. 이 저장소는 아직 문서만 담고 있으며 하네스 서버/런타임 구현을 담고 있지 않습니다. | 아직 서버/런타임 코드, 런타임 상태, 생성된 운영 아티팩트, 실행 가능한 fixture, fixture 파일, 생성된 읽기용 요약, 런타임 기록, 실행 가능한 하네스 서버 conformance test가 없습니다. |
| 서버 코딩 전 결정 기록 | Open decision이 단계별 전달 계획에 기록되어 있습니다. 아직 server/runtime implementation decision은 코드 작성용으로 수락되지 않았습니다. | 영향을 받는 API, DDL, state transition, local access, status card, evidence, acceptance, risk behavior를 코딩하기 전에 [단계별 전달 계획: 서버 코딩 전 필요한 구현 결정](mvp-plan.md#서버-코딩-전-필요한-구현-결정)의 open decision-ledger 항목을 해결하거나 명시적으로 미룹니다. |

Build 독자는 이 표를 진입 기준으로 보아야 합니다. 유지보수자 인계가 구현 계획 준비 상태를 명시적으로 수락하기 전까지 v0.1 Core Authority Smoke도 이 저장소에서는 계획 전용이며 하네스 서버/런타임 구현을 시작하면 안 됩니다.

## 문서 인계 요약

이 섹션은 이번 리비전의 최종 문서 인계 요약입니다. 문서 세트가 무엇을 정의하는지, 무엇이 아직 열려 있거나 확인이 필요한지, 이 저장소에서 하네스 서버 구현 계획을 시작하기 전에 무엇이 참이어야 하는지 보여 줍니다. 이것은 문서 인계일 뿐입니다. 런타임 상태, 작업 수락 기록, 생성된 읽기용 요약, conformance 결과, 런타임 기록, 구현 권한, 서버 코드를 만들지 않습니다.

이 문서 세트가 정의하는 것:

- Harness 제품 명제: scope, 사용자 소유 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태, 닫기 준비 상태를 위한 로컬 기준 기록과 판단 경로입니다.
- Learn, Use, Build, Reference, Maintain, Roadmap 문서 구조와 독자 경로입니다.
- 향후 하네스 서버/설치 프로그램 구현 계획입니다. 시작점은 v0.1 Core Authority Smoke이고, 그다음은 v0.2 First User-Value Slice입니다.
- Kernel, MCP/API schema, storage/DDL, projection/template, conformance fixture, operations, security, agent integration, design quality, glossary, runtime architecture의 exact contract owner 위치입니다.
- Owner 경계, 영어/한국어 의미 일치, 상태 표현, TODO hygiene, drift routing을 위한 문서 유지보수 규칙입니다.

이 문서 세트는 runnable server code, executable fixture file, 생성된 runtime artifact, 생성된 읽기용 요약, runtime conformance result, implementation acceptance record, Harness Runtime Home을 정의하지 않습니다.

현재 단계와 향후 저장소 역할:

- 이 저장소는 재설계 이후 문서 검토 단계이며 문서 수락 후보일 뿐입니다.
- 이 저장소의 향후 역할은 하네스 서버 소스 저장소입니다. 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.
- 사용자의 제품 저장소가 아니며 하네스 런타임 홈도 아닙니다.
- 아직 하네스 서버/런타임 구현, 런타임 상태, 생성된 운영 아티팩트, 실행 가능한 fixture, fixture 파일, 생성된 읽기용 요약, 런타임 기록, 실행 가능한 하네스 서버 conformance test가 없습니다.

보존하는 하네스 원칙:

- Harness는 scope, 사용자 소유 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태, 닫기 준비 상태를 위한 로컬 기준 기록입니다.
- Harness는 사용자 소유 판단을 보존합니다. 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, QA 기대치, 작업 수락, waiver, 잔여 위험 수용은 소유자 계약이 달리 정하지 않는 한 사용자 판단으로 남습니다.
- 근거, 검증, 수동 QA, 작업 수락, 잔여 위험은 서로 다른 기록과 판단입니다. 어느 것도 다른 것을 대신하지 않습니다.
- 대화, connector 출력, 생성 문서, Markdown으로 렌더링된 읽기용 요약(Projection)은 운영 기준이 아닙니다. Core가 소유한 로컬 상태와 아티팩트 참조가 기준입니다.

현재 단계 모델:

- v0.1 Core Authority Smoke은 가장 작은 로컬 Core 권한 루프를 증명합니다. Kernel Smoke는 이 단계의 좁은 future smoke-check 작성 label입니다.
- v0.2 First User-Value Slice는 좁은 범위의 ordinary user value를 증명합니다. Start/resume, work-shape classification, scope/non-goals/success criteria, user-owned judgment routing, evidence summary, 닫기 막힘 요약, 작업 수락 display, residual-risk visibility가 여기에 속합니다.
- 에이전시 보증 팩(v0.3 Agency Assurance Pack)은 verification, 수동 QA, 잔여 위험 수용 close, 작업 수락 분리, stewardship, Decision Packet, Approval 분리, TDD, feedback-loop policy, context hygiene를 단단하게 만듭니다.
- 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 doctor/readiness, recover/export, artifact integrity, release handoff, 더 넓은 fixture coverage, later-boundary check를 단단하게 만듭니다.
- v1+ Expansion은 향후 소유자 결정이 exact contract, fixture, fallback behavior, 읽기용 요약을 기준으로 삼는 의존성 없음으로 승격하기 전까지 로드맵 범위입니다.

정리된 내용:

- 저장소 정체성이 명확합니다. 지금은 문서 전용이고, 향후 역할은 하네스 서버 소스 저장소이며, 서버/런타임 구현은 별도로 gate됩니다.
- 제품 명제가 명확합니다. Harness는 prompt 묶음, dashboard, broad hosted agent platform, generated Markdown 시스템이 아닙니다.
- 판단 모델은 Approval, Decision Packet, 작업 수락, 잔여 위험 수용, QA/검증 waiver decision, Write Authorization을 분리합니다.
- 읽기용 요약(Projection)과 대화는 읽기/대화 접점이며 운영 기준이 아닙니다.
- 읽기용 요약(Projection) 범위는 단계화되어 있습니다. v0.1은 owner path가 이미 만든 경우에만 freshness/read fact를 노출할 수 있고, v0.2는 Core-derived 간결한 상태 카드(compact status card)와 최소 사용자 읽기용 출력을 제공하며, detailed report/template은 승격되기 전까지 later-profile scope입니다.
- 보안 표현은 실제 enforcement level에 묶입니다. Cooperative, detective, preventive, isolated 주장은 해당 동작에 대해 문서화된 capability와 fixture-proven path가 있을 때만 사용합니다.
- Agent context는 제한됩니다. 항상 주입되는 맥락은 현재 Task 요약, 작업 모양, 범위/하지 않을 일, 대기 중인 사용자 판단, 활성 막힘, 다음 안전한 행동, 근거 공백, 닫기 막힘, 잔여 위험 요약, 보장 수준, 출처 참조와 최신성만 담고, 상세 contract와 큰 본문은 필요할 때 담당 문서나 retrieval path에서 가져옵니다.
- Conformance fixture 문서는 단계화된 향후 검증 계획입니다. 현재 executable fixture file이나 runnable conformance test가 있다는 뜻이 아닙니다.

현재 준비 상태:

- 문서 수락: 아직 대기 중입니다. 이 리비전은 maintainer 수락 검토 후보이지 수락된 문서가 아닙니다.
- 구현 계획 준비 상태: 아직 수락되지 않았습니다. 유지보수자가 아래 구현 준비 조건을 명시적으로 수락하거나 남은 blocker를 재분류하기 전까지 첫 runtime batch 계획은 시작하면 안 됩니다.
- 런타임 구현: 시작하지 않았습니다. 서버 코딩, fixture materialization, runtime conformance, 생성된 운영 산출물은 이 저장소 단계의 범위 밖입니다.
- 서버/런타임 구현 결정: open decision-ledger 항목은 있지만 코드 작성용으로 공식 수락된 결정은 없습니다. Maintainer review나 구현 준비 검토에서 추가 설계 쟁점이 발견될 수 있습니다.

서버 코딩 전 결정 기록 상태:

- Open server-coding decision-ledger 항목은 [단계별 전달 계획: 서버 코딩 전 필요한 구현 결정](mvp-plan.md#서버-코딩-전-필요한-구현-결정)에 기록되어 있습니다.
- 현재 open item에는 simplified judgment model and naming, `request_user_decision` vs `request_user_judgment`, `harness.next` vs `status.next_actions`, v0.2 storage minimum, local access error taxonomy, compact status card scope, small direct change evidence requirement, 작업 수락/잔여 위험 minimal records가 포함됩니다.
- Server/runtime implementation decision은 아직 코드 작성용으로 공식 수락되지 않았습니다. 해당 behavior의 server code나 DDL을 바꾸기 전에 영향을 받는 ledger item을 해결하거나 명시적으로 미룹니다.

문서 drift와 검토 위험 상태:

- 현재 기준에서 active docs 곳곳에 흩어진 major implementation-decision TODO를 의도적으로 남기지 않습니다.
- [문서 작성 가이드 tracker](../maintain/authoring-guide.md#알려진-재설계-쟁점-트래커)는 candidate drift와 regression risk를 확인하는 checklist로 남아 있습니다. 확인된 finding은 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목 중 하나로 routing합니다.
- 사용자에게 보이는 판단 묶음, 내부 기록 경로, 표시 깊이, 작은 Decision Packet의 무게와 관련해 추적하던 판단 모델 drift는 이 문서 기준에서 해소되었습니다. Review에서 남은 owner-contract decision이 드러나면 흩어진 TODO가 아니라 단계별 전달 계획 decision log로 routing합니다.
- 유지보수자가 확인해야 할 candidate review 영역에는 stage 이름 drift, 사용자용 문서의 무거운 disclaimer, Discovery/Change Unit 조기 수렴, Storage/API/DDL의 이른 범위 암시, projection/template 범위, conformance fixture detail, 너무 이른 operations entrypoint, security guarantee wording, agent context load, 한국어 기술 명사 과다, roadmap 경계 drift, 낙관적인 decision-log wording이 포함됩니다.

maintainer 수락 조건:

- 유지보수자가 [문서 수락 상태](#문서-수락-상태)를 의도적으로 갱신합니다. 이 checklist만 보고 수락을 추론하면 안 됩니다.
- 확인된 문서 drift는 고치거나, owner, 영향을 받는 단계, blocking meaning과 함께 분류합니다.
- 확인된 schema/design decision, stage boundary decision, 기타 server-coding decision은 server code나 DDL 변경 전에 단계별 전달 계획에 기록하고, 현재 open ledger item은 stage impact와 함께 해결하거나 명시적으로 미룹니다.
- [하네스 서버 구현 준비 조건](#하네스-서버-구현-준비-조건)이 충족되었거나 maintainer가 명시적으로 재분류합니다.
- 유지보수자가 수락 검토 중 [문서 작성 가이드](../maintain/authoring-guide.md#최종-사전-수락-리뷰)의 최종 docs-maintenance pass를 완료합니다. 영어/한국어 의미 일치, link/anchor, owner boundary, TODO hygiene, 현재 상태 표현을 확인합니다.
- 문서 수락과 별도의 구현 계획 준비 결정이 모두 있어야 첫 runtime batch 계획을 시작할 수 있습니다. 그 준비 결정이 명시적으로 수락되기 전까지 server/runtime 구현은 계속 막혀 있습니다.

## 하네스 서버 구현 준비 조건

이 checkpoint는 유지보수자가 구현 계획 준비 상태를 문서 유지보수에서 첫 런타임 배치 계획으로 바꾸기 전에 무엇이 참이어야 하는지 판단할 때 사용합니다. 이것은 계획 인계일 뿐입니다. 그 자체로 런타임/서버 구현을 허가하지 않으며, 정확한 schema, DDL, fixture 의미, runtime contract를 정의하지 않습니다.

첫 구현 계획은 v0.2 First User-Value Slice, 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack), roadmap automation이 아니라 v0.1 Core Authority Smoke 계획부터 시작한다는 뜻입니다. 편집 정리는 필요하지만 그것만으로 충분하지 않습니다. 스키마/설계 결정과 단계 경계 결정은 담당 문서에서 정리되거나, 서버 코딩 전에 단계별 전달 계획에 단계 영향과 함께 기록되어야 합니다. 아래 조건이 모두 참일 때만 첫 구현 계획을 시작할 수 있습니다.

- v0.1 API subset이 accepted 상태다.
- v0.1 DDL이 accepted 상태다.
- State transitions가 accepted 상태다.
- Write Authorization lifecycle이 accepted 상태다.
- Artifact/evidence ref shape가 accepted 상태다.
- Structured blocker shape가 accepted 상태다.
- Local access posture가 accepted 상태다.
- v0.2 promotion criteria가 accepted 상태다.

- Root README, docs README, 언어별 README, Build 문서, 관련 Reference 문서에서 저장소 정체성이 명확하다. 지금은 문서 전용이며, 향후 역할은 하네스 서버 소스 저장소이고, 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있으며, 제품 저장소나 하네스 런타임 홈이 아니다.
- 사용자가 보는 흐름이 내부 용어를 먼저 알아야만 시작, 재개, unblock, 작업 수락, close를 할 수 있는 형태가 아니다.
- Discovery와 요구사항 확인은 Change Unit 또는 first safe implementation unit으로 너무 빨리 수렴하지 않고, shared understanding과 사용자 소유 판단을 먼저 보존한다. Owner path가 필요로 할 때 Change Unit이 범위가 정해진 작업을 표현할 수 있지만, Discovery 자체가 조기 Change Unit 선택은 아니다.
- 판단 모델이 Kernel, MCP/API schema, storage, template, fixture, Learn/Use 설명, glossary term과 schema-aligned 상태다. `judgment_category`는 사용자에게 보이는 판단 묶음을 담당하고, `judgment_route`는 internal owner path와 recorded-answer route를 담당하며, `display_depth`는 prompt depth를 담당한다. Affected gate와 blocked action은 별도의 owner field에 남는다.
- Decision Packet prompt는 판단의 크기에 맞다. 작은 명시적 unblocker는 `display_depth=simple`을 사용할 수 있고, trade-off, high-risk, close-affecting, approval-shaped, waiver, 작업 수락, residual-risk acceptance, reconcile, mixed prompt는 owner contract가 요구하는 추가 context를 담되 모든 작은 판단을 무겁게 만들지 않는다.
- Approval, 작업 수락, 잔여 위험 수용이 예시, template, API/schema 문구, close behavior, user-facing routing에서 분리되어 있다.
- Stage가 일관적이다. v0.1 Core Authority Smoke는 제품 MVP가 아니고, v0.2 First User-Value Slice는 첫 narrow user-value slice이지 full assurance/Eval/QA/reporting system이 아니며, v0.3 Agency Assurance Pack은 verification, QA, residual risk, work acceptance, stewardship를 단단하게 만들고, v0.4 Operations & Handoff Pack은 operational handoff capability를 추가하며, v1+ Expansion은 승격 전까지 roadmap 범위다.
- Kernel, API, storage, reference, Build contract가 Core ownership, state transition, write authority, evidence, judgment record, close semantics, idempotency, state conflict behavior, artifact, projection job, fixture semantics에서 서로 맞는다.
- Storage와 API는 단계화되어 있다. Reference schema와 DDL은 future-profile field나 table을 정의할 수 있지만, implementation은 active method, record, profile을 따른다. Future-profile 존재 자체가 v0.1 범위를 넓히지는 않는다.
- 읽기용 요약(Projection) 범위가 단계화되어 있고 권한이 없다. 읽기용 요약과 card는 Core record와 artifact ref에서 파생되며, 권한을 만들거나 첫 증명이 되지 않는다.
- 보안 보장이 실제 enforcement level과 맞다. Cooperative, detective, preventive, isolated 표현은 해당 동작에 대해 문서화된 surface와 fixture-proven path가 있을 때만 사용한다.
- Agent context 전략이 정의되어 있다. 항상 주입되는 맥락은 한 화면 안팎으로 유지하고 현재 Task 요약, 작업 모양, 범위/하지 않을 일, 대기 중인 사용자 판단, 활성 막힘, 다음 안전한 행동, 근거 공백, 닫기 막힘, 잔여 위험 요약, 보장 수준, 출처 참조와 최신성만 담는다. 전체 Reference 문서, schema, DDL, historical log, 읽기용 요약 본문, artifact contents, 관련 없는 template, future catalog material은 알맞은 담당 문서/조회 경로로만 가져온다.
- Conformance fixture plan이 단계화되고 향후 검증 계획으로 유지된다. Kernel Smoke는 작은 smoke check를 위한 좁은 v0.1 작성 label일 뿐이고, 이후 suite profile은 v0.2 First User-Value Slice, 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack), 승격된 v1+ item에 맞으며, fixture file, 향후 fixture catalog, full v0.1 conformance suite, runnable conformance test가 이미 존재한다고 암시하지 않는다.
- Operations surface가 단계화되어 있다. Minimal local status/diagnostic behavior는 초기 stage를 도울 수 있지만, doctor/readiness, reconcile, recover, export, artifact check, release handoff, conformance run entrypoint는 owner stage가 포함하기 전까지 v0.1 requirement가 아니다.
- 한국어 사용자 대상 문서는 읽기 쉽고 일관적이다. 자연스러운 한국어를 먼저 쓰고, stable schema identifier, API name, enum value, DDL name, file name, validator ID, official product/stage name은 정밀도가 필요할 때 정확히 유지한다.
- Link, TODO, terminology, 영어/한국어 의미 일치가 정리되어 있다. Active docs에 흩어진 unresolved major-decision TODO가 없고, 서버 코딩 전 필요한 구현 결정은 [단계별 전달 계획](mvp-plan.md#서버-코딩-전-필요한-구현-결정)의 open ledger에 기록되어 있다.
- 유지보수자가 구현 계획 준비 상태를 수락하기 전에 마지막 docs-maintenance drift pass를 완료한다. 남은 항목은 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목 중 하나로 명시되어 있다. 문서 검토에는 막힘이 아니지만 구현 계획이나 서버 코딩 전에는 막힘이라면 그 이후 막힘을 이름 붙인다. Docs-maintenance는 읽기 전용 문서 점검으로 남습니다. [문서 작성 가이드](../maintain/authoring-guide.md#docs-maintenance-checks)와 [운영과 Conformance 참조](../reference/operations-and-conformance.md#docs-maintenance-프로필)를 봅니다.
- v0.1 Core Authority Smoke의 local-only MCP 노출 baseline이 수락되어 있다. Remote, shared, tunneled, non-loopback 노출은 담당 문서가 connector profile을 승격하고 증명하기 전까지 v0.1 baseline 밖입니다. [런타임 아키텍처](../reference/runtime-architecture.md#로컬-접근-기대사항), [보안 위협 모델 참조](../reference/security-threat-model.md#mcp-local-access와-caller-boundary), [MCP API와 스키마](../reference/mcp-api-and-schemas.md#mcp-경계와-호출자-신뢰)를 봅니다.
- 첫 authority path를 실행하는 데 사용하는 reference-surface capability가 실제 host/profile/configuration에 대한 구체적인 declaration으로 수락되어 있다. 넓은 connector profile과 surface recipe detail은 [Agent 통합 참조](../reference/agent-integration.md#capability-profiles)와 [Surface Cookbook](../reference/surface-cookbook.md)에 둡니다.
- Core-only mutation model이 수락되어 있다. 기준 운영 상태를 변경하는 것은 Core뿐이며, resource, projection, report, diagnostic, MCP caller, operator entrypoint는 Core의 상태 변경 경로에 들어가지 않는 한 read-only 또는 derived로 남습니다. [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), MCP [Idempotency](../reference/mcp-api-and-schemas.md#idempotency)와 [State Conflict 동작](../reference/mcp-api-and-schemas.md#state-conflict-동작)을 봅니다.
- 커널 스모크(Kernel Smoke) fixture queue가 v0.1 Core Authority Smoke conformance 작성 순서이자 향후 적합성 검증 계획으로 확인되어 있다. 정확한 fixture format, assertion, catalog semantics는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)에 둡니다. 이 checkpoint는 fixture file이나 runnable conformance test가 이미 존재한다는 뜻이 아닙니다.
- 첫 실행 가능한 조각은 로컬, 단일 프로젝트, minimal authority loop 범위를 유지한다. 계획 점검 목록은 [첫 실행 가능한 조각](first-runnable-slice.md)을 사용합니다.
- v1+ Expansion 기능은 [로드맵 단계 승격 조건](../roadmap.md#단계-승격-조건)에 따라 담당 문서가 승격하기 전까지 v0.1 Core Authority Smoke, v0.2 First User-Value Slice, 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack) 밖에 남아 있다.

이 인계는 roadmap 항목, dashboard 또는 hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystem 또는 marketplace, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics 또는 long-term metrics, parallel orchestration을 v0.1 Core Authority Smoke, v0.2 First User-Value Slice, 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack)으로 승격하지 않습니다. 정확한 계약은 Reference 문서에 두고, 이 섹션은 짧은 readiness checkpoint로만 사용합니다.

## 증명 경계

| 경계 | 증명하는 것 | 사용자 또는 운영자가 관찰할 수 있는 것 |
|---|---|---|
| v0.1 Core Authority Smoke | 하나의 로컬 Task가 첫 Core 권한 루프를 통과할 수 있음을 증명합니다. 여기에는 local project registration, active Task, Reference 계약상 필요한 경우에만 Change Unit 소유자 형태로 표현되는 scoped boundary 하나, `prepare_write`, single-use 쓰기 허가 기록, `record_run`, artifact/evidence ref 하나, 구조화된 상태/막힘 응답이 포함됩니다. | 상태/막힘 출력이 현재 Task, scope, 쓰기 권한, artifact/evidence support, blocker를 보여 줍니다. `prepare_write`가 범위 밖 쓰기 권한을 거절하고, 호환되는 scoped work는 권한을 받아 한 번만 사용되며, scope, write authority, 또는 artifact/evidence support가 없으면 status 또는 좁은 close-task smoke가 구조화된 막힘과 함께 거절합니다. |
| v0.2 First User-Value Slice | 평범한 사용자 작업이 scope, non-goals, success criteria, user-owned judgment, evidence summary, 닫기 막힘 요약, 작업 수락 display, 잔여 위험 표시로 정리됨을 증명합니다. | 사용자는 work shape classification, product/UX judgment와 architecture judgment의 분리, 사용자에게 묻기 전 codebase-answerable fact 확인, small direct change와 tracked work의 다른 procedural budget, ambiguous feature request의 clarification 보류, required evidence 또는 필요한 user-owned judgment 누락 시 닫기 막힘, residual risk display, ambiguous consent가 judgment resolution으로 취급되지 않는 점, MCP/Core unavailable 상태가 authority를 만들어내지 않는 점, Core-derived 간결한 상태 카드(compact status card), active profile/user request/task type/risk profile이 요구할 때만 필요한 detached verification, required verification을 일부러 건너뛸 때만 필요한 waiver, Approval과 residual-risk acceptance에서 분리된 작업 수락을 볼 수 있습니다. |
| 에이전시 보증 팩(v0.3 Agency Assurance Pack) | v0.2 user-value path가 verification, 수동 QA, 잔여 위험 수용 close, 작업 수락 분리, stewardship, profile별 Decision Packet 품질, Approval separation, TDD, feedback-loop policy, context hygiene를 정직한 경계 안에서 처리함을 증명합니다. | Fixture가 같은 Core record와 error를 통해 work가 진행, 검증, 수동 QA 요구, 작업 수락, 잔여 위험 수용, close될 수 있는지 보여 줍니다. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Operator readiness, recover/export, artifact integrity, release handoff, broader fixture suite coverage, later-boundary checks가 [강화된 로컬 기준 목표](../reference/glossary.md#강화된-로컬-기준-목표)를 완성합니다. | Operator 진입점이 두 번째 authority model을 만들지 않고 같은 Core state 위에서 diagnose, recover, export, artifact check, conformance run, release handoff 준비를 수행합니다. |
| Roadmap 경계: v1+ Expansion | 로컬 kernel과 agency 증명이 안정된 뒤에만 later surface 또는 automation을 검토할 수 있음을 분리합니다. | 선택 capability는 담당자가 [로드맵 단계 승격 조건](../roadmap.md#단계-승격-조건)에 따라 exact contract와 fixture로 승격하기 전까지 read-only, display-only, metadata-only, 또는 artifact 후보 제공 전용으로 남습니다. |

## 무엇을 만드는가

유지보수자 인계가 첫 런타임 배치를 위한 구현 계획 준비 상태를 명시적으로 수락한 뒤, 하네스 구현은 이 저장소에서 v0.1 Core Authority Smoke로 시작합니다. 이것은 로컬 작업 장부와 판단 라우터를 위한 내부 kernel입니다. v0.2 First User-Value Slice가 그 장부를 사용자 가치로 보여 주는 첫 단계입니다. v0.1은 authority loop를 확인하는 데 필요한 지속 로컬 상태, 쓰기 허가 기록, Run 기록, artifact/evidence ref, 구조화된 상태/막힘 출력만 유지합니다. v0.2는 ordinary-language start/resume, work-shape classification, minimal judgment routing, evidence summary, 닫기 막힘 요약, residual-risk visibility, Core-derived 간결한 상태 카드(compact status card)를 더합니다. 전체 작업 여정을 다루는 projection과 다듬어진 report는 later derived output으로 남깁니다. 제품 이력, 실행 가능한 확인, 리뷰, 사용자 판단은 기존 엔지니어링 절차에 남겨 둡니다. 사용자 판단권을 보존하는 로컬 권한 커널 원칙은 구현의 중심에 남습니다. Core가 기준 로컬 상태를 소유하고, 사용자 소유 판단은 사용자에게 남습니다. 초기 구현 가정은 명확한 내부 모듈을 가진 하나의 로컬 시스템이며, 분산 플랫폼으로 시작하지 않습니다.

아래 섹션은 그 런타임 배치의 향후 책임을 설명합니다. 현재 문서 수락 단계의 작업 지시가 아닙니다.

### Local Server / Process

MCP 경계를 제공하고, Core 전이를 소유하며, 하네스 런타임 홈을 읽고 쓰는 로컬 하네스 서버/설치 프로세스 하나를 계획합니다. 검증기 실행, 읽기용 요약 작업 대기열 추가, reconcile, 복구, export, conformance 진입점은 이후 단계 또는 profile-specific capability이며, 범위에 들어올 때 모두 같은 Core 규칙 위에서 실행되어야 합니다.

v0.1 Core Authority Smoke은 모듈을 가진 단일 프로세스로 충분합니다. Core, projection, validation, 운영자 도구를 별도 서비스로 나눌 필요는 없습니다.

### Core

Core는 운영 상태의 기준 기록을 변경하는 유일한 경로입니다. 향후 Core 구현은 [런타임 아키텍처](../reference/runtime-architecture.md#state-transaction-flow)가 담당하는 transaction order를 따라야 합니다. 순서는 envelope와 state-version validation, lock 획득, 현재 상태 읽기, 범위에 들어온 owner check 또는 validator, record update, owner-required event append, projection support가 범위에 있을 때의 optional projection job enqueue, commit입니다. Build 계층에서 요약하면 Core는 다음을 해야 합니다.

- 새 mutation 전에 tool envelope, idempotency key, expected state version을 검증한다
- 필요한 project 또는 task lock을 획득한다
- 현재 기록을 읽는다
- Core check와 active stage가 요구하는 validator만 실행한다
- Core transaction 안에서 현재 기록을 갱신하고, owner-required event를 추가하며, projection support가 범위에 있을 때만 projection 작업을 대기열에 넣는다
- 결과를 설명하는 막힘과 참조를 반환한다

Agent, MCP tool, 운영자 명령, projector, recovery flow는 Core를 통하거나 같은 Core compatibility rule을 보존해야 합니다. 어느 것도 두 번째 기준 상태 모델을 유지하면 안 됩니다.

### State Store

State store는 권한 루프의 기준 운영 상태를 보관합니다. v0.1에서는 project와 Task state, scoped boundary, write authority, Run 하나, artifact/evidence ref 하나, 상태/막힘 출력에 필요한 최소 owner record를 뜻합니다. Judgment record, projection/reconcile tracking, full Evidence Manifest behavior, Eval, 수동 QA, 더 넓은 event history는 이후 단계 또는 owner-profile 범위입니다.

Build 계층에서 이를 새로 설계하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

### Artifact Store

Artifact store는 오래 보존해야 하는 근거 파일과 integrity metadata를 보관합니다. Raw artifact는 diff, log, screenshot, bundle, manifest, checkpoint, export component, 그 밖의 근거 파일이 될 수 있습니다.

Artifact store는 느슨한 파일 덤프가 아닙니다. 하네스 상태를 뒷받침하는 artifact는 artifact owner path로 등록하고, 이를 사용하는 Task 또는 owner record와 연결해야 합니다. Exact artifact ref, integrity field, redaction state, retention rule은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#artifactref)와 [Storage와 DDL](../reference/storage-and-ddl.md#artifact-directory-layout)이 담당합니다.

### MCP API

향후 MCP server contract는 read resource와 public tool을 제공합니다. MCP resource는 read-only입니다. 상태를 변경하는 작업은 public tool과 Core를 거칩니다.

MCP server에 닿을 수 없으면 해당 call path에서 기준이 되는 Core response가 없습니다. 첫 구현은 이를 MCP unavailable로 보고하고, 선언된 local caller 또는 surface guarantee level이 있다면 그 실제 수준에 따라 write-capable work를 보류하며, cached projection, generated file, chat text에서 상태를 만들어 내지 않아야 합니다.

v0.1 Core Authority Smoke에서는 다음만 우선합니다.

- 현재 Core 상태를 읽는 minimal status/blocker read
- 첫 Task와 scope를 만들거나 seed하는 owner-valid path 하나
- write-authority path: `prepare_write`, compatible single-use Write Authorization 하나, `record_run`
- artifact/evidence owner path 하나
- missing scope, missing write authority, 또는 missing artifact/evidence support에 대한 structured blocker 동작

v0.2 First User-Value Slice에서는 같은 API surface를 필요한 만큼만 넓힙니다. Ordinary request가 tracked work를 시작하거나 이어 가고, work shape를 분류하며, scope/non-goals/success criteria를 요약하고, minimal user judgment를 요청/기록하며, status/next output, evidence summary, 닫기 막힘 요약, residual-risk visibility, 민감 동작 승인 / 작업 수락 / 잔여 위험 수용 분리 표시를 제공하는 정도입니다.

Public request와 response 규칙은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가 담당합니다.

State conflict와 idempotency replay 동작도 그 public tool 계약의 일부입니다. Build code는 [Idempotency](../reference/mcp-api-and-schemas.md#idempotency)와 [State Conflict 동작](../reference/mcp-api-and-schemas.md#state-conflict-동작) 담당 섹션을 사용하고, 지속 저장 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)에 맡깁니다.

### 읽기용 요약(Projection)

읽기용 요약(Projection)은 Core state record와 아티팩트 참조에서 나온 읽기용 파생 보기입니다. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 기준 상태가 아닙니다.

읽기용 요약 출력은 그것이 의존하는 Core 원천 기록에서 파생합니다. 예를 들어 Task, gate, Run, artifact, 근거, Eval, QA, 그 밖의 owner record가 존재한 뒤 그 기록에서 나와야 합니다. v0.1 Core Authority Smoke은 full projection renderer나 여러 projection kind를 요구하지 않습니다. 최소 상태/막힘 출력이면 충분합니다. Owner path가 이미 freshness/read fact를 만든 경우에만 이를 보고할 수 있지만, 읽기용 요약 렌더링이 첫 증명은 아닙니다. v0.2 First User-Value Slice는 현재 작업 상태, next output, 사용자 판단 요청/기록, evidence summary, 닫기 막힘 요약, 작업 수락 display, residual-risk visibility를 사용자가 이해할 만큼의 읽기 쉬운 요약 또는 card로 제공해야 합니다. 이 산출물은 사용자 경험을 지원할 뿐이며, v0.2를 projection, evidence, reporting, Eval, QA, operations, dashboard pack으로 만들지 않습니다. 읽기용 요약 템플릿은 권한을 만들거나, 근거를 충족하거나, 상태를 대체하거나, 상태 모델을 정하거나, 첫 증명이 될 수 없습니다.

이후 단계는 원천 기록이 존재하거나 변경되고 owner profile이 승격할 때 optional, future, diagnostic `ProjectionKind` value를 켤 수 있습니다. `ProjectionKind` value와 API 소유 지원 계층은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#shared-schemas)가 담당합니다.
[읽기용 요약(Projection) 참조](../reference/document-projection.md#템플릿-구현-계층)는 Projection 권한 경계, source-record rule, freshness rule, 템플릿 구현 계층을 담당하고, [Template 참조](../reference/templates/README.md)는 rendered template body와 display card를 담당합니다.

Projection failure는 committed Core 상태를 롤백하면 안 됩니다. 읽기용 요약이 최신인지 또는 job 상태가 어떤지 표시하고, repair나 reconcile은 이후 action에 맡깁니다. `source_state_version`과 freshness는 display/readiness fact입니다. Close/readiness output은 읽기용 보기가 오래되었거나 failed임을 보여줘야 하지만, stale Markdown이 work를 authorize하거나 close를 충족하거나 현재 Core state, 소스 관리, 테스트, 리뷰를 대체할 수는 없습니다.

사람이 편집할 수 있는 읽기용 요약 섹션은 proposal surface입니다. 구현 경로는 proposal -> reconcile item -> accepted Core state-changing action과 `task_events` row, 또는 reject, defer, note로 라우팅해야 합니다. Managed block direct edit는 drift이지 state change가 아닙니다.

### Operator Commands

Operator 진입점은 Core 동작 위에 놓이는 경로이지 두 번째 상태 모델이 아닙니다. 넓은 v0.1 요구사항이 아닙니다. 관련 stage 또는 owner profile이 범위에 넣을 때만 command-independent 기능으로 계획합니다.

| Stage | 운영자 capability boundary |
|---|---|
| v0.1 Core Authority Smoke | 최소 connect/register, 기본 상태 또는 진단 읽기, 첫 조각이 그 boundary를 요구할 때만 local MCP/API exposure. |
| v0.2 First User-Value Slice | 현재 작업, 사용자 판단, 근거 상태, 닫기 막힘, 작업 수락 필요 여부/상태, 민감 동작 승인 display, 잔여 위험 표시를 위한 user-facing status/next diagnostic. |
| v0.3 Agency Assurance Pack | Verification, Manual QA, residual risk, 작업 수락, stewardship, context hygiene를 위한 assurance-profile diagnostic과 owner-path support. |
| v0.4 Operations & Handoff Pack | Full local operations입니다. Doctor/readiness, 읽기용 요약 refresh, reconcile, recover, export, artifact integrity, 담당 문서가 정의한 release handoff, suite가 materialized된 뒤 conformance run을 포함합니다. |
| v1+ Expansion | Remote/shared operations, dashboard, broad connector automation, team workflow, orchestration, higher automation은 승격 뒤에만 포함합니다. |

정확한 command name과 flag는 나중에 정해도 됩니다. 중요한 것은 command-independent behavior contract입니다. Operator 동작은 MCP tool과 같은 Core state, `task_events`, artifacts, projections, 기존 error 또는 diagnostics를 사용합니다. 상태를 변경하는 operator outcome은 Core 또는 Core ordering을 보존하는 문서화된 recovery path에 들어가야 하며, operator output이 별도 state truth가 되면 안 됩니다.

## 아직 만들지 않는 것

첫 구현 계획은 좁게 유지합니다. 아래 항목은 담당 문서가 승격하기 전까지 선행 조건으로 만들지 않습니다.

| Capability | Stage boundary |
|---|---|
| Dashboard, hosted workflow UI, rich UI | v0.1부터 v0.4까지 authority, 근거, close readiness, 작업 수락, 잔여 위험 수용이 아닙니다. |
| Broad connector ecosystem 또는 marketplace | 담당 문서가 승격하기 전까지 첫 local authority path를 넘어 staged delivery 범위를 넓히지 않습니다. |
| Context Index | 읽기 전용 v1+ 후보입니다. Authority 또는 read/write prerequisite가 아닙니다. |
| Browser QA Capture | v1+ 후보입니다. Required automation, 수동 QA 대체물, 작업 수락 대체물이 아닙니다. |
| Cross-Surface Verification | v1+ automation 후보입니다. 분리 검증은 이 automation 없이 local path에서 먼저 증명할 수 있습니다. |
| Native hook expansion, Advanced Sidecar Watcher, preventive guard expansion | Capability-dependent enhancement입니다. Proven concrete pre-tool block 또는 observation path가 있어야 주장할 수 있습니다. |
| Local Derived Metrics 또는 long-term metrics | 읽기 전용 diagnostics입니다. Staged-delivery-critical state, authority, readiness가 아닙니다. |
| Team workflow, shared workspaces, permissions, profile import/export, parallel orchestration | Future coordination scope입니다. Local single-project authority path의 필수 요소가 아닙니다. |

v0.1 Core Authority Smoke은 협력형(cooperative) 또는 탐지형(detective) guard/freeze 상태를 표시할 수 있고, existing scope, 이미 존재하는 경우의 Autonomy Boundary, `prepare_write` 동작을 통해 작업을 보류하거나 범위를 좁힐 수 있습니다. 접점 label만으로 저장된 guarantee level이 올라가지는 않습니다.

유용한 향후 capability라도 담당 문서가 capability profile, redaction/secret/PII policy, 필요한 경우 retention 또는 test-environment rule, fixture coverage, fallback 동작, 읽기용 요약을 기준으로 삼는 의존성 없음을 정의하기 전까지는 읽기 전용 표시, metadata, 기존 owner path를 위한 artifact 후보, fixture candidate로만 나타날 수 있습니다. v0.1 Core Authority Smoke을 실행하거나, v0.2 First User-Value Slice를 완료하거나, staged-delivery close readiness를 주장하기 위한 전제 조건이 되어서는 안 됩니다.

## 첫 증명

첫 실행 목표는 v0.1 Core Authority Smoke입니다. 하네스가 하나의 권한 결정을 만들고 적용할 수 있음을 보여 주는 가장 작은 실행 가능한 경로입니다. 커널 스모크(Kernel Smoke)는 이 목표의 smoke check를 위한 좁은 향후 작성 label이지 전체 conformance suite가 아닙니다.

v0.1은 내부 authority loop를 증명하는 단계입니다. Product MVP, template 완성도, broad automation을 증명하는 단계가 아닙니다.

다음을 보여야 합니다.

- 등록된 local project 하나
- 현재 Core-owned state를 가진 Task 하나
- intended change를 위한 범위가 정해진 작업 경계 하나
- `prepare_write`가 compatible scope 없는 write authorization을 거절하고 compatible scoped write 하나를 허용함
- 허용된 `prepare_write`가 지속적이며 한 번만 쓸 수 있는 쓰기 허가 기록을 만듦
- `record_run`이 direct Run 또는 implementation Run에서 그 쓰기 허가 기록을 한 번 사용한 것으로 기록하고 observed changes를 기록함
- artifact/evidence ref 하나를 등록하고 Run 또는 minimal owner relation에 연결할 수 있음
- 상태/막힘 출력이 mutation을 만들지 않음
- scope, write authority, 또는 artifact/evidence support가 없으면 status 또는 close-task smoke가 구조화된 막힘과 함께 차단함
- 같은 동작을 향후 작은 Kernel Smoke candidate에 매핑할 수 있음

v0.1 Core Authority Smoke은 v0.2 First User-Value Slice가 아닙니다. 쓰기 권한 경로가 살아 있음을 증명하는 단계입니다. 문서 수준 수락 점검은 [첫 실행 가능한 조각](first-runnable-slice.md#문서-수준-수락-점검)을 사용하고, 정확한 fixture 의미는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)를 사용합니다.

## 첫 사용자 가치 증명

첫 사용자 가치 목표는 v0.2 First User-Value Slice입니다. v0.1 Core Authority Smoke 뒤에 도달하는 목표이지 첫 실행 batch를 키워서 만들지 않습니다. 이 단계는 평범한 요청이 작은 범위의 하네스 작업으로 보이게 하고, 하네스가 scope, non-goals, success criteria, user-owned judgment, evidence summary, 닫기 막힘 요약, 작업 수락 display, residual-risk visibility를 로컬 권한 기록에 보존한다는 점을 사용자에게 보여 줍니다. 근거와 간결한 상태 출력은 지원 수단이지 full product system 자체가 아닙니다.

다음을 보여야 합니다.

- ordinary user language가 Harness vocabulary를 요구하지 않고 tracked work를 시작하거나 resume할 수 있음
- work shape가 분류되고 small direct change와 tracked work가 구분됨
- work가 scope, non-goals, success criteria, 근거 기대 수준, close readiness, judgment boundaries로 정리됨
- Codebase-answerable 또는 state-answerable fact는 사용자에게 반복 질문하기 전에 확인됨
- Clarification은 blocking question과 useful-but-not-blocking question을 분리하고 긴 questionnaire가 되지 않음
- product/UX judgment와 기술 구조 판단를 분리해 제시할 수 있음
- minimal user judgment request와 record가 full Decision Packet machinery 없이 존재함
- small direct changes와 tracked work가 authority를 우회하지 않고 서로 다른 procedural budget을 사용함
- ambiguous feature request는 premature implementation이 아니라 clarification으로 들어감
- required 근거 또는 user judgment가 없으면 close가 block됨
- 닫기 관련 위험이 있으면 작업 수락 또는 close 전에 잔여 위험이 보임
- "go ahead", "looks good", "좋아", "진행해" 같은 ambiguous consent가 ambiguous route 해소나 risk acceptance로 취급되지 않음
- MCP/Core unavailable status가 authority를 만들어내지 않음
- 사용자의 작업 수락이 sensitive-action Approval과 잔여 위험 수용과 구분됨
- 잔여 위험 수용을 지원하는 경우, 이것이 작업 수락과 뚜렷하게 구분되어 보임
- 간결한 상태 카드(compact status card)가 Core records에서 파생되며, template polish가 기준 권한이 되지 않아도 충분함
- detached verification은 기본 요구사항이 아님
- verification은 active profile, user request, task type, risk profile이 요구할 때만 필요함
- verification waiver는 required verification을 일부러 건너뛸 때만 필요함

## 강화된 로컬 기준 증명

[강화된 로컬 기준 목표](../reference/glossary.md#강화된-로컬-기준-목표)(hardened local reference target)는 v0.2 First User-Value Slice 이후 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)을 통해 도달하는 향후 reference 목표이지 첫 구현 batch가 아닙니다. 별도 stage, fixture profile, alternate implementation path도 아닙니다. Agent가 정직한 경계 안에서 행동하기 위해 필요한 나머지 conformance를 추가합니다.

- Decision Packet 품질과 사용자 판단 라우팅
- sensitive-action Approval, Decision Packet, 쓰기 허가 기록의 분리
- 작업 수락과 close 전에 잔여 위험을 표시하는 규칙
- 분리 검증 독립성
- 수동 QA 기록과 QA 차단 조건
- feedback-loop, TDD, stewardship, context-hygiene validators
- 읽기용 요약과 reconcile 완전성
- recovery, export, artifact integrity 동작
- 담당 문서가 정의하는 release handoff report/export behavior
- broad automation을 v1+ Expansion에 두는 later 경계 확인
- 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack), 또는 승격된 v1+ Expansion 후보 fixture를 통한 fixture coverage

강화된 로컬 기준 목표(hardened local reference target)는 향후 conformance가 생성된 문장이나 renderer output만이 아니라 Core 상태, events, artifacts, projection/freshness facts, errors로 동작을 증명할 때 완료됩니다.

## Build 읽기 경로

Build 계층은 다음 순서로 읽습니다.

1. [구현 개요](implementation-overview.md): 현재 상태, 유지보수자 인계, 향후 시스템 모양을 확인합니다.
2. [단계별 전달 계획](mvp-plan.md): v0.1부터 v0.4까지의 단계별 전달, 단계 경계, 서버 코딩 전 결정 기록을 확인합니다.
3. [첫 실행 가능한 조각](first-runnable-slice.md): v0.1 구현 순서를 확인합니다.
4. [Runtime Walkthrough](runtime-walkthrough.md): request-to-close runtime path를 확인합니다.

v1+ Expansion 후보와 승격 규칙은 [로드맵](../roadmap.md)을 사용합니다.

그다음 정확한 동작은 [Reference 색인](../reference/README.md)에서 현재 owner를 골라 확인합니다.

- [커널 참조](../reference/kernel.md): entity, gate, state logic, `prepare_write`, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): runtime space, Core flow, artifact, projection/reconcile, guarantee level.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool, schema, error, 아티팩트 참조, idempotency, state conflict behavior.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout, staged DDL, migration, lock, artifact와 later-profile baseline, projection job, validator-run candidate를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator semantics와 conformance run overview.
- [Conformance Fixtures 참조](../reference/conformance-fixtures.md): 핵심 적합성 모델, fixture body shape, assertion semantics, 축소된 Kernel Smoke queue.
- [향후 Fixture Catalog](../reference/future-fixture-catalog.md): 그 자체로 early-stage requirement가 아닌 detailed later scenario candidate.
