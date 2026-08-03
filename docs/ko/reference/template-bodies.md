# 템플릿 본문

이 문서는 현재 렌더링되는 템플릿 본문의 표시용 문구와 표시용 패킷/본문 형태를 담당합니다.

해당 본문은 아래와 같습니다.

- 상태 카드
- 공개 오류 메시지
- 판단 요청
- 실행/증거 요약
- 닫기 결과
- 관리 `AGENTS.md` 안내 블록
- 에이전트 맥락 패킷

이 문서는 렌더링 본문 지침, 사용자 표시 라벨, 표시 문구만 담당합니다.

권한, 저장소 기록, API 오류 의미, 닫기 차단 사유 의미는 연결된 담당 문서에 남습니다.

## 담당 경계

이 문서는 표시 표현만 담당합니다.

- 현재 상태 표시와 지원 표시를 위한 렌더링 템플릿 본문 지침과 표시용 패킷/본문 형태
- 정확한 생성 `AGENTS.md` 관리 안내 본문
- 그 본문의 사용자 표시 라벨, 표시 문구, 지역화 라벨, 해결 안내
- 표시 문구로서의 공개 오류 표시 라벨
- 본문 자리 표시자에서 스키마 및 권한 담당 문서로 가는 링크

이웃 담당 문서에는 아래 권한이 남습니다.

- 상태 보기 최신성과 읽기 전용 파생 표시 규칙: [상태 보기와 템플릿 표시 경계](projection-and-templates.md)
- Core 상태, 사용자 소유 판단, 증거, 닫기 준비 상태, 수락, 잔여 위험: [Core 모델](core-model.md)
- API 스키마와 값 집합: 스키마 담당 문서와 [API 값 집합](api/schema-value-sets.md)
- 공개 `ErrorCode` 의미: [API 오류 코드](api/error-codes.md)
- 응답 분기: [API 오류 처리 경로](api/error-routing.md)
- 오류 우선순위: [API 오류 우선순위](api/error-precedence.md)
- 차단 사유 처리 경로: [API 차단 사유 처리 경로](api/blocker-routing.md)
- `ToolError.details`: [API 오류 세부사항](api/error-details.md)
- 저장소 기록 구조, 지속성, 아티팩트 생명주기, 저장 효과: [참조 색인](README.md)에서 고르는 저장소 담당 문서
- 지원 경계, 보안 보장, 연결 맥락: [범위 참조](scope.md), [보안](security.md), [Agent Connection](agent-connection.md)

## 권한 경계

템플릿 문구는 표시 문구입니다. 담당 기록을 요약하고 의미 담당 문서를 가리킬 수 있지만, 그 의미를 다시 정의하거나 권한이 될 수는 없습니다.

담당 문서 소유 입력은 표시 문구를 고르거나 채우는 데 사용할 수 있습니다.

- 공개 `ErrorCode`
- `CloseReadinessBlocker`
- `state_version`
- `ArtifactRef`

그 의미, 우선순위, 처리 경로, 저장 효과, 스키마 권한은 각 담당 문서에 남습니다.

템플릿 문구만으로는 아래 일을 할 수 없습니다.

- 쓰기 티켓 생성 또는 담당 기록 변경
- 증거, 영속 아티팩트, 최종 수락, 잔여 위험 수락 생성
- 증거, QA, 검증, 수락, 닫기 준비 상태, 닫기 관문 충족
- 저장소 구조나 저장 효과 정의 또는 렌더링 본문을 저장소 권한으로 만들기
- 공개 `ErrorCode` 식별자나 의미의 정의, 이름 변경, 지역화, 의미 변경
- 응답 분기 동작, 오류 우선순위, 기계 판독용 세부 키의 정의나 변경
- 닫기 차단 사유 의미, 차단 사유 코드, 차단 사유 처리 경로의 정의나 변경
- 거부 응답 오류를 차단 사유나 차단 결과로 바꾸기

## 공개 오류 표시 라벨

