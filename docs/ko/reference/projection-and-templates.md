# Projection과 템플릿 참조

## 담당하는 것 / 담당하지 않는 것

이 문서는 향후 하네스 동작을 위한 Projection과 활성 템플릿 표시 계약을 담당합니다. 문서 원천 자료일 뿐입니다. 런타임 projection, 런타임 상태, 생성된 산출물, 증거 기록, QA 기록, Acceptance 기록, 잔여 위험 기록, 닫기 기록, 구현 준비가 끝난 서버 계획이 아닙니다.

이 문서가 담당하는 것:

- Projection 권한 경계
- 파생 표시로서의 Projection
- 사람이 편집 가능한 영역 규칙
- 관리 블록 규칙
- `ArtifactRef` 렌더링 규칙
- 작은 보기의 출처/최신성 표시 규칙
- 활성 현재 MVP 템플릿 세트
- 활성 현재 MVP 템플릿 다섯 개의 전체 렌더링 본문

이 문서가 담당하지 않는 것:

- Core 상태, lifecycle, gate, `prepare_write`, `record_run`, `close_task`, 사용자 판단 상태 변경. [Core Model 참조](core-model.md)를 봅니다.
- Public MCP 요청/응답 schema, `ProjectionKind`, `ArtifactRef`, error shape. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- SQLite DDL, 저장소 배치, artifact storage, projection job storage. [Storage](storage.md)를 봅니다.
- 설계 품질 close-impact 경계. [설계 품질](design-quality.md)을 봅니다.
- 활성 Reference 범위로서의 운영자 명령 동작. 향후 운영 후보는 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다.
- Conformance fixture assertion 동작. [적합성 참조](conformance.md)를 봅니다.
- Connector 맥락 동작. [Agent 통합 참조](agent-integration.md)를 봅니다.
- Later 후보 템플릿 본문. Active documentation이 아닙니다.

## 권한 경계

Core가 소유한 상태와 등록된 아티팩트 참조가 기준입니다. Projection, 상태 카드, Markdown 보고서, 렌더링된 템플릿, 대화 메시지, connector output, 에이전트 맥락 패킷은 표시 또는 지원 맥락일 뿐입니다.

템플릿은 Core 상태를 덮어쓸 수 없습니다. 렌더링된 보기는 쓰기를 승인하거나, Write Authorization을 만들거나, gate를 충족하거나, 증거를 만들거나, 검증을 수행하거나, QA를 기록하거나, 민감 동작 승인을 부여하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, Task를 닫거나, 담당 기록을 변경할 수 없습니다. 그런 효과는 담당 Core/API 경로에서 와야 합니다.

표시 라벨은 기준 schema 값이 아닙니다. 사용자에게 보이는 판단 유형 같은 지역화 라벨은 `judgment_kind`와 locale 같은 기준 필드에서 렌더링됩니다. Compatibility 또는 response-only output에 `display_label`이 나타나도 표시 문구일 뿐입니다. Enum value, storage value, API field owner, schema category처럼 다루면 안 됩니다.

사용자가 Projection을 편집해도 그 내용은 입력일 뿐입니다. Reconcile과 Core 상태 변경 action 같은 명시적 담당 경로를 거쳐야 상태가 될 수 있습니다. 관리 텍스트, front matter, 표시된 상태, 아티팩트 참조, 닫기 상태, 최종 수락 상태, 잔여 위험 상태, 템플릿 텍스트를 직접 고쳐도 수락된 상태가 되지 않습니다.

## Projection은 파생 표시

Projection은 현재 Core 소유 기록과 등록된 `ArtifactRef` metadata에서 만든 파생 표시입니다. 사람이 작업, 출처 참조, 차단 사유, 증거 공백, 닫기 차단 사유, 최신성, 다음 안전한 행동을 읽게 돕습니다. 두 번째 상태 저장소가 아닙니다.

현재 MVP의 Projection 범위는 작습니다.

- 사용자용 작은 보기 네 가지: `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`
- 에이전트용 지원 패킷 하나: `agent-context-packet`

