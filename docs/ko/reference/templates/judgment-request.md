# 판단 요청 템플릿

## 사용 시점

진행, 범위, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단에 영향을 주는 선택을 사용자가 소유할 때 `judgment-request`를 사용합니다. 이것은 일반 사용자 소유 판단을 위한 MVP-1 prompt shape입니다.

구현 계층: MVP-1 사용자 작업 루프 보기입니다. 전체 Decision Packet presentation은 later/full-profile 범위이며 [../../later/index.md#later-template-candidates](../../later/index.md#later-template-candidates)에 있습니다.

경계: 이 템플릿은 대기 중이거나 기록된 `user_judgment`를 표시합니다. 이 표시만으로 판단 기록을 만들거나, Write Authorization을 부여하거나, QA 또는 검증을 수행하거나, QA 증거를 만들거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 검증 위험을 수락하거나, Task를 닫지 않습니다.

## 기준 기록

- 대기 중이거나 기록된 `user_judgment`
- `judgment_kind`, `presentation`, locale에서 파생한 표시 판단 라벨
- 정확한 질문, 이유, 추천, 불확실성, 사용자가 결정하지 않을 때의 결과
- 영향을 받는 Task, Change Unit, 쓰기 범위, 닫기 범위, 민감 동작 범위, 기준 또는 다른 대상
- 선택지 또는 선택된 결과
- 결과 영향, 에이전트가 사용자 대신 판단하지 않는 것, 에이전트가 사용자 대신 판단할 수 없는 이유
- 영향을 받는 작업을 식별하는 데 필요한 최소 출처 참조
- 판단에 영향을 줄 때만 증거, 위험, 민감 동작 승인, QA, 검증, 닫기 참조

## 렌더링 섹션

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

## 전체 템플릿

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

## 메모

작은 판단은 한 화면에 들어가야 합니다. 더 자세한 장단점, 추천, 영향을 받는 gate, 증거/위험 참조, 미룰 때의 분석은 active profile이나 판단 복잡도가 요구할 때만 `presentation=full`로 보여줍니다.

민감 동작 승인, 제품 판단, 기술 판단, 범위 판단, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단을 하나의 넓은 승인 질문으로 합치지 않습니다. "yes, do it", "진행해", "좋아" 같은 채팅 문구는 scope, `judgment_kind`, affected object, recorded user intent가 pending judgment와 맞을 때만 해당 gate를 만족합니다.

표시되는 `유형` 라벨은 `judgment_kind`와 사용자 locale에서 렌더링합니다. 이 라벨은 표시 문구일 뿐이며, 기준 판단 범주는 `judgment_kind`입니다.