이 절은 공개 API 오류를 사용자나 에이전트가 보는 표시 대상에 렌더링할 때 표시 라벨과 해결 안내를 고르는 데 사용합니다.

이 절은 아래 항목을 정의하지 않습니다.

- 어떤 오류가 존재하는지
- 그 의미가 무엇인지
- 어떤 분기가 우선하는지
- 차단 결과를 어떻게 처리하는지

렌더링 오류 문구는 아래 기준을 지킵니다.

- 정확한 진단 식별자를 보여 줄 때는 공개 `ErrorCode`를 그대로 보존합니다.
- 표시 대상에 공간이 있으면 짧은 라벨과 해결 안내 하나를 함께 보여 줍니다.
- 라벨을 `CloseReadinessBlocker.code`, `WriteDecisionReason.code`, `PlannedBlocker.code`, `ToolError.details` 키와 구분합니다.
- 코드 의미, 우선순위, 응답 분기, 차단 사유 처리 경로, 기계 판독용 세부사항을 설명해야 하면 API 담당 문서로 연결합니다.

렌더링 오류 문구는 아래처럼 쓰면 안 됩니다.

- 공개 `ErrorCode`를 지역화 라벨로 대체하기.
- 공개 `ErrorCode` 의미를 정의하거나 바꾸기.
- 라벨을 의미 담당 문서나 기계 판독용 코드로 취급하기.
- 닫기 차단 사유를 숨기거나 거부 응답을 차단 결과로 바꾸기.

담당 문서 링크:
- [API 오류 코드](api/error-codes.md): 공개 코드 의미.
- [API 오류 우선순위](api/error-precedence.md): 오류 우선순위.
- [API 오류 처리 경로](api/error-routing.md): API 응답 분기 경로.
- [API 차단 사유 처리 경로](api/blocker-routing.md): 차단 사유 처리 경로.
- [API 오류 세부사항](api/error-details.md): 기계 판독용 세부 규칙.

<a id="label-validation-failed"></a>
### `VALIDATION_FAILED`

라벨 선택 입력:
- `VALIDATION_FAILED`.

표시 라벨:
- 잘못된 요청

해결 안내:
- 다시 시도하기 전에 요청 본문, enum 값, 적용 규칙, 프로필 값, 필드 집합을 고칩니다.

<a id="label-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

라벨 선택 입력:
- `STATE_VERSION_CONFLICT`.

표시 라벨:
- 상태 버전 충돌

해결 안내:
- 현재 상태를 새로 고치고 현재 `project_state.state_version`으로 다시 시도하거나 원래 멱등 요청을 재실행합니다.

<a id="label-invocation-context-mismatch"></a>
### `INVOCATION_CONTEXT_MISMATCH`

라벨 선택 입력:
- `INVOCATION_CONTEXT_MISMATCH`.

표시 라벨:
- 호출 맥락 불일치

해결 안내:
- 등록된 Agent Connection, User Channel, 프로젝트 처리 경로 또는 메서드와
  호환되는 호출 맥락을 사용합니다.
- 필요한 경우 연결 바인딩이나 호출 맥락 설정을 고칩니다.

<a id="label-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

라벨 선택 입력:
- `CAPABILITY_INSUFFICIENT`.

표시 라벨:
- 연결 역량 부족

해결 안내:
- 역량이 있는 Agent Connection을 사용합니다.
- 동작을 줄이거나 빠진 역량이 필요 없는 경로를 선택합니다.

<a id="label-no-active-task"></a>
### `NO_ACTIVE_TASK`

라벨 선택 입력:
- `NO_ACTIVE_TASK`.

표시 라벨:
- 현재 적용 `Task` 없음

해결 안내:
- 작업 범위 동작 전에 `Task`를 선택하거나 생성합니다.

<a id="label-scope-boundary-baseline"></a>
### 범위, 경계, 기준 상태

라벨 선택 입력:
- `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE`.

표시 라벨:
- 범위, 경계, 기준 상태 문제

