# 소개

## 문서 역할

사용자와 구현자가 함께 읽는 공유 정신 모델입니다.

## 담당 범위

- Harness가 줄이는 문제 요약
- 세 공간 모델 요약
- Task, Change Unit, Strategic Agency, Decision Gate, Decision Packet, Evidence, Journey Card, Journey Spine, Residual Risk, Projection의 기본 개념
- `advisor`, `direct`, `work` 소개
- Journey Card 예시
- source-of-truth 경계 요약

## 담당하지 않는 범위

- 구현 schemas
- 상태 전이표
- tool schema
- template 전문

## 섹션

### Harness가 필요한 이유

AI 지원 개발은 빠르게 움직입니다. 하지만 중요한 작업 사실은 종종 chat 안에 갇힙니다. 사용자가 무엇을 요청했는지, 어떤 범위에 합의했는지, 어떤 설계 방향을 골랐는지, 무엇이 바뀌었는지, 어떤 evidence가 있는지, 어떤 승인이나 제품 판단이 아직 필요한지, 결과를 실제로 확인했는지, 어떤 Residual Risk가 남았는지가 대화 속에 흩어집니다.

Harness는 이런 작업에 사용자의 전략적 판단권을 보존하는 로컬 운영 커널을 제공합니다. 대화는 자연스럽게 이어가되, 오래 남아야 하는 작업 상태는 chat 밖에 기록합니다. 그래서 Task는 기억이 아니라 현재 상태를 기준으로 따라가고, resume하고, verify하고, reconcile하고, close할 수 있습니다.

짧게 말하면:

```text
Harness는 명시적인 상태, 범위, 결정, evidence, verification, QA, acceptance, residual risk를 통해 사용자의 전략적 판단권을 보존하면서 AI 작업 여정을 따라갈 수 있게 합니다.
```

### 세 공간

Harness는 세 공간을 분리합니다.

| 공간 | 독자 수준의 의미 |
|---|---|
| Product Repository | 사용자의 실제 product workspace입니다. code, tests, 생성된 readable reports, 사람이 편집할 수 있는 proposal areas가 여기에 있습니다. |
| Harness Server / Installation | 로컬 harness process와 tools입니다. MCP server, Core, validators, projector, connectors, operator commands가 여기에 속합니다. |
| Harness Runtime Home | 로컬 운영 저장소입니다. project registration, project별 state, durable evidence artifacts가 여기에 있습니다. |

이 분리는 product files, 생성된 Markdown, chat text, operational state가 서로 뒤섞이지 않게 합니다. 정식 architecture 세부 내용은 [04-runtime-architecture.md](04-runtime-architecture.md)가 담당합니다.

### 핵심 개념

- Task는 사용자 가치 단위입니다. 사용자가 끝내거나 답을 얻고 싶은 일입니다.
- Change Unit은 product writes를 허가하는 범위 지정 구현 단위입니다.
- Strategic Agency는 사용자가 작업 여정을 이해하고 목표, 범위, 설계, 트레이드오프, Codebase Stewardship, QA, acceptance, Residual Risk에 대해 판단하거나 보류할 수 있는 지속적인 권한입니다.
- Decision Gate는 제품 판단이 기록될 때까지 진행이 막히는 지점입니다.
- Decision Packet은 blocking product judgment에 필요한 결정, options, trade-offs, evidence, affected scope, Residual Risk, next action을 기록합니다.
- Evidence는 작업에 대한 주장을 뒷받침하는 기록입니다. 예를 들어 diffs, logs, tests, screenshots, run summaries, Eval records, Manual QA records가 있습니다.
- Raw artifact는 artifact store 안에 보존되는 durable evidence file입니다.
- Journey Spine은 Task, Change Units, decisions, runs, evidence, QA, acceptance, Residual Risk가 state에서 파생되어 순서대로 이어지는 작업 여정입니다.
- Journey Card는 그 여정에서 현재 위치를 압축해 보여주는 projection입니다.
- Projection은 state records와 artifact refs를 사람이 읽을 수 있는 Markdown으로 렌더링한 것입니다.
- Reconcile은 human-editable notes나 projection drift를 accepted state changes, rejected proposals, notes, decisions, deferred items로 바꾸는 명시적 경로입니다.

자세한 entity와 gate 모델은 [03-kernel-spec.md](03-kernel-spec.md)가 담당합니다. Projection rules는 [07-document-projection.md](07-document-projection.md)가 담당합니다.

### 작업 모드

Harness는 세 가지 작업 모드를 사용합니다.

| 모드 | 사용 대상 | Product writes |
|---|---|---|
| `advisor` | 설명, 비교, review, planning, decision support. | 허용되지 않습니다. |
| `direct` | 범위와 결과가 명확한 작고 위험이 낮은 변경. | 활성 scoped Change Unit 안에서만 허용됩니다. |
| `work` | feature work, structural change, risky work, multi-step implementation. | 활성 scoped Change Unit 안에서만 허용되며 보통 더 강한 evidence와 verification이 필요합니다. |

Task는 작게 시작할 수 있습니다. 범위가 커지면 Harness는 그 사실을 보이게 하고, 안전하게 실행할 수 있는 형태로 작업을 옮겨야 합니다.

### Journey Card 읽기

Journey Card는 canonical state가 아니라 파생 display입니다. 네 가지 질문에 빠르게 답하기 위해 있습니다.

- 지금 어떤 Task를 진행 중인가?
- 다음으로 안전한 action은 무엇인가?
- 어떤 user decisions 또는 gates가 진행을 막고 있는가?
- readable projection은 믿을 만큼 최신인가?

예:

```text
TASK-0044 Email login flow
State: work / shaping
Next action: decide failed-login UX
Scope: login form, login API call, session storage
Decision Gate: failed-login UX pending
Decision Packet: DEC-0012 current
Approval: dependency_change required
Evidence: none
Verification: not started
Manual QA: pending
Acceptance: pending
Residual risk: none recorded
Projection: current
```

`Manual QA: pending` 같은 친근한 label은 display text입니다. Canonical fields와 close rules는 kernel이 정의합니다.

### Source Of Truth 요약

source-of-truth 경계는 다음과 같습니다.

```text
운영 상태:
  state.sqlite current records와 state.sqlite.task_events

Raw evidence:
  artifact store의 durable files

Markdown reports:
  state records와 artifact refs에서 생성되는 projections

Human-editable sections:
  notes와 proposals를 위한 input surfaces
```

Human-editable input은 reconcile 또는 Core state-changing action이 accepted state event나 record를 기록한 뒤에만 operational truth가 됩니다.

정식 규칙은 [03-kernel-spec.md](03-kernel-spec.md), [04-runtime-architecture.md](04-runtime-architecture.md), [07-document-projection.md](07-document-projection.md)를 참고합니다.
