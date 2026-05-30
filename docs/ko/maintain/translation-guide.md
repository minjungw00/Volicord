# 번역 가이드

## 이 문서로 할 수 있는 일

영어와 한국어 하네스 문서를 함께 고칠 때 이 가이드를 사용합니다.

이 문서는 이중 언어 문서 유지보수를 위한 Maintain 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data, 제품 상태 변경을 승인하지 않으며, conformance pass/fail, evidence, QA, acceptance, close readiness, implementation readiness를 정의하지 않습니다. 첫 제품 MVP 목표는 v0.1 Kernel MVP이며, Kernel Smoke는 이를 좁게 실행하는 conformance profile입니다. v0.2 Evidence & Projection Pack, v0.3 Agency Pack, v0.4 Operations Pack은 Agency-Hardened MVP reference conformance target으로 가는 staged pack입니다. v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 남습니다.

## 이런 때 읽기

- 영어 또는 한국어 문서의 의미를 바꿀 때.
- 영어/한국어 의미 일치를 review할 때.
- 한국어 문장이 영어 identifier를 그대로 유지해야 하는지, 자연스러운 한국어 prose를 써야 하는지 판단할 때.

## 먼저 읽기

Owner boundary, docs-maintenance check, strict contract가 Reference 문서에 머문다는 규칙은 [문서 작성 가이드](authoring-guide.md)를 봅니다.

## 핵심 생각

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어 문서는 한국어 기술 문서답게 자연스럽게 읽혀야 하며, 공식 식별자(official identifier), 정확한 계약, 코드처럼 쓰이는 이름, 안정적인 제품 용어(product term)는 흔들리지 않아야 합니다.

사용자용 한국어에서는 자연스러운 한국어 표현을 먼저 두고, 정확한 하네스 라벨이 도움이 될 때만 뒤에 붙입니다. 예를 들어 독자 친화 표현과 라벨이 모두 필요하면 `범위(Change Unit)`, `결정 패킷(Decision Packet)`, `쓰기 허가 기록(Write Authorization)`, `잔여 위험(Residual Risk)`, `수동 QA(Manual QA)`, `분리 검증(detached verification)`, `결과 수락(final acceptance)`처럼 씁니다.

## 그대로 유지할 것

다음 항목은 영어와 한국어 문서에서 그대로 유지합니다.

- API names
- schema names
- enum values
- DDL names
- code identifiers
- file names and path names
- error codes and validator IDs