해결 안내:
- 범위를 확인하거나 좁힙니다.
- 유효한 범위 또는 기준 상태 변경은 알맞은 범위 또는 기준 상태 담당 문서가 정의한 동작으로 갱신합니다.
- 필요한 사용자 판단을 요청합니다.

<a id="label-write-ticket"></a>
### 쓰기 티켓

라벨 선택 입력:
- `WRITE_TICKET_REQUIRED` 또는 `WRITE_TICKET_INVALID`.

표시 라벨:
- 쓰기 티켓 필요 또는 사용 불가

해결 안내:
- 정확한 동작, 현재 적용 범위, 현재 상태로 `volicord.prepare_write`를 호출하거나 다시 시도합니다.

<a id="label-judgment"></a>
### 판단

라벨 선택 입력:
- `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`.

표시 라벨:
- 판단 필요

해결 안내:
- 집중된 `UserActionRequest`를 그 소유 workflow를 통해 요청하거나 해결합니다.

<a id="label-sensitive-approval"></a>
### 민감 동작 승인

라벨 선택 입력:
- `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`.

표시 라벨:
- 민감 동작 승인 필요 또는 사용 불가

해결 안내:
- `judgment_kind=sensitive_approval`을 요청, 해결, 갱신합니다.

<a id="label-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

라벨 선택 입력:
- `EVIDENCE_INSUFFICIENT`.

표시 라벨:
- 증거 필요

해결 안내:
- 누락된 증거를 기록하거나 재실행하거나 보여 주고, 다음에 필요한 최소 행동을 표시합니다.

<a id="label-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

라벨 선택 입력:
- `ACCEPTANCE_REQUIRED`.

표시 라벨:
- 최종 수락 필요

해결 안내:
- 표시된 결과 근거에 대해 `judgment_kind=final_acceptance`를 요청하거나 해결합니다.

<a id="label-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

라벨 선택 입력:
- `RESIDUAL_RISK_NOT_VISIBLE`.

표시 라벨:
- 잔여 위험이 보이지 않음

해결 안내:
- 최종 수락이나 닫기 전에 닫기 관련 잔여 위험을 보여 줍니다.

<a id="label-projection-stale"></a>
### `PROJECTION_STALE`

라벨 선택 입력:
- `PROJECTION_STALE`.

표시 라벨:
- 오래된 상태 보기

해결 안내:
- 그 보기에 의존하기 전에 새로 고칩니다.

<a id="label-artifact-missing"></a>
### `ARTIFACT_MISSING`

라벨 선택 입력:
- `ARTIFACT_MISSING`.

표시 라벨:
- 아티팩트 문제

해결 안내:
- 없거나 사용할 수 없는 아티팩트를 복구, 재생성, 교체, 다시 연결합니다.

<a id="label-validator-failed"></a>
### `VALIDATOR_FAILED`

라벨 선택 입력:
- `VALIDATOR_FAILED`.

표시 라벨:
- 검사 실패

해결 안내:
- 가능하면 특정 검증기나 확인 결과를 보여 줍니다.
- 더 분명한 타입 있는 공개 코드가 없을 때만 이 대체 라벨을 사용합니다.

<a id="managed-agents-guidance-body"></a>
## 관리 `AGENTS.md` 안내 본문

Init은 다음 블록을 정확히 생성합니다. 고정된 status/intake/check-close 호출 절차를
강요하지 않고 오래 유지되는 권한·안전 경계를 설명합니다.