이 보기들은 접점에 따라 짧은 text, 카드, Markdown snippet, structured payload로 렌더링될 수 있습니다. Persisted Markdown projection job이나 full report renderer가 필요하지 않습니다. 첫 내부 smoke 목표는 렌더링된 카드 대신 plain structured status/blocker output을 반환해도 됩니다.

최신성은 source clock 위에 표시되는 정보입니다. Stale, failed, unknown, 또는 너무 넓은 읽기용 보기는 상태 변경 작업이나 닫기의 근거가 될 수 없습니다. 현재 상태가 중요하면 현재 Core state 또는 현재 Core state에서 파생된 packet을 가져와야 합니다.

원천 record나 ref가 없으면 `none`, `unknown`, `not_required`, `unavailable`, 또는 blocking note를 렌더링합니다. 템플릿을 채우려고 placeholder state를 만들지 않습니다.

Projection은 privacy 경계이기도 합니다. 생략, 가림 처리, 차단된 artifact, 가용성 note를 보여주되 생략되거나 차단된 원본 값을 재구성하지 않습니다.

## 사람이 편집 가능한 영역

명시적으로 표시된 사람이 편집 가능한 영역만 입력으로 편집할 수 있습니다. 일반적인 형태는 다음과 같습니다.

```md
User Notes and Proposals:
-
```

사람이 편집 가능한 텍스트에는 메모, 질문, 수정, 제안을 담을 수 있습니다. Task 요약, 범위, 수용 기준, 설계 메모, 증거 메모, 다른 담당 기록에 대한 변경을 제안할 수 있습니다. 하지만 제안 자체는 대상 기록이 아닙니다.

상태 변경 경로는 명시적입니다.

```text
human edit -> reconcile candidate -> explicit reconcile outcome -> Core state-changing action, rejection, deferral, or note
```

담당 경로가 accepted Core outcome을 기록하기 전까지 사람이 편집한 텍스트는 Task state, 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, 그 밖의 담당 기록이 아닙니다.

사람은 아래 항목을 직접 상태로 편집할 수 없습니다.

- 관리 블록 content
- `source_state_version` 같은 front matter field
- gate 값, lifecycle phase, result, close reason, close status, assurance level
- user judgment, 민감 동작 승인, 최종 수락, QA 면제, 검증 위험 수락, 잔여 위험 수락, close display text
- artifact identity, `sha256`, `size_bytes`, `content_type`, `redaction_state`, artifact availability
- 상태 카드, 에이전트 맥락 패킷, 생성된 보고서, 템플릿 본문

## 관리 블록 규칙

관리 블록은 projector가 소유하는 Markdown 영역입니다.

```md
<!-- HARNESS:BEGIN managed -->
...
<!-- HARNESS:END managed -->
```

규칙:

- 관리 블록 content는 committed Core-owned record와 등록된 artifact ref에서 생성됩니다.
- Projector는 관리 블록을 다시 생성할 수 있습니다.
- 관리 블록은 표시이지 권한이 아닙니다.
- 관리 블록 안을 직접 편집하면 drift입니다. Accepted state가 아닙니다.
- Projection job storage가 active이면 projector는 storage 담당 경로를 통해 source state version, projection version 또는 status, render timestamp, job status, managed hash를 기록합니다.
- 현재 MVP의 작은 보기는 persisted projection job 없이 read-time source/freshness text를 담을 수 있습니다.
- Managed hash는 marker line을 제외한 projector-owned 관리 블록 본문에서 계산합니다. Line ending은 normalize하고 projector가 의미 있다고 정한 whitespace rule을 따릅니다.
- Managed hash는 drift를 감지할 뿐입니다. Markdown을 상태로 만들지 않습니다.
- 렌더링 전에 관리 블록 hash가 마지막 projected hash와 다르면 projector는 drift를 보고하거나 담당 경로로 가는 repair candidate를 만듭니다. 편집된 블록을 조용히 accepted state로 받아들이지 않습니다.
- 다시 생성할 때 관련 없는 사람이 편집 가능한 영역은 보존해야 합니다.
- 렌더링 실패나 stale source data는 상황에 맞게 `failed`, `stale`, `unknown`, unavailable을 표시해야 합니다. Committed Core 상태를 롤백하거나, event를 바꾸거나, gate 값을 바꾸면 안 됩니다.

