# 요청 Lifecycle

이 가이드는 관리 stdio 요청이 adapter 검증, Core planning, Store 접근, commit, 응답
projection을 거치는 흐름을 설명합니다.

## 전체 흐름

```text
Codex -> stdio MCP -> public argument DTO -> Core request -> plan
      -> Store read/validation -> optional atomic commit -> public result
      -> MCP projection -> Codex
```

1. stdio 프로세스가 관리 launch 맥락을 해석하고 Connection과 프로젝트 선택을
   검증하며 정확한 StorageManifest를 열고 권위 있는 runtime/project session을
   기록합니다.
2. JSON-RPC가 lifecycle, method 이름, 공개 argument 객체를 검증합니다.
3. MCP adapter는 각 project tool 호출마다 숨은 envelope 또는 invocation 필드를
   거부하고 현재 관리 runtime/project session을 검증한 뒤 서버 소유 context에서
   완전한 Core 요청을 만듭니다.
4. Core 공통 preflight가 typed Agent Session, actor, operation category, project,
   replay identity, expected state, 현재 Task context, 구조 입력을 검증합니다.
5. 메서드 계획 코드는 일관된 snapshot 하나를 읽고 메서드별 결과 필드를 담은
   typed 값과 정확한 제안 효과를 만듭니다. 공유 의미 작업은 집중된 identity,
   artifact, continuity, evidence, Write Ticket, 닫기 준비 상태, projection 담당
   모듈에 위임합니다. 이 담당 모듈은 typed fact나 plan을 반환하고 메서드 응답을
   구성하지 않습니다.
6. 공유 파이프라인이 typed 읽기 전용, 효과 없음, dry-run, 커밋 분기를 선택합니다.
   Mutation 분기는 commit 전제 조건을 다시 검증하고 Store transaction 하나를
   원자적으로 적용합니다.
7. 분기의 효과, 상태 버전, 이벤트, 재실행 사실이 확정된 뒤 파이프라인이 완전한
   공개 응답을 한 번 구성하고 직렬화합니다. MCP는 권한 의미를 바꾸지 않은 채
   담당 문서의 세부 내용을 projection합니다.

Core 전 실패는 Core 또는 Store 효과가 없습니다. Commit 뒤 실패는 operation-result
복구 좌표를 보존하고 mutation을 암시적으로 다시 시도하지 않습니다.

## 읽기 전용 요청

`volicord.status`, `volicord.check_close`, 적격
`volicord.get_operation_result`는 일관된 read snapshot을 사용합니다. Replay row,
authority event, current pointer, state-version 증가를 만들지 않습니다. Typed pagination
cursor는 lookup 전에 검증합니다.

## 구조적 거부

구조 입력 검증은 policy와 저장 mutation보다 먼저입니다. 특히
`volicord.prepare_write`는 현재 Change Unit이 없으면 ticket lookup, invalidation,
policy 평가, 그 밖의 효과 전에 `NO_ACTIVE_CHANGE_UNIT`과
`details.reason=current_change_unit_required`로 거부합니다. 이는 policy
`NotAllowed` 결정이 아니라 `Rejected`입니다.

## Mutation planning과 commit

계획 코드는 닫힌 outcome, typed 메서드 필드 값, 정확한 commit input을 반환합니다.
[`crates/volicord-types/src/methods.rs`](../../../crates/volicord-types/src/methods.rs)의
공개 메서드 선언 하나가 메서드 이름, 요청 및 결과 타입, 정확한 응답 계열, 계약 ID,
스키마, 커밋 결과 replay 적격 여부와 정확한 성공 결과 효과를 함께 결합합니다.
공유 파이프라인은 분기를 선택하는 동안 선언된 필드 타입과 메서드별 base 타입을
유지하고, 메서드 응답 계열에 없는 효과를 거부합니다. Store는 transaction 안에서
담당 문서의 최종 검증을 수행하고
immutable row 삽입, current pointer 갱신, 해당할 때 authority event와 replay 추가,
`state_version` 정확히 한 번 증가를 수행합니다.

메서드 결과 분기에서는 공통 사실이 확정된 뒤에만 파이프라인이 해당 메서드의
정확한 결과 메타데이터를 구성합니다. Compile-time 분기 capability는 메서드가
선언한 읽기 전용, 효과 없음, 스테이징, 커밋 생성자만 노출합니다. 효과별 타입은
고정된 dry-run 값과 이벤트 개수 규칙을 담당합니다. 거절 응답과 dry-run 응답은
서로 다른 `ToolRejectedBase`, `ToolDryRunBase` 메타데이터 타입을 사용합니다.
Strict decoding은 untagged 계열이 다른 분기를 선택하기 전에 알 수 없는 필드,
분기 사이에 섞인 필드, 해당 메서드에서 불가능한 효과를 거절합니다.

CLI와 MCP adapter는 rendering이나 protocol projection 전에 반환된 공개 객체를 해당
메서드의 정확한 응답 계열로 decode합니다. 따라서 adapter carrier는 Core와 메서드
스키마가 선언하지 않은 응답 분기를 추가할 수 없습니다.