```markdown
<!-- BEGIN VOLICORD MANAGED GUIDANCE -->
# Volicord

- Treat Volicord's recorded scope, current shaping checkpoint, and linked UserAction authority as authoritative.
- Do not modify Product Repository files outside an active compatible write authorization.
- Do not infer, resolve, approve, or record user-owned trust judgments on the user's behalf.
- Follow the tagged workflow's `required_action`; do not call workflow tools speculatively or select progression from top-level array order.
- Never replace the current shaping checkpoint to remove a pending or resolved-but-unapplied decision. Preserve its UserAction authority and follow the tagged recovery method.
- User Channel resolution does not apply a shaping decision. Apply each resolved decision only through its `application_owner`, using the exact current resolution refs.
- Do not invent a scope decision or pass a scope-decision ref for product-only or technical-only work; follow that decision's mode-specific application owner.
- Creating or replacing a Change Unit does not advance the Task phase. For work, call `volicord.advance_task` only when the tagged workflow requires explicit advance and never while a UserAction is pending. Do not call `volicord.prepare_write` before the Task enters implementation.
- Advisor work uses only a non-write Change Unit. On `ready_to_finalize_advice`, finalize the current advisor result with `volicord.record_shaping`; do not use `volicord.record_run`, `volicord.advance_task`, or `volicord.prepare_write` for advisor.
- Create current UserAction requests before presenting user-owned choices. A chat reply is not a User Channel resolution; surface the canonical CLI inbox instruction.
- Never hide or paraphrase a rejected mutation as success. Surface the tagged workflow and every fact in `presentation.must_surface`, including the current Task phase and exact recovery method.
- Evaluate close readiness only during close review. Close blockers do not replace tagged workflow progression.
- Call `volicord.status` only when the current Task state is unknown or an authoritative refresh is required.
- Do not claim completion while Volicord reports close blockers. If Volicord is unavailable, disclose that its state was not updated or verified.
<!-- END VOLICORD MANAGED GUIDANCE -->
```

이 블록은 생성된 저장소 안내일 뿐 Core 상태, 쓰기 티켓, 사용자 권한, Agent가 지침을
따랐다는 증거, 제품 파일 편집 허가가 아닙니다. 폐쇄형 semantic fact 집합의 canonical
digest는 프로젝트 통합 revision에 포함됩니다. 따라서 rendering 문장 부호나 hook command가
그대로여도 이 guidance semantic 중 하나가 바뀌면 revision 범위 managed evidence는
current가 아니게 됩니다.

## 상태 카드 본문

### 입력 상태

- `volicord.status`가 반환한 현재 읽기 전용 상태입니다.
- `StateSummary`, 차단 사유, 대기 중인 `AgentSafeUserActionRequestSummary` 항목, 증거
  요약과 출처 상태, 닫기 준비 상태 관찰, 잔여 위험 범위, 프로젝트 연속성 요약, 보장
  표시, 다음 안전한 행동 같은 표시 입력입니다.
- 원천 참조, `state_version`, 관찰 시각, 오래됨 표시, 사용할 수 없음 표시, 역량 제한 표시 같은 최신성 단서가 있으면 함께 사용합니다.
- 아티팩트 가용성은 담당 문서가 허용한 `ArtifactRef` 표시 데이터나 사용할 수 없음/가림 처리 메모로만 표시합니다.

### 반드시 표시할 것

- 현재 위치를 압축해 보여 주는 카드.
- 상태와 현재 적용 범위를 별도 영역으로 표시하는 것.
- 현재 목표, 현재 적용 범위, 범위 밖 항목, 허용된 행동 상태를 해당 필드가 있을 때 표시하는 것.
- 차단 사유와 대기 중인 사용자 행동을 별도 영역으로 표시하는 것.
- 실행/증거 요약, 증거 출처 한계, 공백을 별도 영역으로 표시하는 것.
- 닫기 준비 상태 요약, 다음 안전한 행동, 출처 참조와 최신성을 별도 영역으로 표시하는 것.
- 잔여 위험과 이어 가는 연속성 기록을 별도 영역으로 표시하는 것.
- 카드가 읽기 전용 파생 표시라는 사실.
- 오래됨, 일부만 있음, 사용할 수 없음, 가림 처리, 역량 제한 같은 원천 조건.
- 필수 차단 사유, 해결되지 않은 사용자 행동, 필수 증거 공백.
- 닫기 준비 상태를 현재 관찰로 표시하고, 닫기 동작처럼 보이지 않게 하는 것.
- 사용할 수 없거나 가려진 아티팩트 본문을 포함한 아티팩트 한계.

### 암시하면 안 되는 것