다음 이름은 literal identifier, schema/API value, file/template name, heading anchor, code-like reference처럼 정확한 문자열을 가리킬 때 그대로 씁니다. 일반 한국어 prose에서 개념을 설명할 때는 아래 [한국어 기준 용어](#한국어-기준-용어)를 우선합니다.

- Task
- Change Unit
- Decision Packet
- Write Authorization
- Evidence Manifest
- ProjectionKind
- MCP
- Core
- state.sqlite
- task_events
- prepare_write
- record_run
- close_task

`HARNESS:BEGIN` 같은 marker, `ArtifactRef`, `ProjectionKind`, `decision_kind=approval`, `approval_gate`, `ResidualRiskSummary.status=none`, validator ID, error code, file path, API/tool/schema 이름은 번역하지 않습니다.

Exact stage label은 그대로 유지합니다. 예를 들어 `v0.1 Kernel MVP`, `Kernel Smoke`, `v0.2 Evidence & Projection Pack`, `v0.3 Agency Pack`, `v0.4 Operations Pack`, `Agency-Hardened MVP`, `Agency-Hardened MVP reference conformance target`, `v1+ Expansion`은 label 자체를 바꾸지 않습니다. 세 공간 모델을 한국어 prose로 설명할 때는 `제품 저장소`, 이 저장소의 향후 source 역할을 가리킬 때는 `하네스 서버 소스 저장소`, 운영 데이터 공간은 `하네스 런타임 홈`을 사용합니다. Architecture term을 구분해야 할 때만 영어 label을 괄호로 덧붙입니다.

Lookup anchor로 쓰이는 Reference heading은 전용 link/anchor migration으로 모든 link를 함께 고치지 않는 한 안정적으로 유지합니다. 사용자-facing prose에서는 자연스러운 한국어를 우선합니다. 안정적인 Reference heading 아래에는 한국어 alias line으로 자연스러운 표현을 제공할 수 있습니다.

## 한국어 기준 용어

아래 용어는 한국어 prose에서 우선 사용하는 기준 표현입니다. 정확한 identifier나 계약 값이 필요한 곳에서는 영어 문자열을 그대로 유지하고, 독자에게 도움이 될 때 한국어 표현 뒤에 괄호로 붙입니다.

| English term | 한국어 기준 표현 | 사용 메모 |
|---|---|---|
| Harness | 하네스 | 제품명을 일반 prose에서 쓸 때 사용합니다. `HARNESS:BEGIN` 같은 marker나 literal string은 유지합니다. |
| Product Repository | 제품 저장소 | 사용자의 제품 작업 공간을 가리킵니다. 세 공간 모델을 구분해야 할 때만 영어 label을 병기합니다. |
| Harness Server source repository | 하네스 서버 소스 저장소 | 문서 승인 이후 이 저장소가 맡을 future source-code 역할을 가리킵니다. |
| Harness Runtime Home | 하네스 런타임 홈 | 사용자별/설치별 운영 데이터 공간을 가리킵니다. 독자에게 도움이 될 때만 영어 label을 병기합니다. |
| durable local state | 지속 로컬 상태 | 첫 사용에서 `지속 로컬 상태(durable local state)`처럼 병기할 수 있습니다. |
| artifact ref | 아티팩트 참조 | Evidence 문맥에서는 `증거 아티팩트 참조`도 가능합니다. `ArtifactRef` schema name은 유지합니다. |
| projection | 읽기용 투영 문서 | `projection freshness`, `ProjectionKind`, API field, template kind를 말할 때는 `projection`을 유지할 수 있습니다. 일반 설명에서는 `읽기용 보기`도 자연스럽습니다. |
| kernel | 커널 | 공식 heading이나 owner link가 아니라면 `커널`을 씁니다. |
| gate | 관문 | Use/Learn 문서에서는 `관문`을 우선합니다. Reference 문서에서 kernel field나 value를 가리킬 때는 `gate`를 유지할 수 있습니다. |
| Decision Packet | 결정 패킷 | Record ID, API/schema 이름, heading anchor 등 literal context에서는 `Decision Packet`을 유지할 수 있습니다. |
| Write Authorization | 쓰기 허가 기록 | `prepare_write` 결과나 record를 설명하는 prose에서 사용합니다. API/tool 이름과 exact field는 유지합니다. |
| Residual Risk | 잔여 위험 | 사용자용 prose도 `남은 위험` 대신 `잔여 위험`을 기준으로 맞춥니다. 쉬운 설명이 필요하면 `남은 불확실성`처럼 풀어 쓸 수 있습니다. |
| Manual QA | 수동 QA | Exact template/schema/API context에서는 `Manual QA`를 유지합니다. |
| detached verification | 분리 검증 | 보장 수준이나 assurance 설명에서 `detached verification`을 병기할 수 있습니다. |
| cooperative | 협력형 | Guarantee-level table에서는 English label도 유지합니다. |
| detective | 탐지형 | Guarantee-level table에서는 English label도 유지합니다. |
| preventive | 예방형 | Guarantee-level table에서는 English label도 유지합니다. |
| isolated | 격리형 | Guarantee-level table에서는 English label도 유지합니다. |

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
| acceptance / final acceptance | 사용자가 결과를 받아들인다는 판단이면 문맥에 따라 `수락`, `결과 수락`, `최종 수락`을 씁니다. |
| acceptance criteria | 공식 acceptance criteria를 가리키면 `수용 기준`을 씁니다. 정식 기준보다 작업 완료 조건을 말하는 문맥이면 `완료 기준`도 가능합니다. `수락 기준`은 쓰지 않습니다. |
| residual-risk acceptance / accepted risk | 사용자용 문장에서는 `잔여 위험을 받아들이는 판단`, `잔여 위험을 받아들이다`를 우선합니다. 참조 문서처럼 더 엄격한 문맥에서는 주변 문체에 맞으면 `위험 수락`이나 정확한 하네스 식별자를 쓸 수 있습니다. |
| Acceptance Gate / acceptance_gate | 필요한 경우 `Acceptance Gate` 또는 `acceptance_gate` 같은 식별자를 그대로 유지합니다. 불안정한 새 번역어를 만들지 말고 한국어 문장으로 뜻을 설명합니다. |
| residual risk | 기준 표현은 `잔여 위험`입니다. 쉬운 설명에서 남는 불확실성을 풀어 말할 수 있지만, 문서 전체 용어는 `잔여 위험`으로 맞춥니다. |
| approval / Approval | 기준 하네스 sensitive-action permission 개념, 상태, gate, 기록을 이름 붙일 때는 `Approval`을 유지합니다. 한국어 일반 문장에서 하네스 개념이 아닌 보통 의미일 때만 `승인`을 씁니다. |
| write authority | 일반 문장에서는 `쓰기 권한`을 쓸 수 있습니다. `prepare_write`가 남기는 하네스 기록을 말할 때는 `쓰기 허가 기록(Write Authorization)`을 씁니다. |
| gate | 사용자용 흐름에서는 `관문`, `확인`, `닫기 확인`, `막힘`처럼 문맥에 맞는 말을 우선합니다. Reference 문서에서 kernel field나 strict contract를 가리킬 때는 `gate`를 유지할 수 있습니다. |

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
| `acceptance가 필요하다.` | `결과 수락이 필요하다.` |
| `risk를 accept한다.` | `잔여 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 결과 수락처럼 쓴다.` | `잔여 위험을 받아들이는 판단`처럼 결과 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |
| `Harness 상태는 local state와 artifact ref에 있다.` | `하네스 상태는 지속 로컬 상태와 아티팩트 참조에 있다.` |
| `detached verification을 독립 검증으로 표시한다.` | `분리 검증(detached verification)으로 표시한다.` |

## 한국어 제목 원칙

한국어 제목은 영어 제목을 기계적으로 옮기지 않습니다.

Official identifier 자체를 설명하는 제목이라면 identifier를 정확히 유지합니다. 그렇지 않다면 한국어 기술 독자가 자연스럽게 이해할 제목을 고릅니다.

문서의 heading order와 scope는 영어 문서와 맞춥니다. 하지만 heading text가 단어 단위로 일치할 필요는 없습니다.

## 이중 언어 리뷰 체크리스트

```text
[ ] 한국어 페이지가 영어 페이지와 같은 의미를 보존하는가?
[ ] Paired file이 같은 active file path, reader purpose, semantic section coverage, owner link, contractual detail을 유지하는가?
[ ] 한국어 문장이 한국어 기술 독자에게 자연스럽게 읽히는가?
[ ] API name, schema name, enum value, DDL name, identifier, path, error code, validator ID가 정확한가?
[ ] 일반 prose의 하네스 기준 용어가 일관되고, exact identifier는 그대로 유지되었는가?
[ ] Source-of-truth phrase와 owner link가 owner Reference docs와 맞는가?
[ ] Owner가 아닌 중복 contract는 전체 contract 번역이 아니라 요약과 owner link로 처리되었는가?
[ ] 불필요한 혼합어 표현을 가능한 한 자연스러운 한국어로 바꾸었는가?
[ ] 사용자용 문서에서 한국어 표현과 하네스 라벨이 모두 필요할 때 자연스러운 한국어가 먼저 나오는가?
[ ] 제목이 자연스러우면서도 같은 문서 구조와 범위를 유지하는가?
[ ] 영어와 한국어 링크 변경이 같은 batch에 들어갔는가?
[ ] Review가 translation drift를 runtime state, evidence, QA, acceptance, close readiness, implementation readiness로 취급하지 않는가?
```
