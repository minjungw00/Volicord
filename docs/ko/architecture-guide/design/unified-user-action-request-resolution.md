# 통합 UserAction 요청과 해결 설계

## 목적

이 설계는 하나의 durable UserAction request lifecycle, 하나의 local-user resolution
transition, 서로 다른 agent-safe 및 User Channel projection을 위한 현재 typed
architecture를 설명합니다.

## 설계

Core는 지원되는 모든 action family에 strict shared `UserActionRequest`와
`UserActionResolution` type을 사용합니다. `methods/user_actions.rs`는 재사용되는
의미 검증, 정규 typed request 구성, identity 할당, Store mutation 구체화, 엄격한
권한 해석, 의미 lifecycle 정책을 담당합니다. `methods/user_action.rs`는 공개
request/resume 및 resolution 조율과 adapter-neutral current/pending fact read를
담당합니다. `reconcile_changes.rs`를 포함한 다른 Core 메서드는 공유 서비스를 직접
호출하고 그 typed 결과를 각자 연산에 반영하는 방식을 결정합니다.

Store의 `core_pipeline/user_actions.rs`는 effective-status read, coherent inbox
resolution snapshot, immutable resolution insertion, grouped mutation 적용을 담당합니다.

MCP adapter는 request/resume 경로를 호출하고 `CurrentUserActionFacts`를 다시 읽은
뒤 자체 safe protocol projection을 구성합니다. CLI는 `PendingUserActionFacts`를
사용합니다. `volicord-user-action-presentation`은 이 neutral fact에서 현재
`UserActionInboxItem`, availability label, CLI recovery instruction을 만듭니다.
명령은 실제 Clap 선언에서 도출한 typed `volicord-command-model` invocation을
사용합니다.

## 불변 조건

- Request 하나에는 resolution이 없거나 immutable resolution 하나만 있습니다.
- Request create/resume과 resolution은 서로 다른 Core operation입니다.
- Caller는 typed 의미 의도와 현재 도메인 사실을 제공하며 정규 request JSON을
  직접 만들지 않습니다.
- 정규 request body와 basis는 Store 경계까지 typed 상태를 유지합니다.
- Effective status는 stored resolution과 current basis에서 파생합니다.
- Agent-facing projection은 complete resolving form이나 user-only resolution body를
  포함하지 않습니다.
- Core fact 결과는 의미 좌표, lifecycle status, availability, safe resolution
  fact만 포함합니다. Command string, presentation label, CLI capture metadata,
  rendered instruction, MCP 이름의 envelope는 포함하지 않습니다.
- CLI resolution command는 이를 parsing하는 같은 Clap 선언에서 도출합니다.
- Local inbox는 resolution planning 전에 coherent Store snapshot 하나를 읽습니다.
- Resolution replay는 immutable authority state를 분기하지 못합니다.

## 책임 경계

`volicord-types`는 dependency-safe request, resolution, form, basis, summary shape를
담당합니다. Core UserAction 서비스는 재사용되는 구성, 구체화, 권한 해석,
lifecycle 정책, adapter-neutral semantic fact를 담당합니다. 개별 메서드 모듈은
요청별 연산과 응답 구성을 담당합니다. Store는 strict record와 snapshot
consistency를 담당합니다. Command model은 정규 CLI invocation 구성을 담당하고
`volicord-user-action-presentation`은 공유 CLI 지향 projection을 담당합니다. CLI는
terminal rendering을 담당하며 MCP는 bounded protocol projection과 adapter별 failure
mapping을 담당합니다.

## 실행 흐름

1. Core 메서드가 typed 의미 행동 의도와 현재 도메인 사실을 UserAction 서비스에
   전달합니다.
2. 서비스가 의미 조합과 현재 좌표를 검증합니다.
3. 서비스가 정규 typed request body와 basis를 구성합니다.
4. 서비스가 request identity를 할당하고 typed 공개 request, 유효 Store record,
   mutation plan을 반환합니다.
5. 호출 메서드가 그 결과를 자신의 연산과 응답에 반영하거나 explicit resume
   projection을 반환합니다.
6. Store가 일반 Core commit으로 request를 지속 저장합니다.
7. MCP가 agent-safe summary와 continuation만 반환합니다.
8. CLI가 Task 하나의 neutral pending fact를 요청합니다.
9. Store가 project snapshot 하나에서 effective record를 읽고 Core가 typed
   lifecycle 및 resolution-availability fact를 반환합니다.
10. 공유 presentation이 local inbox item을 만들고 command model에서 정규 typed
    resolution invocation을 얻습니다.
11. Core가 선택한 local-user answer를 검증하고 immutable resolution 하나를
    planning합니다.
12. Store가 resolution, derived record, event, replay response를 atomic하게
    commit합니다.

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
  직접 공개 메서드 조율과 adapter-neutral current-fact boundary.
- [`crates/volicord-core/src/methods/user_actions.rs`](../../../../crates/volicord-core/src/methods/user_actions.rs):
  공유 typed 구성, 구체화, 권한 해석, lifecycle 정책.
- [`crates/volicord-core/src/methods/reconcile_changes.rs`](../../../../crates/volicord-core/src/methods/reconcile_changes.rs):
  UserAction 서비스를 사용하는 reconciliation별 조율.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  strict read, effective status, coherent snapshot, mutation.
- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs)와
  [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  typed 정규 CLI invocation과 공유 CLI 지향 presentation.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs)와
  [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  terminal 및 MCP protocol projection.

## 참조 담당 문서

정확한 동작은 [사용자 행동 요청](../../reference/api/method-request-user-action.md),
[사용자 행동 해결](../../reference/api/method-resolve-user-action.md),
[사용자 행동 스키마](../../reference/api/schema-user-action.md),
[Core 모델](../../reference/core-model.md),
[Agent Connection](../../reference/agent-connection.md),
[관리 CLI](../../reference/admin-cli.md),
[저장소 기록](../../reference/storage-records.md)에 남습니다.
