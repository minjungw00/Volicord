# 번역 가이드

## 목적

영어와 한국어 Harness 문서를 함께 고칠 때 이 가이드를 사용합니다.

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어 문서는 한국어 기술 문서답게 자연스럽게 읽혀야 하며, official identifier와 정확한 계약, product term은 흔들리지 않아야 합니다.

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
| context | Identifier처럼 쓰이거나 AI session 맥락을 가리키면 `context`를 유지할 수 있습니다. 일반 prose에서는 `맥락`을 씁니다. |
| boundary | Code나 identifier 맥락에서는 `boundary`를 유지합니다. 일반 prose에서는 `경계`를 씁니다. |
| authority | Operational authority는 보통 `권한`으로 옮깁니다. 권한의 source를 강조해야 하면 `기준 권한`을 씁니다. |
| canonical | Identifier 맥락에서는 `canonical`을 유지합니다. 일반 prose에서는 `기준` 또는 `기준 기록`을 씁니다. |
| change / modify | 상태나 기록을 바꾸는 의미입니다. 한국어에서는 `변경하다`를 씁니다. |
| surface | 문맥에 맞게 `interface`, `view`, `entrypoint`, `display area`나 그에 맞는 한국어 표현을 고릅니다. |
| evidence | Product term으로 필요할 때만 `evidence`를 유지합니다. 일반 prose에서는 `근거` 또는 `증거`를 씁니다. |

## 피할 표현

영어 기술어를 그대로 끼워 넣었지만 독자에게 더 선명해지지 않는 표현은 피합니다.

```text
상태 변경을 영어 동사와 섞어 쓴다
authority boundary 유지라고 쓴다
surface 표시라고 쓴다
projection freshness를 report한다
```

## 더 나은 표현

의미는 보존하되 한국어 문장으로 자연스럽게 씁니다.

```text
상태를 변경한다
권한 경계를 유지한다
화면에 보여준다
projection이 최신인지 표시한다
```

## Before / After 예시

| Before | After |
|---|---|
| `Core가 state 변경을 수행한다.` | `Core가 상태를 변경한다.` |
| `Agent는 authority boundary 유지가 필요하다.` | `Agent는 권한 경계를 유지해야 한다.` |
| `이 surface는 blocker 표시를 담당한다.` | `이 화면은 blocker를 보여준다.` |
| `Operations는 projection freshness를 report한다.` | `Operations는 projection이 최신인지 표시한다.` |
| `canonical source를 update한다.` | `기준 기록을 업데이트한다.` |
| `context를 잃지 않도록 한다.` | 일반 prose에서는 `맥락을 잃지 않도록 한다.` |

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
[ ] 제목이 자연스러우면서도 같은 문서 구조와 범위를 유지하는가?
[ ] 영어와 한국어 링크 변경이 같은 batch에 들어갔는가?
```