Rejected, dry-run, unavailable, corrupt, unsupported-contract, conflict 분기는
[저장 효과](../reference/storage-effects.md)를 따릅니다. 가까운 성공 분기의 효과를
빌려오지 않습니다.

## Record Run 흐름

공통 preflight 뒤 공개 `record_run` 진입점은 공개 요청을 `RecordRunInput`으로
변환해 기록 패키지에 위임합니다. Envelope 식별자, idempotency, 예상 상태,
locale, replay, 응답 metadata는 메서드 조율에 남습니다. `recording/context.rs`는
의미 입력을 정규화하고 현재 typed 연산 fact를 취득합니다. 그다음
`recording/authority.rs`, `recording/evidence.rs`, `recording/artifact.rs`가 집중된
증거 및 artifact 담당 모듈을 재사용해 캡처 intent 및 receipt 해석, 증거 관찰과
producer 계획, artifact 검증 또는 승격 계획을 수행합니다.

Write Ticket 담당 모듈은 의미 연산 fact로 필요한 ticket을 승인합니다. 닫기 준비
상태의 recording 담당 모듈은 닫기 근거 참조와 잔여 위험 입력을 해석합니다.
`recording/plan.rs`는 이 결과를 하나의 typed mutation plan으로 결합하며 도메인별
mutation variant와 Store 적용 순서를 보존합니다. `recording/state.rs`는 연산 후
상태 fact를 취득하며, Recording은 typed effect와 결과 fact를 담은
`RecordRunOperationPlan`을 반환합니다. 공개 메서드는 이 fact를 중립
`OperationPlan`과 메서드 소유 `RecordRunResultFields`로 변환합니다. 공개 메서드
모듈만 의미 오류를 응답 분기로 매핑하며 공유 pipeline이 transaction, replay,
응답 envelope 담당을 유지합니다.

## 쓰기 티켓 흐름

`prepare_write`는 `crates/volicord-core/src/write_ticket/`를 통해 현재 Task,
Change Unit, scope, baseline, policy, 민감 승인, 정규 path, 현재 write-authority
fingerprint를 평가합니다. 발급 예정이면 `planning.rs`가 검증된
`PlannedWriteTicket` 하나를 만듭니다. 메서드 projection과 Store 삽입 입력은 같은
의미 값에서 파생합니다. Dry run은 ID가 없는 plan을 유지하고 미리보기 효과만
반환하며, 재사용은 이미 decode된 `StoredWriteTicket`을 전달합니다. 기존 ticket은 담당 문서의 모든
좌표가 계속 유효할 때만 재사용할 수 있습니다. `record_run`에서는
`write_ticket/admission.rs`가 typed Task, Change Unit, invocation, observed-change,
policy fingerprint, operation fact를 받습니다. 물리 row를 decode하거나 메서드
응답을 구성하지 않고 같은 validity 및 attempt-scope 정책을 적용합니다. 기록
plan은 정확히 일치해 승인된 ticket만 Run과 같은 commit 안에서 소비합니다.

## UserAction 분리

`volicord.request_user_action`은 strict pending request를 만들거나 명시적인 read-only
resume 분기를 사용합니다. MCP adapter는 agent-safe summary와 현재 projection만
반환하며 해결 form을 표시하거나 제출하지 않습니다.

로컬 CLI inbox가 strict stored form을 읽고 local-user provenance로
`volicord.resolve_user_action`을 호출합니다. Resolution은 별도 user-only mutation이며
원래 요청 결과를 대신하지 않습니다. Guard prompt 관찰은 계속 관찰입니다.

## Guard suppression

조정은 제한된 suppression service를 호출합니다. `Applied`는 정확한 remaining path와
suppression record를 담습니다. `Unavailable`은 모든 observed path, reason, scan budget,
observed count를 보존합니다. Store 실패나 손상 correlation이 빈 성공이 되지 않습니다.

## 응답 projection

공개 메서드 결과가 권한을 담는 응답으로 남습니다. 평면 JSON 형태와 생성 schema는
완전한 공개 결과 타입에서 나오며, 메서드 계획은 메서드별 필드만 다룹니다. 커밋된
재실행 행은 완전한 직렬화 결과를 저장하고, 재실행 decode도 같은 현재 타입으로
검증합니다. MCP structured content는 광고한 schema를 만족하고 text는 제한된 사람용
rendering입니다. Compact schema와 summary view는 표시 detail을 생략할 수 있지만
필요한 권한 좌표를 빼거나 server validation을 느슨하게 할 수 없습니다.

## 관련 담당 문서

- [MCP 전송](../reference/mcp-transport.md)
- [API 메서드](../reference/api/methods.md)
- [저장 효과](../reference/storage-effects.md)
- [실패 모델](../reference/failure-model.md)
- [Guard suppression](../reference/guard-suppression.md)
