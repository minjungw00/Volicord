# 템플릿 참조

## 사용 시점

MVP-1의 작은 보기들이 어떤 형태로 렌더링되는지 확인할 때 이 디렉터리를 사용합니다. 읽기용 요약 규칙, 권한 경계, 관리 영역 동작, 아티팩트 참조 표시, 최신성 동작은 [Projection과 Template 참조](../projection-and-templates.md)가 담당합니다.

권한 규칙:

- 템플릿은 보기이지 권한 상태가 아닙니다.
- 사용자 템플릿은 읽기 쉬움을 우선합니다.
- 에이전트 템플릿은 다음 안전한 행동에 필요한 작고 정확한 맥락을 우선합니다.
- 렌더링된 보기는 민감 동작 승인, 작업 수락, 잔여 위험 수용, 근거, 닫기 준비 상태, Write Authorization, close를 만들 수 없습니다.
- 대화, Markdown, 상태 카드, 에이전트 맥락 패킷, 보고서는 Core 상태를 덮어쓸 수 없습니다.
- 저장소에 템플릿이 있다는 사실만으로 MVP-1 요구사항이 되지는 않습니다.

Owner 경계: 이 디렉터리는 렌더링된 템플릿 본문과 표시 카드 형태를 담당합니다. 기준 kernel state, MCP schema, SQLite DDL, gate, artifact storage, conformance, operations behavior, implementation readiness는 정의하지 않습니다. 현재 저장소 단계와 인계 상태는 [구현 개요](../../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 산출물 계층

| 계층 | 템플릿 범위 | 규칙 |
|---|---|---|
| 내부 엔지니어링 점검 상태 | Plain structured status/blocker output. 선택적으로 [상태 카드](status-card.md)를 렌더링할 수 있습니다. | Projection job이나 full renderer가 필요하지 않습니다. |
| MVP-1 사용자 작업 루프 보기 | [상태 카드](status-card.md), [에이전트 맥락 패킷](agent-context-packet.md), [판단 요청](judgment-request.md), [실행/근거 요약](run-evidence-summary.md), [닫기 결과](close-result.md) | 이것이 정확한 전체 MVP-1 템플릿/보기 세트입니다. 각 보기는 Core 상태와 참조에서 파생됩니다. |
| Later/full-profile 템플릿 | [later-profile/](later-profile/README.md) | 상세 보증, 진단, 운영, export, stewardship, 전체 보고서 템플릿은 owner profile이 명시적으로 승격하기 전까지 later-profile로 남습니다. |

## 템플릿 구현 계층

| 계층 | 템플릿 | 처음 활성화되는 단계 | 메모 |
|---|---|---|---|
| 사용자 상태 | [상태 카드](status-card.md) | MVP-1 사용자 작업 루프 | 사용자가 읽는 짧은 현재 상태 보기입니다. 기본 사용자 상태 보기입니다. |
| 에이전트 다음 행동 맥락 | [에이전트 맥락 패킷](agent-context-packet.md) | MVP-1 지원 보기 | 다음 안전한 행동에 필요한 참조, 막힘, source clock, 최신성, owner section pointer를 작게 담습니다. |
| 사용자 소유 판단 질문 | [판단 요청](judgment-request.md) | MVP-1 사용자 작업 루프 | 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 위한 간결한 질문입니다. 전체 Decision Packet 표시는 later/full-profile입니다. |
| 실행과 근거 요약 | [실행/근거 요약](run-evidence-summary.md) | MVP-1 사용자 작업 루프 | 최소 Run, 확인, 근거 참조, 아티팩트 참조, 가림 처리, 공백 요약입니다. 상세 Run Summary와 Evidence Manifest는 later/full-profile입니다. |
| 닫기 표시 | [닫기 결과](close-result.md) | MVP-1 사용자 작업 루프 | 닫기 준비 상태, 작업 수락, 잔여 위험, 막힘, 가장 작은 해소 방법, 닫기 결과를 보여줍니다. 상세 Journey, direct-result, export, release-handoff report는 later/full-profile입니다. |

## MVP-1 템플릿 세트

MVP-1 템플릿/보기는 정확히 다음 다섯 개로 제한됩니다.

- [상태 카드](status-card.md): 사용자가 보는 짧은 현재 상태.
- [에이전트 맥락 패킷](agent-context-packet.md): 다음 안전한 행동을 위한 작은 맥락.
- [판단 요청](judgment-request.md): 사용자 소유 판단 요청.
- [실행/근거 요약](run-evidence-summary.md): 최소 실행과 근거 요약.
- [닫기 결과](close-result.md): 닫기 준비 상태, 작업 수락, 잔여 위험, 막힘.

이 다섯 보기는 접점에 따라 structured payload, 짧은 text, card, Markdown snippet으로 반환될 수 있습니다. MVP-1은 persisted Markdown projection job, full renderer, 모든 상세 report template을 요구하지 않습니다.

## Later/Full-Profile 템플릿

상세 템플릿은 [later-profile/](later-profile/README.md)에 둡니다. 상태: MVP-1 요구사항 아님, 구현된 런타임 아님. Later profile에서 유용할 수 있지만, 존재한다고 해서 런타임이 구현했다는 뜻도 아닙니다.

| 버킷 | 템플릿 | 경계 |
|---|---|---|
| 보증 프로필 | [DEC / Decision Packet](later-profile/decision-packet.md), [APR](later-profile/approval.md), [Approval Card](later-profile/approval-card.md), [EVIDENCE-MANIFEST](later-profile/evidence-manifest.md), [EVAL](later-profile/eval.md), [MANUAL-QA](later-profile/manual-qa.md), [수동 QA Card](later-profile/manual-qa-card.md), [Verification Result Card](later-profile/verification-result-card.md) | Owner profile이 active일 때의 검증 강화, 수동 QA, 상세 근거, 위험 검토, 상세 평가 출력에만 사용합니다. |
| 운영 프로필 | [EXPORT](later-profile/export.md) | Operations/export path가 active일 때의 export, handoff, artifact availability, redaction/omission, release-handoff display에만 사용합니다. |
| Future/diagnostic profile material | [TASK](later-profile/task.md), [DIRECT-RESULT](later-profile/direct-result.md), [JOURNEY-CARD](later-profile/journey-card.md), [DESIGN](later-profile/design.md), [DOMAIN-LANGUAGE](later-profile/domain-language.md), [MODULE-MAP](later-profile/module-map.md), [INTERFACE-CONTRACT](later-profile/interface-contract.md), [RUN-SUMMARY](later-profile/run-summary.md), [TDD-TRACE](later-profile/tdd-trace.md) | Detailed continuity, stewardship, TDD, diagnostic, reporting view는 owner가 non-required display 또는 later-stage scope로 승격하기 전까지 later-profile에 남습니다. |

Dashboard, hosted workflow, team workflow, broader connector, automation, analytics view는 template requirement가 아니라 [로드맵](../../roadmap.md) 향후 후보입니다.

## 메모

Source record나 ref가 없으면 `none`, `unknown`, `not_required`, 또는 blocking/unavailable note로 렌더링합니다. Template completeness를 맞추려고 placeholder state를 만들면 안 됩니다.

큰 log, diff, trace, screenshot, recording, bundle, export component, 민감한 artifact body는 기본적으로 본문에 embed하지 말고 `ArtifactRef`로 참조합니다. `redaction_state`를 보존하고, 생략/차단 note는 보여주되 생략되거나 차단된 원본 값을 재구성하지 않습니다.
