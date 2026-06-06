# 템플릿 참조

## 사용 시점

MVP-1의 작은 보기들이 어떤 형태로 렌더링되는지 확인할 때 이 디렉터리를 사용합니다. 읽기용 요약 규칙, 권한 경계, 관리 영역 동작, 아티팩트 참조 표시, 최신성 동작은 [Projection과 Template 참조](../projection-and-templates.md)가 담당합니다.

권한 규칙:

- 템플릿은 보기이지 권한 상태가 아닙니다.
- 사용자 템플릿은 읽기 쉬움과 판단 지원을 우선합니다.
- 에이전트 템플릿은 다음 안전한 행동에 필요한 작고 정확한 맥락을 우선합니다.
- 사용자용 템플릿과 에이전트용 패킷은 독자가 다릅니다. 하나를 다른 독자에게 그대로 재사용하지 않습니다.
- 렌더링된 보기는 민감 동작 승인, 최종 수락, 잔여 위험 수락, 증거, 닫기 준비 상태, Write Authorization, close를 만들 수 없습니다.
- 대화, Markdown, 상태 카드, 에이전트 맥락 패킷, 보고서는 Core 상태를 덮어쓸 수 없습니다.
- 저장소에 템플릿이 있다는 사실만으로 MVP-1 요구사항이 되지는 않습니다.