렌더링된 보기의 위쪽 또는 관리 summary 근처에는 짧은 경계 안내가 있어야 합니다. 표시 전용이고, Core 상태와 ref에서 파생되었으며, Write Authorization이나 닫기 결과가 아니라는 점을 보여줍니다.

## `ArtifactRef` 렌더링

큰 log, diff, trace, screenshot, recording, bundle, export component, 민감한 artifact body는 기본적으로 본문에 넣지 않고 `ArtifactRef`로 참조합니다.

독자나 다음 행동에 도움이 될 때는 다음을 렌더링합니다.

- artifact ref id
- 담당 관계 또는 affected work ref
- artifact kind 또는 source summary
- `sha256`, `size_bytes`, `content_type` 같은 integrity metadata
- `redaction_state`
- availability state
- omission, redaction, blocking note
- ref가 중요한 짧은 이유

`secret_omitted`, `blocked`, unavailable, redacted artifact body를 Markdown에 펼치지 않습니다. Metadata나 주변 문장으로 생략된 원본 값을 재구성하지 않습니다.

표시된 `ArtifactRef`는 등록된 artifact record를 가리키는 포인터입니다. 그 자체가 증거 충분성, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태는 아닙니다. Claim에 필요한 담당 관계, integrity metadata, redaction state, availability가 빠졌으면 공백을 보여줍니다.

## 활성 현재 MVP 템플릿 세트

활성 현재 MVP 템플릿 세트는 정확히 아래 다섯 개입니다.