- 카드가 쓰기 티켓을 만들거나, 증거를 기록하거나, 위험을 수락하거나, `Task`를 닫는다는 의미.
- [API 값 집합](api/schema-value-sets.md)이 지원 값으로 정의하지 않았는데 초록색 또는 긍정 라벨이 기준 enum 값이라는 의미.
- 아티팩트가 있다는 사실만으로 증거가 충분하다는 의미.
- 빠진 원천 데이터를 낙관적인 문구로 대신할 수 있다는 의미.
- 기존 대기 요청의 질문, 선택지, 맥락, resolution form, 캡처 경로, 명령, URL,
  credential을 차단 사유, 다음 행동, template text에서 다시 만들 수 있다는 의미. 이
  요청은 요청 ID, `pending`, 다음 actor/User Channel만 표시합니다. 요청을 만들기 전의
  `missing_final_acceptance` 행동은 Agent가 요청을 만드는 데 필요한 차단 질문을 계속
  표시할 수 있지만, 요청을 만든 뒤에는 일반 대기 요약 규칙을 적용합니다.

### 사용자에게 보이는 문구

직접적인 상태 문구를 씁니다.

- `상태 버전: {state_version}; 관찰 시각: {observed_at}.`
- `사용자 행동 필요: {pending_user_action_summary}.`
- `닫기 차단 사유: {close_blocker_summary}.`
- `증거 출처: {provenance_summary}.`
- `이어 가는 연속성: {continuity_summary}.`
- `다음 안전한 행동: {next_action}.`

그 담당 기록이 있고 링크되어 있을 때만 `승인됨`, `수락됨`, `검증됨`, `닫힘` 같은 문구를 씁니다.

그렇지 않으면 위 문구를 피합니다.

### 담당 문서 링크

- [상태 보기와 템플릿 표시 경계](projection-and-templates.md): 읽기 전용 표시와 최신성 경계.
- [Core 모델](core-model.md): Core 권한과 닫기 준비 상태 의미.
- [API 상태 스키마](api/schema-state.md): 상태 형태 표시 입력.
- [API 사용자 행동 스키마](api/schema-user-action.md): 공통 요청, adapter-neutral
  resolution form, 상태, resolution 형태.
- [API 판단 스키마](api/schema-judgment.md): 선택형 판단 전용 세부 형태.
- [API 아티팩트 스키마](api/schema-artifacts.md): `ArtifactRef` 표시 입력.

<a id="judgment-request-body"></a>
## 판단 요청 본문

### 입력 상태

- 검증된 User Channel renderer에 전달된 typed `CliUserActionInboxItem` 하나. 이
  template은 안전한 요약만 담는 Agent 대상 `volicord.request_user_action` 결과에서 만들지
  않습니다.
- 정확한 질문과 제한된 선택지.
- 근거, 불확실성, 영향을 받는 범위, 미룰 때의 결과, 대체 불가 메모.
- 연결된 출처 참조, `state_version`, 최신성 또는 역량 제한 메모가 있으면 함께 사용합니다.

### 반드시 표시할 것

- 결정 하나만 묻는 집중 요청으로, 사용자의 답변을 증거, 수락, 잔여 위험 수락, 쓰기 티켓과 분리해서 표시합니다.
- 사용자가 판단해야 하는 정확한 질문.
- 왜 에이전트 추론이 아니라 사용자 소유 판단인지.
- 현재 사실과 맞고 서로 구분되는 짧은 선택지.
- 답변이 무엇을 정하고 무엇을 정하지 않는지.
- 기다리거나 답하지 않을 때의 결과.

### 암시하면 안 되는 것

- 선택지가 명백해 보여서 에이전트가 대신 고를 수 있다는 의미.
- 포괄적인 "예"가 민감 동작 승인, 최종 수락, 잔여 위험 수락 또는 다른 별도 판단을 대신한다는 의미.
- 답변이 증거를 만들거나, 작업을 검증하거나, 관련 없는 쓰기를 승인한다는 의미.
- 서로 다른 결정을 묶어 질문하고 하나의 답변으로 기록할 수 있다는 의미.
- 이 완전한 질문·선택지 template을 Agent Connection 출력, fallback text, status,
  close, replay, operation-result byte에 표시할 수 있다는 의미.

