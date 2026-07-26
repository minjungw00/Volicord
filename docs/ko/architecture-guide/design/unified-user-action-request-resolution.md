# 통합 UserAction 요청과 해결 설계

## 목적

이 설계는 하나의 durable UserAction request lifecycle, 하나의 local-user resolution
transition, 서로 다른 agent-safe 및 User Channel projection을 위한 현재 typed
architecture를 설명합니다.

## 설계

Core는 지원되는 모든 action family에 strict shared `UserActionRequest`와
`UserActionResolution` type을 사용합니다. `methods/user_action.rs`는 create, resume,
inbox projection, resolution 경로를 planning합니다. Store의
`core_pipeline/user_actions.rs`는 effective-status read, coherent inbox resolution
snapshot, immutable resolution insertion, grouped mutation 적용을 담당합니다.

MCP adapter는 request/resume 경로만 호출하고 `AgentSafeUserActionRequestSummary`를
투영합니다. CLI inbox는 local presentation을 위한 complete trusted capture form을
포함하는 nonserialized `UserChannelInboxProjection` 경계를 사용합니다.

## 불변 조건

- Request 하나에는 resolution이 없거나 immutable resolution 하나만 있습니다.
- Request create/resume과 resolution은 서로 다른 Core operation입니다.
- Effective status는 stored resolution과 current basis에서 파생합니다.
- Agent-facing projection은 complete resolving form이나 user-only resolution body를
  포함하지 않습니다.
- Local inbox는 resolution planning 전에 coherent Store snapshot 하나를 읽습니다.
- Resolution replay는 immutable authority state를 분기하지 못합니다.

## 책임 경계

`volicord-types`는 dependency-safe request, resolution, form, summary shape를
담당합니다. Core는 lifecycle 및 basis policy와 method planning을 담당합니다. Store는
strict record와 snapshot consistency를 담당합니다. CLI는 local presentation을
담당하고 MCP는 bounded agent projection을 담당합니다.

## 실행 흐름

1. Core가 current basis와 capture form을 가진 request를 만들거나 explicit resume
   projection을 반환합니다.
2. Store가 일반 Core commit으로 request를 지속 저장합니다.
3. MCP가 agent-safe summary와 continuation만 반환합니다.
4. CLI가 Task 하나의 current inbox projection을 요청합니다.
5. Store가 project snapshot 하나에서 effective record와 pending form을 읽습니다.
6. Core가 선택한 local-user answer를 검증하고 immutable resolution 하나를 planning합니다.
7. Store가 resolution, derived record, event, replay response를 atomic하게 commit합니다.

## 실패 동작

Malformed stored variant, missing basis 또는 form, stale coordinate, expiry, existing
conflicting resolution, invalid choice, provenance mismatch는 partial derived state 없이
실패합니다. Read-only status 계산은 시간이 지났다는 이유만으로 record를 변경하지
않습니다.

## 범위 제외

이 설계는 action kind, form, option semantic, effective status value, public method field,
delivery support, authority meaning을 정의하지 않습니다. Prompt capture나 MCP transport를
resolution channel로 만들지 않습니다.

## 구현 경로

- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs)와
  [`methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  shared shape와 public result composition.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs)와
  [`lib.rs`](../../../../crates/volicord-core/src/lib.rs):
  request/resolution planning과 internal projection boundary.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  strict read, effective status, coherent snapshot, mutation.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)와
  [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  local 및 agent-facing projection.

## 참조 담당 문서

정확한 동작은 [사용자 행동 요청](../../reference/api/method-request-user-action.md),
[사용자 행동 해결](../../reference/api/method-resolve-user-action.md),
[사용자 행동 스키마](../../reference/api/schema-user-action.md),
[Core 모델](../../reference/core-model.md),
[Agent Connection](../../reference/agent-connection.md),
[관리 CLI](../../reference/admin-cli.md),
[저장소 기록](../../reference/storage-records.md)에 남습니다.
