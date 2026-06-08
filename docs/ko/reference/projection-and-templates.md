# Projection과 템플릿 참조

## 담당하는 것 / 담당하지 않는 것

이 문서는 향후 하네스 동작을 위한 Projection과 활성 템플릿 표시 계약을 담당합니다. 문서 원천 자료일 뿐입니다. 런타임 Projection, 런타임 상태, 생성 산출물, 증거 기록, QA 기록, 최종 수락 기록, 잔여 위험 기록, 닫기 기록, 구현 준비가 끝난 서버 계획이 아닙니다.

이 문서가 담당하는 것:

- Projection 권한 경계
- 파생 표시로서의 Projection
- 사람이 편집 가능한 영역 규칙
- 관리 블록 규칙
- `ArtifactRef` 렌더링 규칙
- 작은 보기의 출처/최신성 표시 규칙
- 현재 MVP 활성 템플릿 세트
- 현재 MVP 활성 템플릿 다섯 개의 전체 렌더링 본문

이 문서가 담당하지 않는 것:

- Core 상태, 생명주기, gate, `prepare_write`, `record_run`, `close_task`, 사용자 판단 상태 변경. [Core Model 참조](core-model.md)를 봅니다.
- 공개 MCP 요청/응답 스키마, `ProjectionKind`, `ArtifactRef`, 오류 형태. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- SQLite DDL, 저장소 배치, 아티팩트 저장소, Projection 작업 저장소. [Storage](storage.md)를 봅니다.
- 설계 품질의 닫기 영향 경계. [설계 품질](design-quality.md)을 봅니다.
- 활성 참조 범위로서의 운영자 명령 동작. 향후 운영 후보는 [이후 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다.
- 적합성 fixture 검증 주장 동작. [적합성 참조](conformance.md)를 봅니다.
- 커넥터 맥락 동작. [Agent 통합 참조](agent-integration.md)를 봅니다.
- 이후 후보 템플릿 본문. 활성 문서가 아닙니다.

## 권한 경계

Core가 소유한 상태와 등록된 아티팩트 참조가 기준입니다. Projection, 상태 카드, Markdown 보고서, 렌더링된 템플릿, 대화 메시지, 커넥터 출력, 에이전트 맥락 패킷은 표시 또는 지원 맥락일 뿐입니다.

템플릿은 Core 상태를 덮어쓸 수 없습니다. 렌더링된 보기는 쓰기를 승인하거나, Write Authorization을 만들거나, gate를 충족하거나, 증거를 만들거나, 검증을 수행하거나, QA를 기록하거나, 민감 동작 승인을 부여하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, Task를 닫거나, 담당 기록을 변경할 수 없습니다. 그런 효과는 담당 Core/API 경로에서 와야 합니다.

표시 라벨은 기준 스키마 값이 아닙니다. 사용자에게 보이는 판단 유형 같은 지역화 라벨은 `judgment_kind`와 locale 같은 기준 필드에서 렌더링됩니다. 호환성 또는 응답 전용 출력에 `display_label`이 나타나도 표시 문구일 뿐입니다. enum 값, 저장소 값, API 필드 담당자, 스키마 범주처럼 다루면 안 됩니다.

사용자가 Projection을 편집해도 그 내용은 입력일 뿐입니다. Projection reconcile은 현재 MVP 활성 동작이 아닙니다. 향후 담당 문서가 승격하기 전까지 사람이 고친 내용은 사용자가 또는 에이전트가 `harness.update_scope`나 `user_judgment` 같은 활성 담당 행동으로 의도를 전달할 때만 상태에 영향을 줄 수 있습니다. 관리 텍스트, 전면 메타데이터(front matter), 표시된 상태, 아티팩트 참조, 닫기 상태, 최종 수락 상태, 잔여 위험 상태, 템플릿 텍스트를 직접 고쳐도 담당 경로로 기록된 상태가 되지 않습니다.

## Projection은 파생 표시

Projection은 현재 Core 소유 기록과 등록된 `ArtifactRef` 메타데이터에서 만든 파생 표시입니다. 사람이 작업, 출처 참조, 차단 사유, 증거 공백, 닫기 차단 사유, 최신성, 다음 안전한 행동을 읽게 돕습니다. 두 번째 상태 저장소가 아닙니다.

현재 MVP의 Projection 범위는 작습니다.

- 사용자용 작은 보기 네 가지: `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`
- 에이전트용 지원 패킷 하나: `agent-context-packet`

이 보기들은 접점에 따라 짧은 텍스트, 카드, Markdown 조각, 구조화된 페이로드로 렌더링될 수 있습니다. 영속 Markdown Projection 작업이나 전체 보고서 렌더러가 필요하지 않습니다. 첫 내부 스모크 목표는 렌더링된 카드 대신 구조화된 상태/차단 사유 출력을 반환해도 됩니다.

최신성은 원천 기록의 시각 위에 표시되는 정보입니다. `stale`, `failed`, `unknown`, 또는 너무 넓은 읽기용 보기는 상태 변경 작업이나 닫기의 근거가 될 수 없습니다. 현재 상태가 중요하면 현재 Core 상태 또는 현재 Core 상태에서 파생된 패킷을 가져와야 합니다.

원천 기록이나 참조가 없으면 `none`, `unknown`, `not_required`, `unavailable`, 또는 차단 메모를 렌더링합니다. 템플릿을 채우려고 자리 표시자 상태를 만들지 않습니다.

Projection은 개인정보 보호 경계이기도 합니다. 생략, 가림 처리, 차단된 아티팩트, 가용성 메모를 보여주되 생략되거나 차단된 원본 값을 재구성하지 않습니다.

## 사람이 편집 가능한 영역

명시적으로 표시된 사람이 편집 가능한 영역만 입력으로 편집할 수 있습니다. 일반적인 형태는 다음과 같습니다.

```md
User Notes and Proposals:
-
```

사람이 편집 가능한 텍스트에는 메모, 질문, 수정, 제안을 담을 수 있습니다. Task 요약, 범위, 수용 기준, 설계 메모, 증거 메모, 다른 담당 기록에 대한 변경을 제안할 수 있습니다. 하지만 제안 자체는 대상 기록이 아닙니다.

향후 상태 변경 경로가 승격된다면 그 경로는 명시적이어야 합니다.

```text
사람 편집 -> reconcile 후보 -> 명시적 reconcile 결과 -> Core 상태 변경 조치, 거절, 보류, 메모
```

이 reconcile 경로는 현재 MVP 범위가 아닙니다. 향후 담당 문서가 승격하고 담당 경로가 Core 결과를 기록하기 전까지 사람이 편집한 텍스트는 Task 상태, 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, 그 밖의 담당 기록이 아닙니다.

사람은 아래 항목을 직접 상태로 편집할 수 없습니다.

- 관리 블록 내용
- `source_state_version` 같은 전면 메타데이터(front matter) 필드
- gate 값, 생명주기 단계, 결과, 닫기 이유, 닫기 상태, 보증 수준
- 사용자 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 닫기 표시 문구, later/reserved QA 면제 판단과 검증 위험 수락 표시 문구
- 아티팩트 식별 정보, `sha256`, `size_bytes`, `content_type`, `redaction_state`, 아티팩트 가용성
- 상태 카드, 에이전트 맥락 패킷, 생성된 보고서, 템플릿 본문

## 관리 블록 규칙

관리 블록은 projector가 소유하는 Markdown 영역입니다.

```md
<!-- HARNESS:BEGIN managed -->
...
<!-- HARNESS:END managed -->
```

규칙:

- 관리 블록 내용은 커밋된 Core 소유 기록과 등록된 아티팩트 참조에서 생성됩니다.
- projector는 관리 블록을 다시 생성할 수 있습니다.
- 관리 블록은 표시이지 권한이 아닙니다.
- 관리 블록 안을 직접 편집하면 drift입니다. 담당 경로로 기록된 상태가 아닙니다.
- 영속 Projection 작업, Projection 작업 저장소, 관리 블록 drift 복구는 이후 후보이며 현재 MVP 의무가 아닙니다.
- 현재 MVP의 작은 보기는 영속 Projection 작업 없이 읽는 시점의 출처/최신성 문구를 담습니다.
- 관리 해시는 마커 줄을 제외한 projector 소유 관리 블록 본문에서 계산합니다. 줄 끝은 정규화하고 projector가 의미 있다고 정한 공백 규칙을 따릅니다.
- 관리 해시는 drift를 감지할 뿐입니다. Markdown을 상태로 만들지 않습니다.
- 향후 승격된 Projection 작업 경로에서 렌더링 전에 관리 블록 해시가 마지막 Projection 해시와 다르면 projector는 drift를 보고하거나 담당 경로로 가는 복구 후보를 만들 수 있습니다. 이것은 이후 drift 복구 동작이며 현재 MVP 복구 의무가 아닙니다. 편집된 블록을 조용히 담당 경로로 기록된 상태처럼 받아들이면 안 됩니다.
- 다시 생성할 때 관련 없는 사람이 편집 가능한 영역은 보존해야 합니다.
- 렌더링 실패나 최신이 아닌 원천 데이터는 상황에 맞게 `failed`, `stale`, `unknown`, `unavailable`을 표시해야 합니다. 커밋된 Core 상태를 롤백하거나, event를 바꾸거나, gate 값을 바꾸면 안 됩니다.

렌더링된 보기의 위쪽 또는 관리 요약 근처에는 짧은 경계 안내가 있어야 합니다. 표시 전용이고, Core 상태와 참조에서 파생되었으며, Write Authorization이나 닫기 결과가 아니라는 점을 보여줍니다.

## `ArtifactRef` 렌더링

큰 로그, diff, trace, 스크린샷, 녹화, 번들, 내보내기 구성 요소, 민감한 아티팩트 본문은 기본적으로 본문에 넣지 않고 `ArtifactRef`로 참조합니다.

독자나 다음 행동에 도움이 될 때는 다음을 렌더링합니다.

- 아티팩트 참조 ID
- 담당 관계 또는 영향을 받는 작업 참조
- 아티팩트 종류 또는 출처 요약
- `sha256`, `size_bytes`, `content_type` 같은 무결성 메타데이터
- `redaction_state`
- 가용성 상태
- 생략, 가림 처리, 차단 메모
- 참조가 중요한 짧은 이유

`secret_omitted`, `blocked`, unavailable, redacted 상태의 아티팩트 본문을 Markdown에 펼치지 않습니다. 메타데이터나 주변 문장으로 생략된 원본 값을 재구성하지 않습니다.

표시된 `ArtifactRef`는 등록된 아티팩트 기록을 가리키는 포인터입니다. 그 자체가 증거 충분성, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태는 아닙니다. 주장에 필요한 담당 관계, 무결성 메타데이터, `redaction_state`, 가용성이 빠졌으면 공백을 보여줍니다.

## 현재 MVP 활성 템플릿 세트

현재 MVP 활성 템플릿 세트는 정확히 아래 다섯 개입니다.

| 독자 | 템플릿 | 본문 |
|---|---|---|
| 사용자용 | `status-card` | [상태 카드 본문](#상태-카드-본문) |
| 사용자용 | `judgment-request` | [판단 요청 본문](#판단-요청-본문) |
| 사용자용 | `run-evidence-summary` | [실행/증거 요약 본문](#실행증거-요약-본문) |
| 사용자용 | `close-result` | [닫기 결과 본문](#닫기-결과-본문) |
| 에이전트용 | `agent-context-packet` | [에이전트 맥락 패킷 본문](#에이전트-맥락-패킷-본문) |

사용자용 출력 네 가지는 평범한 말을 씁니다. 사용자가 판단하거나, 차단 사유를 이해하거나, 증거를 살피거나, 닫기를 이해하는 데 도움이 될 때만 출처 참조와 최신성을 보여줍니다. 스키마, DDL, 이벤트 로그, 전체 아티팩트, 전체 보고서 본문, 전체 증거 목록, 향후 목록을 쏟아내지 않습니다.

에이전트용 패킷은 별도 독자를 위한 것입니다. 현재 다음 행동에 필요한 참조, 차단 사유, 증거 공백, 닫기 차단 사유, 보장 표시 수준, 하나의 다음 안전한 행동만 담습니다. 사용자용 문장이 아니며 권한도 아닙니다.

## 상태 카드 본문

현재 MVP에서 사용자가 현재 상태를 짧게 읽어야 할 때 `status-card`를 사용합니다. 상태 카드는 지금 무엇을 하는지, 무엇이 범위 안인지, 사용자가 무엇을 판단해야 하는지, 어떤 증거가 있거나 빠졌는지, 닫기를 무엇이 막는지, 다음 안전한 행동이 무엇인지를 보여줍니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 첫 내부 스모크 목표는 이 카드 대신 구조화된 상태/차단 사유 출력을 반환해도 됩니다.

경계: 이 템플릿은 렌더링된 표시일 뿐입니다. Core 상태, 증거, 민감 동작 승인, 최종 수락, 잔여 위험 수락, Write Authorization, 닫기 준비 상태 기록이 아닙니다. 최신이 아닌 대화가 아니라 현재 Core 소유 상태와 참조에서 렌더링해야 합니다.

기준 기록:

- 현재 Task 요약, 작업 모양, 생명주기 단계, 필요할 때 막히는 질문 하나, 다음 안전한 행동
- 사용자가 이해하는 데 필요한 현재 범위, 허용 경로 또는 영향 영역, 범위 밖 항목, 수락 기준, Autonomy Boundary, 활성 Change Unit 요약
- 사용자에게 읽히는 라벨로 렌더링한 대기 중인 판단
- 진행 또는 닫기가 보류된 평이한 이유와 활성 차단 사유
- 현재 증거 요약, 뒷받침 참조, 가림 처리 또는 가용성 메모, 증거 공백
- 관련 있을 때 닫기 차단 사유, 최종 수락 필요 여부, 잔여 위험 표시, 잔여 위험 수락 상태
- 보이는 다음 행동을 바꿀 때만 설계 품질 담당 경로로 전달된 조치
- 보장 표시 수준 또는 unavailable capability 상태
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
표시 전용: Core 상태와 참조에서 파생된 보기이며 Core 상태나 쓰기 승인 기록이 아닙니다.

작업: {work_shape}. {current_task_summary}
구체화 상태: {lifecycle_phase}; {shaping_state_reason|none}
범위: {scope_summary}
허용 경로/영역: {allowed_paths_or_affected_areas|unknown}
범위 밖: {non_goals|none}
수락 기준: {acceptance_criteria|unknown}
Autonomy Boundary: {autonomy_boundary|default}
차단 사유: {active_blocked_reason|none}
막히는 질문: {blocking_question|none}
사용자가 결정할 것: {pending_user_judgments_with_localized_labels|none}
증거: {evidence_status}. {known_evidence_summary|none}
증거 공백: {evidence_gaps|none}
확인: {check_summary|none}
닫기: {close_readiness_summary}; 차단 사유={close_blockers|none}
설계 품질 조치: {design_quality_routed_action|none}
잔여 위험: {residual_risk_visibility|none}
다음 안전한 행동: {next_safe_action}
보장 표시: {guarantee_level_or_unavailable}; {guarantee_note}
출처/최신성: {source_freshness_summary}
````

메모:

- 하네스 내부를 모르는 사용자도 읽을 수 있게 유지합니다.
- 기준 기록이 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, 또는 명시적인 차단 사유로 렌더링합니다.
- 보장 표시 줄은 항상 렌더링합니다. 현재 MVP 기본 동작에서는 실제 한계가 협력형 보류이면 그렇게 적고, 탐지형 보고는 관찰 가능한 지원 사실과 통과한 역량 확인이 그 한계를 뒷받침할 때만 적습니다. Core/MCP를 사용할 수 없으면 최신이 아니거나 추측한 보장 표시 값 대신 `unavailable` 조건을 렌더링합니다.
- 설계 품질 내용은 한 줄에 맞춥니다. 현재 담당 경로로 전달된 조치와, 차단일 때는 하나의 다음 행동만 보여줍니다.
- 에이전트 전용 참조와 행동 경계 세부사항은 [에이전트 맥락 패킷 본문](#에이전트-맥락-패킷-본문)에 둡니다. 사용자가 판단하거나 차단 사유를 이해하거나 출처 최신성을 살피는 데 도움이 될 때만 상태 카드에 참조를 넣습니다.

## 판단 요청 본문

진행, 범위, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소 판단에 영향을 주는 선택을 사용자가 소유할 때 `judgment-request`를 사용합니다. 이것은 일반 사용자 소유 판단을 위한 현재 MVP 질문 형태입니다. Later/reserved QA 면제 판단과 검증 위험 수락 질문은 향후 담당 경로가 승격되어야 활성 값이 됩니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 전체 형식 판단 표시는 이후 후보 범위이며 [이후 템플릿 후보](../later/index.md#later-template-candidates)에 후보로만 남습니다.

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

- 작은 판단은 한 화면에 들어가야 하며 현재 MVP에서는 `presentation=short`를 사용합니다. `presentation=full`과 `Decision Packet`은 담당 사용자 판단/템플릿 경로가 승격하기 전까지 이후 후보 자료로 남습니다.
- 민감 동작 승인, 제품 판단, 기술 판단, 범위 판단, 최종 수락, 잔여 위험 수락, 취소 판단, later/reserved QA 면제 판단과 검증 위험 수락 경로를 하나의 넓은 승인 질문으로 합치지 않습니다.
- "yes, do it", "진행해", "좋아" 같은 채팅 문구는 범위, `judgment_kind`, 영향받는 대상, 기록된 사용자 의도가 대기 중인 판단과 맞을 때만 해당 gate를 만족합니다.
- 표시되는 `유형` 라벨은 `judgment_kind`와 사용자 locale에서 렌더링합니다. 이 라벨은 표시 문구일 뿐이며, 기준 판단 범주는 `judgment_kind`입니다.

<a id="실행증거-요약-본문"></a>

## 실행/증거 요약 본문

조언, 실행, 확인, 변경 뒤 무엇이 일어났고 현재 주장에 어떤 증거가 생겼는지 최소한으로 보여줘야 할 때 `run-evidence-summary`를 사용합니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 상세 실행 보고서와 상세 증거 목록은 이후 후보 범위입니다.

경계: 이 템플릿은 Run과 증거 참조를 표시할 뿐입니다. 증거 자체, 상세 증거 목록, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태 기록이 아닙니다.

기준 기록:

- Run 참조와 명령/확인 요약
- 변경 경로 또는 파일 변경 없음 결과
- 관련 있을 때 소비된 Write Authorization 참조, 쓰기 없음 근거, 또는 무효한 승인 맥락 시도
- 증거 참조, 아티팩트 참조, 가림 처리, 가용성 메모
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
표시 전용: 참조와 요약일 뿐이며 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기가 아닙니다.

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

- 증거 충분성은 양이 아니라 포괄성입니다. 현재 뒷받침하는 참조가 없거나 중요한 아티팩트 참조에 담당 관계, 무결성 메타데이터, `redaction_state`, 가용성이 없으면 공백과 현재 증거 상태를 보여줘야 합니다. 긴 아티팩트 목록이나 보고서 문장을 충분한 뒷받침처럼 취급하면 안 됩니다.
- 제품 쓰기 Run에서 제품 쓰기 호환성 기록으로 표시할 수 있는 것은 호환되게 소비된 Write Authorization뿐입니다. 무효한 승인 시도 참조는 violation/audit 또는 validator-finding 맥락으로만 보여줄 수 있으며, 소비된 Write Authorization이나 완료 증거처럼 렌더링하면 안 됩니다.
- 이 요약은 전체 증거 보고서보다 작게 유지합니다. 사용자의 다음 판단에 필요한 증거 참조와 보이는 공백만 보여주고, 전체 아티팩트 목록이나 원본 아티팩트 본문을 펼치지 않습니다.

## 닫기 결과 본문

사용자가 닫기 준비 상태, 닫기 차단 사유, 또는 닫기 결과를 간결하게 봐야 할 때 `close-result`를 사용합니다. 최종 수락, 잔여 위험, 증거, 아티팩트 가용성, 자체 확인 근거, 차단 사유를 서로 분리해 보여줍니다.

구현 계층: 현재 MVP 사용자 작업 루프 보기입니다. 상세 연속성, release-handoff, export 보고서는 이후 후보 범위입니다.

경계: 이 템플릿은 닫기 상태를 표시합니다. Task를 닫거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 검증 또는 QA를 기록하거나, 증거를 만들거나, gate 값을 바꾸지 않습니다. 닫기 결과는 Core 닫기 경로만 만들 수 있습니다.

기준 기록:

- 현재 Task 상태와 닫기 시도 또는 닫기 준비 상태 결과
- 범위와 변경 범위 요약
- 증거 참조와 증거 공백
- 활성 증거 요약에 포함된 자체 확인 요약
- 닫기 관련 증거 참조에 대한 아티팩트 가용성
- 필요한 경우 최종 수락 `user_judgment` 참조
- 관련 있을 때 잔여 위험 표시와 잔여 위험 수락 참조
- 닫기에 영향을 줄 때 설계 품질 담당 경로로 전달된 조치. 이후 후보가 활성화되지 않았다면 현재 MVP 차단 집합으로 제한합니다.
- 닫기 가능 여부, 닫기 차단 사유, 가장 작은 해결 방법
- 원천 상태 버전, 최신성, capability 상태

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
표시 전용: Core 닫기 상태와 담당 참조가 기준입니다.

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

- 증거 요약, 아티팩트 가용성, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 차단 사유, 설계 품질 담당 경로로 전달된 조치, 읽기용 보기 최신성을 하나의 "완료" 줄로 뭉개지 않습니다.
- 현재 MVP `close-result` 출력은 현재 MVP 닫기 의미만 보여줍니다. 이후 후보인 보장 정보와 상세 QA 줄은 이후 후보 범위에 남습니다.
- 닫기가 막혔으면 주된 차단 사유와 다음 행동 하나를 말하고, 다음 경로에 영향을 주는 보조 차단 사유만 보이게 둡니다.
- 읽기용 닫기 보기가 `stale` 또는 `failed`이면 이 템플릿의 문장으로 닫지 말고 현재 Core 닫기 결과를 가져와야 합니다.

## 에이전트 맥락 패킷 본문

다음 안전한 행동에 필요한 현재 맥락을 에이전트가 작고 정확하게 받아야 할 때 `agent-context-packet`을 사용합니다. 이 보기는 사용자용 문장이나 전체 보고서가 아니라 최신성, Core 기반 참조, 활성 차단 사유, 해결되지 않은 사용자 판단, 증거 공백, 닫기 차단 사유, 보장 표시 수준, 하나의 다음 행동에 최적화됩니다.

구현 계층: 현재 MVP 지원 보기입니다. 구조화된 페이로드나 프롬프트 크기의 텍스트로 반환할 수 있습니다. 영속 Markdown Projection이 필수는 아닙니다.

경계: 에이전트 맥락 패킷은 행동을 돕는 맥락일 뿐입니다. 쓰기를 승인하거나, gate를 충족하거나, 증거를 만들거나, 민감 동작 승인을 부여하거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, Task를 닫을 수 없습니다.

기준 기록:

- Task와 활성 Change Unit 참조
- 현재 상태 버전과 원천 참조
- 활성 범위, 허용 경로 또는 영향 영역, 범위 밖 항목, 수락 기준, Autonomy Boundary
- 해결되지 않은 사용자 판단
- 활성 차단 사유
- 증거 공백
- 닫기 차단 사유
- 활성일 때 잔여 위험 요약
- 보장 표시 수준 또는 `unavailable`/역량 상태
- 정확히 하나의 다음 안전한 행동

렌더링 섹션:

- Task와 Change Unit 참조
- 상태 버전과 원천 참조
- 활성 범위
- 구체화 요약
- 해결되지 않은 사용자 판단
- 차단 사유
- 다음 안전한 행동
- 증거 공백
- 닫기 차단 사유
- 잔여 위험 요약
- 보장 표시 수준

템플릿:

````text
agent_context_packet:
  display_only: true
  authority: none; 실제 권한은 현재 Core 상태를 사용
  task_ref: {task_ref}
  change_unit_ref: {change_unit_ref|none}
  state_version: {source_state_version}
  source_refs: {source_refs}
  freshness: {freshness_state}
  active_scope: {scope_summary}
  allowed_paths_or_areas: {allowed_paths_or_affected_areas|unknown}
  non_goals: {non_goals|none}
  acceptance_criteria: {acceptance_criteria|unknown}
  autonomy_boundary: {autonomy_boundary|default}
  blocking_question: {blocking_question|none}
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
- 전체 스키마, 전체 참조 문서, 전체 이벤트 로그, 등록된 아티팩트 파일 본문, 전체 보고서 본문, 전체 템플릿, 관련 없는 템플릿, 전체 설계 품질 목록, 향후 목록 자료를 기본으로 넣지 않습니다.
- 다음 행동에 더 자세한 담당 문서 섹션이 필요하면 그 섹션을 패킷에 넣지 말고 필요할 때 따로 불러옵니다.
- `guarantee_level` 필드는 필수 보장 표시 맥락입니다. Core/MCP를 사용할 수 없으면 `unavailable`/역량 조건을 넣고, 새로 확인하기 전까지 하네스에 의존하는 상태, 쓰기, 증거, 최종 수락, 잔여 위험, 닫기 주장을 `unavailable`로 다룹니다.

## 이후 후보 템플릿 경계

이후 후보 템플릿 본문은 활성 문서가 아니며 이 참조 문서에 저장하지 않습니다. 이후 후보 템플릿 이름은 본문 없이 [이후 템플릿 후보](../later/index.md#later-template-candidates)에만 둘 수 있습니다.

이후 후보 목록은 현재 MVP 요구사항, 활성 `ProjectionKind`, 스키마 계약, 런타임 동작, 템플릿 본문, 생성된 Projection, 증거, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, 구현 작업, 수락 증거를 만들지 않습니다.

이후 템플릿을 승격하려면 향후 담당 문서가 좁은 범위, 원천 기록, 대체 동작, 대체 불가능 규칙, 최신성 동작, 향후 승격에 필요한 증명 경로 기대치, 정확한 담당 문서 위치를 정의해야 합니다. 그 전까지 현재 MVP 활성 출력은 이 문서의 다섯 템플릿으로 제한됩니다.