Owner 경계: 이 디렉터리는 렌더링된 템플릿 본문과 표시 카드 형태를 담당합니다. 기준 kernel state, MCP schema, SQLite DDL, gate, artifact storage, conformance, operations behavior, implementation readiness는 정의하지 않습니다. 현재 저장소 단계와 인계 상태는 [MVP 계획](../../build/mvp-plan.md#문서-수락-상태)에 있습니다.

## 산출물 계층

| 계층 | 템플릿 범위 | 규칙 |
|---|---|---|
| 내부 엔지니어링 점검 상태 | Plain structured status/blocker output. 선택적으로 [상태 카드](status-card.md)를 렌더링할 수 있습니다. | Projection job이나 full renderer가 필요하지 않습니다. |
| MVP-1 사용자용 작은 출력 | [상태 카드](status-card.md), [판단 요청](judgment-request.md), [실행/증거 요약](run-evidence-summary.md), [닫기 결과](close-result.md) | 이것이 정확한 전체 MVP-1 사용자용 출력 세트입니다. 각 출력은 Core 상태와 참조에서 파생되며, 평범한 말을 쓰고 불필요한 내부 스키마 필드를 피합니다. |
| MVP-1 에이전트용 작은 출력 | [에이전트 맥락 패킷](agent-context-packet.md) | 활성 MVP-1의 유일한 에이전트용 패킷입니다. Task/Change Unit 참조, state/source 참조, 활성 범위, 해결되지 않은 사용자 판단, 차단 사유, 하나의 다음 안전한 행동, 증거 공백, 닫기 차단 사유, 활성 상태일 때 잔여 위험 요약, 보장 수준만 담습니다. |
| Later/full-profile 템플릿 | [Later template 후보](../../later/index.md#later-template-candidates) | 상세 보증, 진단, 운영, export, stewardship, 전체 보고서 템플릿은 owner profile이 명시적으로 승격하기 전까지 candidate-only로 남습니다. |

## 템플릿 구현 계층

| 계층 | 템플릿 | 처음 활성화되는 단계 | 메모 |
|---|---|---|---|
| 사용자 상태 | [상태 카드](status-card.md) | MVP-1 사용자 작업 루프 | 사용자가 읽는 짧은 현재 상태 보기입니다. 기본 사용자 상태 보기입니다. |
| 에이전트 다음 행동 맥락 | [에이전트 맥락 패킷](agent-context-packet.md) | MVP-1 지원 보기 | 다음 안전한 행동에 필요한 참조, 차단 사유, source clock, 최신성, 증거 공백, 닫기 차단 사유, 활성 상태일 때 잔여 위험 요약, 보장 수준, 하나의 다음 행동을 작게 담습니다. 사용자용 문장이 아닙니다. |
| 사용자 소유 판단 질문 | [판단 요청](judgment-request.md) | MVP-1 사용자 작업 루프 | 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단을 위한 간결한 질문입니다. 전체 Decision Packet 표시는 later/full-profile입니다. |
| 실행과 증거 요약 | [실행/증거 요약](run-evidence-summary.md) | MVP-1 사용자 작업 루프 | 최소 Run, 확인, 증거 참조, 아티팩트 참조, 가림 처리, 가용성, 공백 요약입니다. 상세 Run Summary와 상세 증거 보고서는 later/full-profile입니다. |
| 닫기 표시 | [닫기 결과](close-result.md) | MVP-1 사용자 작업 루프 | 활성 MVP 닫기 준비 상태, 최종 수락 상태, 잔여 위험, 차단 사유, 가장 작은 해소 방법, 닫기 결과를 보여줍니다. 상세 Journey, direct-result, export, release-handoff, 나중 보증 보고서는 later/full-profile입니다. |

## MVP-1 템플릿 세트

MVP-1 활성 작은 출력은 독자별로 정확히 아래 형태로 제한됩니다.

- 사용자용 [상태 카드](status-card.md): 짧은 현재 상태.
- 사용자용 [판단 요청](judgment-request.md): 사용자 소유 판단 요청.
- 사용자용 [실행/증거 요약](run-evidence-summary.md): 최소 실행과 증거 요약.
- 사용자용 [닫기 결과](close-result.md): 닫기 준비 상태, 최종 수락, 잔여 위험, 차단 사유.
- 에이전트용 [에이전트 맥락 패킷](agent-context-packet.md): 다음 안전한 행동을 위한 Core 기반 참조.

네 가지 사용자용 출력은 접점에 따라 짧은 텍스트, 카드, Markdown snippet, structured payload로 반환될 수 있습니다. 에이전트용 패킷은 structured payload 또는 prompt 크기의 텍스트가 될 수 있습니다. MVP-1은 persisted Markdown projection job, full renderer, 모든 상세 report template을 요구하지 않습니다.

사용자용 출력은 하네스 내부 지식 없이 읽을 수 있어야 합니다. 사용자가 판단하거나, 차단 사유를 이해하거나, 증거를 살피는 데 도움이 될 때 짧은 ref와 최신성을 보여줄 수 있습니다. 하지만 전체 스키마, 필드 목록, DDL, event log, 전체 보고서 본문, 전체 아티팩트 목록, future/profile catalog를 노출하지 않습니다.

에이전트용 패킷은 prose 기준으로 상태 카드보다 작게 유지합니다. 사용자 카드가 생략하는 ref를 담을 수는 있지만, 전체 Reference 문서, 전체 스키마, 전체 DDL, historical log, 전체 projection 본문, 전체 artifact 내용, 관련 없는 template, future catalog를 주입하면 안 됩니다.

MVP-1 출력에는 [설계 품질 정책](../design-quality-policies.md#영향-분류와-허용-라우트)이 소유하는 routed action을 통해서만 design-quality finding을 표시할 수 있습니다. Action은 block write, block close, ask one focused user judgment, request evidence, mark residual risk, show advisory next action, no action 중 하나입니다. Full policy catalog를 기본 close checklist로 렌더링하지 않습니다.

## Later/Full-Profile 템플릿

상세 template body는 active documentation에서 폐기하고 [Later template 후보](../../later/index.md#later-template-candidates)에 요약했습니다. 상태: MVP-1 요구사항 아님, 구현된 런타임 아님. Later profile에서 유용할 수 있지만, 목록에 있다는 사실이 런타임 구현을 뜻하지 않습니다.

| 버킷 | 템플릿 | 경계 |
|---|---|---|
| 보증 프로필 | [DEC / Decision Packet](../../later/index.md#later-template-candidates), [APR](../../later/index.md#later-template-candidates), [Approval Card](../../later/index.md#later-template-candidates), [EVIDENCE-MANIFEST](../../later/index.md#later-template-candidates), [EVAL](../../later/index.md#later-template-candidates), [MANUAL-QA](../../later/index.md#later-template-candidates), [수동 QA Card](../../later/index.md#later-template-candidates), [Verification Result Card](../../later/index.md#later-template-candidates) | Owner profile이 active일 때의 검증 강화, 수동 QA, 상세 증거, 위험 검토, 상세 평가 출력에만 사용합니다. |
| 운영 프로필 | [EXPORT](../../later/index.md#later-template-candidates) | Operations/export path가 active일 때의 export, handoff, artifact availability, redaction/omission, release-handoff display에만 사용합니다. |
| Future/diagnostic profile material | [TASK](../../later/index.md#later-template-candidates), [DIRECT-RESULT](../../later/index.md#later-template-candidates), [JOURNEY-CARD](../../later/index.md#later-template-candidates), [DESIGN](../../later/index.md#later-template-candidates), [DOMAIN-LANGUAGE](../../later/index.md#later-template-candidates), [MODULE-MAP](../../later/index.md#later-template-candidates), [INTERFACE-CONTRACT](../../later/index.md#later-template-candidates), [RUN-SUMMARY](../../later/index.md#later-template-candidates), [TDD-TRACE](../../later/index.md#later-template-candidates) | Detailed continuity, stewardship, TDD, diagnostic, reporting view는 owner가 non-required display 또는 later-stage scope로 승격하기 전까지 later-profile에 남습니다. |

Dashboard, hosted workflow, team workflow, broader connector, automation, analytics view는 template requirement가 아니라 [로드맵](../../later/index.md#roadmap-candidates) 향후 후보입니다.

## 메모

Source record나 ref가 없으면 `none`, `unknown`, `not_required`, 또는 blocking/unavailable note로 렌더링합니다. Template completeness를 맞추려고 placeholder state를 만들면 안 됩니다.

큰 log, diff, trace, screenshot, recording, bundle, export component, 민감한 artifact body는 기본적으로 본문에 embed하지 말고 `ArtifactRef`로 참조합니다. 사용자용 MVP-1 출력은 integrity, owner, availability, redaction을 평범한 말로 요약할 수 있습니다. Diagnostic과 later/full-profile 출력은 그 세부 정보가 필요할 때 정확한 integrity metadata를 보여줄 수 있습니다. 생략/차단 note는 항상 보여주되 생략되거나 차단된 원본 값을 재구성하지 않습니다.
