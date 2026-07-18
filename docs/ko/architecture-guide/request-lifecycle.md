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
5. 메서드 planner가 일관된 snapshot 하나를 읽고 typed outcome과 정확한 제안 효과를
   만듭니다.
6. 읽기 전용 분기는 변경 없이 반환합니다. Mutation 분기는 commit 전제 조건을 다시
   검증하고 Store transaction 하나를 원자적으로 적용합니다.
7. 공개 응답을 한 번 직렬화하고 MCP가 권한 의미를 바꾸지 않은 채 담당 문서의 detail을
   projection합니다.

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

Planner는 닫힌 outcome과 정확한 commit input을 반환합니다. Store는 transaction 안에서
담당 문서의 최종 검증을 수행하고 immutable row 삽입, current pointer 갱신, 해당할 때
authority event와 replay 추가, `state_version` 정확히 한 번 증가를 수행합니다.

Rejected, dry-run, unavailable, corrupt, unsupported-contract, conflict 분기는
[저장 효과](../reference/storage-effects.md)를 따릅니다. 가까운 성공 분기의 효과를
빌려오지 않습니다.

## 쓰기 티켓 흐름

`prepare_write`는 현재 Task, Change Unit, scope, baseline, policy, 민감 승인, 정규
path, 현재 write-authority fingerprint를 평가합니다. 기존 ticket은 담당 문서의 모든
좌표가 계속 유효할 때만 재사용할 수 있습니다. `record_run`은 ticket을 다시 검증하고
Run과 같은 commit 안에서 정확히 일치하는 효과만 소비합니다.

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

공개 메서드 결과가 권한을 담는 응답으로 남습니다. MCP structured content는 광고한
schema를 만족하고 text는 제한된 사람용 rendering입니다. Compact schema와 summary
view는 표시 detail을 생략할 수 있지만 필요한 권한 좌표를 빼거나 server validation을
느슨하게 할 수 없습니다.

## 관련 담당 문서

- [MCP 전송](../reference/mcp-transport.md)
- [API 메서드](../reference/api/methods.md)
- [저장 효과](../reference/storage-effects.md)
- [실패 모델](../reference/failure-model.md)
- [Guard suppression](../reference/guard-suppression.md)