### 사용자에게 보이는 문구

질문 하나만 묻는 문구를 씁니다.

- `{decision_scope}에 대한 사용자 판단이 필요합니다.`
- `하나를 선택하세요: {option_list}.`
- `이 답변은 {settled_scope}만 정합니다. {non_settled_scope}는 정하지 않습니다.`
- `미루면 다음 안전한 행동은 {deferral_action}입니다.`

`명백히`, `그냥 승인`, `제가 대신 결정할 수 있습니다`처럼 압박하거나 대체하는 문구를 피합니다.

### 담당 문서 링크

- [Core 모델](core-model.md): 사용자 소유 판단과 비대체 규칙.
- [사용자 행동 요청 메서드](api/method-request-user-action.md): 요청 동작.
- [사용자 행동 해결 메서드](api/method-resolve-user-action.md): 불변 User Channel 해결 동작.
- [API 사용자 행동 스키마](api/schema-user-action.md): 공통 요청, 해결, 상태, adapter-neutral resolution form.
- [API 판단 스키마](api/schema-judgment.md): 선택 payload, `SensitiveActionScope`, 수락된 위험 형태.
- [보안](security.md): 민감 동작 승인 경계.

<a id="run--evidence-summary-body"></a>
## 실행/증거 요약 본문

### 입력 상태

- 현재 적용 `Task` 또는 Change Unit의 실행 및 증거 담당 기록.
- 증거 범위 항목과 필수/선택/해당 없음 상태.
- 뒷받침하는 실행 기록 참조, 뒷받침하는 `ArtifactRef` 링크, 차단 사유, 있는 경우 `ValidatorResult`.
- 최신성 단서.
- 아티팩트 담당 문서에서 온 아티팩트 가용성, 가림 처리, 차단된 아티팩트, 사용할 수 없음 메모.

### 반드시 표시할 것

- 증거 위치를 압축해 보여 주는 요약.
- 실행하거나 확인한 것을 별도 영역으로 표시하는 것.
- 결과와 신뢰 한계를 별도 영역으로 표시하는 것.
- 필수 증거 범위와 선택적 뒷받침 증거를 별도 영역으로 표시하는 것.
- 아티팩트와 출처 참조를 별도 영역으로 표시하는 것.
- 공백, 차단 사유, 다음 안전한 행동을 별도 영역으로 표시하는 것.
- 필수 증거와 선택적 뒷받침의 구분.
- 필수 증거가 뒷받침되지 않았거나, 일부만 있거나, 오래됐거나, 막혔거나, 빠진 항목.
- 그런 연결이 있으면 어떤 Run 또는 아티팩트가 어떤 주장을 뒷받침하는지.
- 가림 처리와 본문 읽기 제한을 포함한 아티팩트 가용성 한계.
- 증거 사용에 영향을 주는 최신성 또는 원천 상태 한계.

### 암시하면 안 되는 것

- Run 결과만으로 최종 수락, QA, 검증, 잔여 위험 수락이 된다는 의미.
- 사용할 수 있는 아티팩트가 자동으로 충분한 증거라는 의미.
- 요약이 실행 기록이나 증거 담당 문서가 기록하지 않은 증거를 만든다는 의미.
- 가려졌거나, 생략되었거나, 사용할 수 없거나, 차단된 아티팩트 값을 재구성할 수 있다는 의미.

### 사용자에게 보이는 문구

증거 범위 문구를 씁니다.

- `확인한 것: {run_or_check_summary}.`
- `필수 증거 충족: {covered_items}.`
- `아직 빠진 필수 증거: {gap_items}.`
- `사용 가능한 아티팩트: {artifact_ref}; 본문 상태: {availability_note}.`

