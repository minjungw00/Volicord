# 번역 가이드

## 목적

영어와 한국어 Harness 문서를 함께 고칠 때 이 가이드를 사용합니다.

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어 문서는 한국어 기술 문서답게 자연스럽게 읽혀야 하며, 공식 식별자(official identifier), 정확한 계약, 제품 용어(product term)는 흔들리지 않아야 합니다.

사용자용 한국어에서는 자연스러운 한국어 표현을 먼저 두고, 정확한 Harness 용어가 도움이 될 때만 뒤에 붙입니다. 예를 들어 독자 친화 표현과 Harness 라벨이 모두 필요하면 `범위(Change Unit)`, `판단 자료(Decision Packet)`, `쓰기 권한(Write Authorization)`, `남은 위험(residual risk)`, `결과 수락(final acceptance)`처럼 씁니다.

## 그대로 유지할 것

다음 항목은 영어와 한국어 문서에서 그대로 유지합니다.

- API names
- schema names
- enum values
- DDL names
- code identifiers
- file names and path names
- error codes and validator IDs

다음 stable product terms는 Harness 개념을 가리킬 때 그대로 씁니다.

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
| residual-risk acceptance / accepted risk | 사용자용 문장에서는 `남은 위험을 받아들이는 판단`, `남은 위험을 받아들이다`를 우선합니다. 참조 문서처럼 더 엄격한 문맥에서는 주변 문체에 맞으면 `위험 수락`이나 정확한 Harness 식별자를 쓸 수 있습니다. |
| Acceptance Gate / acceptance_gate | 필요한 경우 `Acceptance Gate` 또는 `acceptance_gate` 같은 식별자를 그대로 유지합니다. 불안정한 새 번역어를 만들지 말고 한국어 문장으로 뜻을 설명합니다. |
| residual risk | 사용자용 문장에서는 `남은 위험`을 씁니다. 문서가 더 격식 있는 기술 문체를 이미 쓰고 있다면 `잔여 리스크`도 가능합니다. |
| approval | 일반 문장에서는 `승인`을 씁니다. Harness 개념, 상태, 기록을 이름 붙일 때는 `Approval`을 유지합니다. |
| write authority | 일반 문장에서는 `쓰기 권한`을 씁니다. Harness 기록을 이름 붙일 때는 `Write Authorization`을 정확히 유지합니다. |
| gate | 엄격한 계약을 가리킬 때는 `게이트`를 씁니다. 사용자용 흐름에서는 `확인`, `닫기 확인`, `막힘`처럼 더 구체적인 말을 우선합니다. |

## 피할 표현

영어 기술어를 그대로 끼워 넣었지만 독자에게 더 선명해지지 않는 표현은 피합니다.

- `상태 변경을 영어 동사와 섞어 쓴다`
- `authority boundary 유지라고 쓴다`
- `surface 표시라고 쓴다`
- `projection freshness를 report한다`
- `acceptance를 complete한다`
- `risk를 accept한다`
- `acceptance criteria를 수락 기준으로 쓴다`

## 더 나은 표현

의미는 보존하되 한국어 문장으로 자연스럽게 씁니다.

- `상태를 변경한다`
- `권한 경계를 유지한다`
- `화면에 보여준다`
- `projection이 최신인지 표시한다`
- `결과를 수락한다`
- `남은 위험을 받아들인다`
- `수용 기준을 확인한다`

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
| `risk를 accept한다.` | `남은 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 결과 수락처럼 쓴다.` | `남은 위험을 받아들이는 판단`처럼 결과 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |

## 한국어 제목 원칙

한국어 제목은 영어 제목을 기계적으로 옮기지 않습니다.

Official identifier 자체를 설명하는 제목이라면 identifier를 정확히 유지합니다. 그렇지 않다면 한국어 기술 독자가 자연스럽게 이해할 제목을 고릅니다.

문서의 heading order와 scope는 영어 문서와 맞춥니다. 하지만 heading text가 단어 단위로 일치할 필요는 없습니다.

## 이중 언어 리뷰 체크리스트

```text
[ ] 한국어 페이지가 영어 페이지와 같은 의미를 보존하는가?
[ ] 한국어 문장이 한국어 기술 독자에게 자연스럽게 읽히는가?
[ ] API name, schema name, enum value, DDL name, identifier, path, error code, validator ID가 정확한가?
[ ] Harness 개념을 가리키는 stable product term이 그대로 유지되었는가?
[ ] 불필요한 혼합어 표현을 가능한 한 자연스러운 한국어로 바꾸었는가?
[ ] 사용자용 문서에서 한국어 표현과 Harness 라벨이 모두 필요할 때 자연스러운 한국어가 먼저 나오는가?
[ ] 제목이 자연스러우면서도 같은 문서 구조와 범위를 유지하는가?
[ ] 영어와 한국어 링크 변경이 같은 batch에 들어갔는가?
```