| 독자 | 템플릿 | 본문 |
|---|---|---|
| 사용자용 | `status-card` | [상태 카드 본문](#상태-카드-본문) |
| 사용자용 | `judgment-request` | [판단 요청 본문](#판단-요청-본문) |
| 사용자용 | `run-evidence-summary` | [실행/증거 요약 본문](#실행증거-요약-본문) |
| 사용자용 | `close-result` | [닫기 결과 본문](#닫기-결과-본문) |
| 에이전트용 | `agent-context-packet` | [에이전트 맥락 패킷 본문](#에이전트-맥락-패킷-본문) |

사용자용 출력 네 가지는 평범한 말을 씁니다. 사용자가 판단하거나, 차단 사유를 이해하거나, 증거를 살피거나, 닫기를 이해하는 데 도움이 될 때만 출처 참조와 최신성을 보여줍니다. Schema, DDL, event log, 전체 artifact, 전체 report body, 전체 evidence catalog, future catalog를 쏟아내지 않습니다.

에이전트용 패킷은 별도 독자를 위한 것입니다. 현재 다음 행동에 필요한 ref, 차단 사유, 증거 공백, 닫기 차단 사유, 보장 수준, 하나의 다음 안전한 행동만 담습니다. 사용자용 prose가 아니며 권한도 아닙니다.

## 상태 카드 본문

현재 MVP에서 사용자가 현재 상태를 짧게 읽어야 할 때 `status-card`를 사용합니다. 상태 카드는 지금 무엇을 하는지, 무엇이 범위 안인지, 사용자가 무엇을 판단해야 하는지, 어떤 증거가 있거나 빠졌는지, 닫기를 무엇이 막는지, 다음 안전한 행동이 무엇인지를 보여줍니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 첫 내부 smoke 목표는 이 카드 대신 plain structured status/blocker output을 반환해도 됩니다.

경계: 이 템플릿은 렌더링된 표시일 뿐입니다. Core 상태, 증거, 민감 동작 승인, 최종 수락, 잔여 위험 수락, Write Authorization, 닫기 준비 상태 기록이 아닙니다. 최신이 아닌 대화가 아니라 현재 Core 소유 상태와 참조에서 렌더링해야 합니다.

기준 기록:

- 현재 Task 요약, 작업 모양, 다음 안전한 행동
- 사용자가 이해하는 데 필요한 현재 범위, 하지 않을 일, active Change Unit 요약
- 사용자에게 읽히는 라벨로 렌더링한 대기 중인 판단
- 진행 또는 닫기가 보류된 평이한 이유와 활성 차단 사유
- 현재 증거 요약, 뒷받침 참조, 가림 처리 또는 가용성 메모, 증거 공백
- 관련 있을 때 닫기 차단 사유, 최종 수락 필요 여부, 잔여 위험 표시, 잔여 위험 수락 상태
- 보이는 다음 행동을 바꿀 때만 설계 품질 routed action
- 보장 수준 또는 unavailable capability 상태
- 짧은 출처 참조, 렌더링 시각, 최신성 상태

렌더링 섹션:

- 작업
- 범위
- 판단
- 차단 사유
- 증거
- 확인
- 닫기
- 다음 안전한 행동
- 출처와 최신성

템플릿:

````text
{task_id} {title}
표시 전용: Core 상태와 ref에서 파생된 보기이며 Core 상태나 쓰기 승인 기록이 아닙니다.

작업: {work_shape}. {current_task_summary}
범위: {scope_summary}
범위 밖: {non_goals|none}
차단 사유: {active_blocked_reason|none}
사용자가 결정할 것: {pending_user_judgments_with_localized_labels|none}
증거: {evidence_status}. {known_evidence_summary|none}
증거 공백: {evidence_gaps|none}
확인: {check_summary|none}
닫기: {close_readiness_summary}; 차단 사유={close_blockers|none}
설계 품질 조치: {design_quality_routed_action|none}
잔여 위험: {residual_risk_visibility|none}
다음 안전한 행동: {next_safe_action}
보장 수준: {guarantee_level_or_unavailable}; {guarantee_note}
출처/최신성: {source_freshness_summary}
````

메모:

- 하네스 내부를 모르는 사용자도 읽을 수 있게 유지합니다.
- 기준 기록이 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, 또는 명시적인 차단 사유로 렌더링합니다.
- 보장 수준 줄은 항상 렌더링합니다. 현재 MVP 기본 동작에서는 실제 한계가 협력형 보류이면 그렇게 적고, 사후 보고라면 그 한계를 note에 적어야 합니다. Core/MCP가 unavailable이면 stale하거나 추측한 guarantee 대신 unavailable condition을 렌더링합니다.
- 설계 품질 내용은 한 줄에 맞춥니다. 현재 routed action과, 차단일 때는 하나의 다음 행동만 보여줍니다.
- 에이전트 전용 참조와 행동 경계 세부사항은 [에이전트 맥락 패킷 본문](#에이전트-맥락-패킷-본문)에 둡니다. 사용자가 판단하거나 차단 사유를 이해하거나 출처 최신성을 살피는 데 도움이 될 때만 상태 카드에 ref를 넣습니다.

## 판단 요청 본문

진행, 범위, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단에 영향을 주는 선택을 사용자가 소유할 때 `judgment-request`를 사용합니다. 이것은 일반 사용자 소유 판단을 위한 현재 MVP 질문 형태입니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. Full-format Decision Packet presentation은 later 후보 범위이며 [Later template 후보](../later/index.md#later-template-candidates)에 후보로만 남습니다.

경계: 이 템플릿은 대기 중이거나 기록된 `user_judgment`를 표시합니다. 이 표시만으로 판단 기록을 만들거나, Write Authorization을 부여하거나, QA 또는 검증을 수행하거나, QA 증거를 만들거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 검증 위험을 수락하거나, Task를 닫지 않습니다.

기준 기록:

- 대기 중이거나 기록된 `user_judgment`
- `judgment_kind`, `presentation`, locale에서 파생한 표시 판단 라벨
- 정확한 질문, 이유, 추천, 불확실성, 사용자가 결정하지 않을 때의 결과
- 영향을 받는 Task, Change Unit, 쓰기 범위, 닫기 범위, 민감 동작 범위, 기준 또는 다른 대상
- 선택지 또는 선택된 결과
- 결과 영향, 에이전트가 사용자 대신 판단하지 않는 것, 에이전트가 사용자 대신 판단할 수 없는 이유
- 영향을 받는 작업을 식별하는 데 필요한 최소 출처 참조
- 판단에 영향을 줄 때만 증거, 위험, 민감 동작 승인, QA, 검증, 닫기 참조

렌더링 섹션:

- 판단 요청
- 지역화된 판단 유형
- 정확한 질문
- 선택지 또는 선택된 결과
- 추천과 이유
- 불확실성
- 영향을 받는 작업
- 사용자가 결정하지 않을 때의 결과
- 에이전트가 사용자 대신 판단하지 않는 것
- 에이전트가 사용자 대신 판단할 수 없는 이유
- 다음 안전한 행동 또는 미룰 때 영향
- 참조

템플릿:

````text
판단 요청: {short_title}
유형: {localized_label_from_judgment_kind}
질문: {question}
선택지: {choices_or_selected_outcome}
추천: {recommendation|none}
왜 중요한가: {rationale}
불확실한 점: {uncertainty}
영향받는 작업: {affected_scope_summary}
결정하지 않으면: {no_decision_consequence}
제가 대신 판단하지 않을 것: {not_deciding}
답변이 필요한 이유: {why_agent_cannot_decide}
미룬다면: {deferral_effect|not_applicable}
답변 뒤 다음 안전한 행동: {next_safe_action}
참조: judgment={user_judgment_ref}; task={task_ref}; supporting={supporting_refs|none}
````

메모:

- 작은 판단은 한 화면에 들어가야 합니다. 더 자세한 장단점, 추천, 영향을 받는 gate, 증거/위험 참조, 미룰 때의 분석은 활성 담당 경로나 판단 복잡도가 요구할 때만 `presentation=full`로 보여줍니다.
- 민감 동작 승인, 제품 판단, 기술 판단, 범위 판단, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단을 하나의 넓은 승인 질문으로 합치지 않습니다.
- "yes, do it", "진행해", "좋아" 같은 채팅 문구는 scope, `judgment_kind`, affected object, recorded user intent가 pending judgment와 맞을 때만 해당 gate를 만족합니다.
- 표시되는 `유형` 라벨은 `judgment_kind`와 사용자 locale에서 렌더링합니다. 이 라벨은 표시 문구일 뿐이며, 기준 판단 범주는 `judgment_kind`입니다.

## 실행/증거 요약 본문

조언, 실행, 확인, 변경 뒤 무엇이 일어났고 현재 주장에 어떤 증거가 생겼는지 최소한으로 보여줘야 할 때 `run-evidence-summary`를 사용합니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 상세 실행 보고서와 상세 증거 목록은 later 후보 범위입니다.

경계: 이 템플릿은 Run과 증거 참조를 표시할 뿐입니다. 증거 자체, 상세 증거 목록, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태 기록이 아닙니다.

기준 기록:

- Run 참조와 command/check 요약
- 변경 경로 또는 파일 변경 없음 결과
- 관련 있을 때 소비된 Write Authorization 참조, no-write basis, 또는 attempted invalid authorization context
- 증거 참조, artifact 참조, 가림 처리, 가용성 메모
- 증거가 뒷받침하는 완료 주장, 수용 기준, 닫기 관련 주장
- 증거 공백, 최신이 아닌 입력, 아직 해결되지 않은 뒷받침 부족
- 다음 안전한 증거 행동

렌더링 섹션:

- 실행 또는 행동
- 변경 경로
- 확인
- 증거 참조
- 뒷받침하는 주장
- 공백 또는 최신이 아닌 증거
- 가림 처리와 가용성
- 다음 안전한 증거 행동

템플릿:

````text
실행/증거 요약
표시 전용: ref와 요약일 뿐이며 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기가 아닙니다.

행동: {run_or_action_summary}
변경 경로: {changed_paths|none}
확인: {checks_run_or_reason_not_run}
쓰기 확인: {write_check_summary|no product write}
증거: {evidence_status}. {evidence_summary}
증거 참조: {evidence_refs|none}
아티팩트 참조: {artifact_ref_summary|none}
가림 처리 또는 가용성: {redaction_availability_summary|none}
뒷받침하는 것: {supported_claims_or_criteria|none}
아직 빠졌거나 최신이 아닌 것: {evidence_gaps_or_stale_inputs|none}
다음 안전한 증거 행동: {next_evidence_action|none}
출처/최신성: {source_freshness_summary}
````

메모:

- 증거 충분성은 양이 아니라 coverage입니다. 현재 뒷받침하는 참조가 없거나 critical artifact ref에 담당 관계, integrity metadata, redaction state, availability가 없으면 공백과 현재 증거 상태를 보여줘야 합니다. 긴 artifact 목록이나 report 문장을 증명처럼 취급하면 안 됩니다.
- Product-write Run의 제품 쓰기 호환성 기록으로 표시할 수 있는 것은 호환되게 소비된 Write Authorization뿐입니다. Attempted invalid authorization ref는 violation/audit 또는 validator-finding context로만 보여줄 수 있으며, consumed Write Authorization이나 완료 증거처럼 렌더링하면 안 됩니다.
- 이 요약은 전체 증거 보고서보다 작게 유지합니다. 사용자의 다음 판단에 필요한 증거 참조와 보이는 공백만 보여주고, 전체 artifact inventory나 원본 artifact body를 펼치지 않습니다.

## 닫기 결과 본문

사용자가 닫기 준비 상태, 닫기 차단 사유, 또는 닫기 결과를 간결하게 봐야 할 때 `close-result`를 사용합니다. 최종 수락, 잔여 위험, 증거, 아티팩트 가용성, 자체 확인 근거, 차단 사유를 서로 분리해 보여줍니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 상세 continuity, release-handoff, export 보고서는 later 후보 범위입니다.

경계: 이 템플릿은 닫기 상태를 표시합니다. Task를 닫거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 검증 또는 QA를 기록하거나, 증거를 만들거나, gate 값을 바꾸지 않습니다. 닫기 결과는 Core close path만 만들 수 있습니다.

기준 기록:

- 현재 Task 상태와 close attempt 또는 close-readiness result
- 범위와 변경 범위 요약
- 증거 참조와 증거 공백
- active evidence summary에 포함된 자체 확인 요약
- close-relevant evidence ref에 대한 아티팩트 가용성
- 필요한 경우 최종 수락 user judgment 참조
- 관련 있을 때 잔여 위험 표시와 잔여 위험 수락 참조
- close에 영향을 줄 때 design-quality routed action. Later 후보가 active가 아니면 현재 MVP 차단 집합으로 제한합니다.
- 닫기 가능 여부, 닫기 차단 사유, 가장 작은 해결 방법
- source state version, 최신성, capability 상태

렌더링 섹션:

- 닫기 상태
- 범위
- 증거
- 아티팩트 가용성과 자체 확인 근거
- 판단과 최종 수락
- 잔여 위험
- 차단 사유
- 다음 안전한 행동
- 출처와 최신성

템플릿:

````text
닫기 상태: {ready|blocked|closed|not requested}
표시 전용: Core close state와 담당 ref가 기준입니다.

범위: {scope_summary}
증거: {evidence_status}. {evidence_summary}; 공백={evidence_gaps|none}
아티팩트 가용성: {artifact_availability_summary}
자체 확인 근거: {self_check_summary|none}
최종 수락: {final_acceptance_status}
민감 동작 승인: {sensitive_permission_status|not_needed}
설계 품질 조치: {design_quality_close_action|none}
잔여 위험: {residual_risk_visibility}
잔여 위험 수락: {residual_risk_acceptance_status|not_needed}
닫기 차단 사유: {close_blockers|none}
가장 작은 해결 방법: {smallest_unblocker|none}
닫기 근거 또는 이유: {close_reason|not_applicable}
다음 안전한 행동: {next_safe_action|none}
출처/최신성: {source_freshness_summary}
````

메모:

- 증거 요약, 아티팩트 가용성, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 차단 사유, design-quality routed action, 읽기용 보기 최신성을 하나의 "완료" 줄로 뭉개지 않습니다.
- 현재 MVP `close-result` 출력은 현재 MVP 닫기 의미만 보여줍니다. Later assurance와 상세 QA 줄은 later 후보 범위에 남습니다.
- 닫기가 막혔으면 primary blocker와 다음 행동 하나를 말하고, 다음 경로에 영향을 주는 secondary blocker만 보이게 둡니다.
- 읽기용 close view가 stale 또는 failed이면 이 템플릿의 prose에서 close하지 말고 current Core close result를 가져와야 합니다.

## 에이전트 맥락 패킷 본문

다음 안전한 행동에 필요한 현재 맥락을 에이전트가 작고 정확하게 받아야 할 때 `agent-context-packet`을 사용합니다. 이 보기는 사용자용 문장이나 전체 보고서가 아니라 최신성, Core 기반 참조, 활성 차단 사유, 해결되지 않은 사용자 판단, 증거 공백, 닫기 차단 사유, 보장 수준, 하나의 다음 행동에 최적화됩니다.

구현 계층: 현재 MVP 지원 보기입니다. Structured payload나 prompt 크기의 text로 반환할 수 있습니다. Persisted Markdown Projection이 필수는 아닙니다.

경계: 에이전트 맥락 패킷은 행동을 돕는 맥락일 뿐입니다. 쓰기를 승인하거나, gate를 충족하거나, 증거를 만들거나, 민감 동작 승인을 부여하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, Task를 닫을 수 없습니다.

기준 기록:

- Task와 active Change Unit 참조
- 현재 state version과 source ref
- 활성 범위와 하지 않을 일
- 해결되지 않은 사용자 판단
- 활성 차단 사유
- 증거 공백
- 닫기 차단 사유
- active일 때 잔여 위험 요약
- 보장 수준 또는 unavailable capability 상태
- 정확히 하나의 다음 안전한 행동

렌더링 섹션:

- Task와 Change Unit 참조
- state version과 source ref
- 활성 범위
- 해결되지 않은 사용자 판단
- 차단 사유
- 다음 안전한 행동
- 증거 공백
- 닫기 차단 사유
- 잔여 위험 요약
- 보장 수준

템플릿:

````text
agent_context_packet:
  display_only: true
  authority: none; authority는 current Core state를 사용
  task_ref: {task_ref}
  change_unit_ref: {change_unit_ref|none}
  state_version: {source_state_version}
  source_refs: {source_refs}
  freshness: {freshness_state}
  active_scope: {scope_summary}
  unresolved_user_judgments: {pending_user_judgment_refs_with_kind_labels|none}
  blockers: {active_blockers|none}
  next_safe_action: {next_safe_action}
  evidence_gaps: {evidence_gaps|none}
  close_blockers: {close_blockers|none}
  residual_risk_summary: {residual_risk_summary_if_active|none}
  guarantee_level: {guarantee_level_or_unavailable}
````

메모:

- 에이전트 맥락 패킷은 한 화면 안팎으로 유지합니다. 현재 다음 행동에 필요한 상태만 담습니다.
- 전체 스키마, 전체 Reference 문서, 전체 event log, 등록된 아티팩트 파일 본문, 전체 report body, 전체 template, 관련 없는 template, full design-quality catalog, future catalog 자료를 기본으로 넣지 않습니다.
- 다음 행동에 더 자세한 owner section이 필요하면 그 section을 패킷에 넣지 말고 필요할 때 따로 불러옵니다.
- `guarantee_level` 필드는 필수 맥락입니다. Core/MCP를 사용할 수 없으면 unavailable/capability condition을 넣고, refresh 전까지 하네스에 의존하는 state, write, 증거, 최종 수락, 잔여 위험, close claim을 unavailable로 다룹니다.

## Later 템플릿 경계

Later 후보 템플릿 본문은 active documentation이 아니며 이 참조 문서에 저장하지 않습니다. Later 템플릿 후보 이름은 본문 없이 [Later template 후보](../later/index.md#later-template-candidates)에만 둘 수 있습니다.

Later 후보 목록은 현재 MVP 요구사항, active `ProjectionKind`, schema contract, runtime behavior, 템플릿 본문, generated Projection, 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, implementation task, acceptance evidence를 만들지 않습니다.

Later 템플릿을 승격하려면 향후 담당 문서가 좁은 scope, source record, 대체 동작, 대체 불가능 규칙, freshness behavior, 증명 기대치, exact owner placement를 정의해야 합니다. 그 전까지 활성 현재 MVP output은 이 문서의 다섯 템플릿으로 제한됩니다.