관련 담당 기록이 있고 링크되어 있을 때만 `완전히 검증됨`, `QA 통과`, `수락됨` 같은 문구를 씁니다.

그렇지 않으면 위 문구를 피합니다.

### 담당 문서 링크

- [Core 모델](core-model.md): 증거 의미와 비대체 규칙.
- [실행 기록 메서드](api/method-record-run.md): 실행/증거 메서드 동작.
- [API 상태 스키마](api/schema-state.md): 증거 요약과 `ValidatorResult` 형태 표시 데이터.
- [API 아티팩트 스키마](api/schema-artifacts.md)와 [아티팩트 저장소](storage-artifacts.md): 아티팩트 참조, 가용성, 본문 읽기 자격.
- [저장 효과](storage-effects.md): 저장소를 바꾸는 것과 바꾸지 않는 것.

<a id="close-result-body"></a>
## 닫기 결과 본문

### 입력 상태

- `volicord.check_close`가 반환한 `CheckCloseResult` 또는
  `volicord.close_task`가 반환한 `CloseTaskResult`.
- `CloseReadinessBlocker[]`, 증거 요약, 대기 중인
  `AgentSafeUserActionRequestSummary` 항목.
- 최종 수락 상태, 잔여 위험 상태, 아티팩트 가용성.
- 닫기 결과가 반환한 프로젝트 연속성 기록.
- 출처 참조, 최신성 단서, 요청한 메서드 또는 닫기 의도.
- 읽기 전용 닫기 확인과 상태를 바꾸는 닫기 시도를 구분하는 담당 결과.

### 반드시 표시할 것

- 본문이 읽기 전용 확인, 차단된 닫기 시도, 담당 기록으로 남은 닫기 결과 중 무엇을 표시하는지 분명히 밝힙니다.
- 닫기 의도가 있을 때 그 의도와 담당 결과가 읽기 전용인지 상태 변경인지.
- 반환된 모든 닫기 차단 사유와 그 책임 차단 사유 범주 또는 다음 행동.
- 남은 증거, 사용자 행동, 최종 수락, 잔여 위험, 아티팩트 가용성 공백.
- 사용할 수 있으면 원천 상태 버전 또는 그에 준하는 최신성 단서.
- 성공한 닫기 뒤에도 관련성이 남는 연속성 기록.
- 닫기가 막혔을 때의 다음 안전한 행동.

### 암시하면 안 되는 것

- 닫기 확인이 `Task`를 닫았다는 의미.
- `ready` 라벨이 `Task`를 닫거나 차단 사유를 제거한다는 의미.
- 포괄적 승인이 최종 수락이나 잔여 위험 수락을 대신한다는 의미.
- 본문이 차단 사유를 성공처럼 보이는 문장 안에 숨길 수 있다는 의미.
- 빠진 증거나 사용할 수 없는 아티팩트를 닫기 문구로 충족할 수 있다는 의미.
- 기존 대기 요청의 질문, 선택지, 맥락, form, 캡처 경로, 명령, URL, credential이 차단
  사유나 다음 행동 문구에 들어갈 수 있다는 의미. Agent-safe 요약과 일반 User Channel
  안내만 사용합니다. 별도의 요청 전 `missing_final_acceptance` 상태는 요청 생성에 필요한
  질문을 표시할 수 있습니다.

### 사용자에게 보이는 문구

닫기 위치 문구를 씁니다.

- `닫기 확인: {blocked_or_ready}.`
- `닫히지 않음: {blocker_summary}.`
- `닫기 시도를 할 준비는 되었지만, 이 확인으로 닫힌 것은 아닙니다.`
- `기록된 닫기 결과로 닫힘: {close_ref}.`
- `이어 가는 연속성: {continuity_summary}.`

`volicord.close_task`가 실제 상태 변경 닫기 결과를 반환했을 때만
`기록된 닫기 결과로 닫힘`을 씁니다.

### 담당 문서 링크

- [Core 모델](core-model.md): 닫기 준비 상태, 정직한 닫기, 최종 수락, 잔여 위험 경계.
- [닫기 메서드](api/method-close-task.md): `volicord.check_close`와 `volicord.close_task` 동작.
- [API 상태 스키마](api/schema-state.md): `CloseReadinessBlocker`.
- [API 판단 스키마](api/schema-judgment.md): 최종 수락과 수락된 위험 입력 형태.
- [API 오류 처리 경로](api/error-routing.md): 닫기 거부 응답 분기 경로.
- [API 차단 사유 처리 경로](api/blocker-routing.md): 차단 사유 처리 경로.

<a id="agent-context-packet-body"></a>
## 에이전트 맥락 패킷 본문

### 입력 상태

- 현재 작업 요약, 현재 적용 범위, 범위 밖 항목.
- 대기 중인 `AgentSafeUserActionRequestSummary` 항목, 차단 사유, 다음 안전한 행동.
- 증거 공백과 아티팩트 가용성 요약.
- 닫기 준비 상태, 잔여 위험 요약, 보장 수준.
- 출처 참조와 최신성 단서.
- 에이전트가 안전하게 추론할 수 있는 범위에 영향을 주는 현재 적용 연결 맥락과 역량 한계.
- 다음 행동에 필요한 언어와 담당 섹션만 사용합니다.

### 반드시 표시할 것

- 담당 기록을 대신하지 않는 에이전트용 압축 지원 패킷.
- 표시 대상이 Markdown, JSON에 가까운 텍스트, 또는 다른 표시 형태를 사용할 때 읽기 쉬운 구조.
- 패킷 안에서 권한과 최신성 단서가 보이는 상태.
- 현재 작업과 범위의 압축 요약.
- 대기 중인 사용자 소유 행동과 차단 사유.
- 다음 안전한 행동과 에이전트가 아직 하면 안 되는 행동.
- 증거, 아티팩트, 닫기 준비 상태, 잔여 위험, 보장 한계.
- 출처 참조, 원천 최신성, 사용할 수 없음 또는 역량 제한 조건.

### 암시하면 안 되는 것

- 패킷이 Core 상태, 저장소 상태, 증거, 수락, 잔여 위험 수락, 닫기 출력이라는 의미.
- 오래된 패킷이 담당 메서드가 반환한 더 최신 상태보다 우선한다는 의미.
- 에이전트가 사용자 판단, 쓰기 티켓, 아티팩트 규칙, 닫기 차단 사유를 우회할 수 있다는 의미.
- 전체 스키마, DDL, 로그, 아티팩트 본문, 관련 없는 계약 자료를 기본으로 주입해야 한다는 의미.
- 지원 범위 밖 기능 자료나 같은 `doc_id`의 한영 문서를 기본으로 주입해야 한다는 의미.
- 대기 요청의 질문, 선택지, 맥락, resolution form, 캡처 경로, 명령, URL, credential을
  포함하거나 다시 만들어야 한다는 의미.

### 사용자에게 보이는 문구

사용자나 채팅 표시 대상에 보이면 읽기 전용 지원 맥락이라고 표시합니다.

- `에이전트 맥락 패킷, 읽기 전용 지원 맥락.`
- `원천 상태: {state_version}; 관찰 시각: {observed_at}.`
- `진행 전에 필요한 것: {blocked_items}.`
- `다음 안전한 행동: {next_action}.`

패킷을 기록, 승인, 닫기 결과처럼 보이게 하는 문구를 피합니다.

### 담당 문서 링크

- [Agent Connection](agent-connection.md): 현재 연결 맥락과 연결 역량 선언.
- [상태 보기와 템플릿 표시 경계](projection-and-templates.md): 읽기 전용 표시와 최신성 경계.
- [Core 모델](core-model.md): 권한, 사용자 소유 판단, 닫기 준비 상태, 잔여 위험 경계.
- [API 상태 스키마](api/schema-state.md), [API 판단 스키마](api/schema-judgment.md), [API 아티팩트 스키마](api/schema-artifacts.md): 패킷 입력 형태.
- [보안](security.md): 보장 표현.
