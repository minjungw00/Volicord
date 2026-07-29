# 통합 UserAction 요청과 해결 설계

## 목적

이 설계는 하나의 durable UserAction request lifecycle, 하나의 local-user resolution
transition, 서로 다른 agent-safe 및 User Channel projection을 위한 현재 typed
architecture를 설명합니다.

## 설계

Core는 지원되는 모든 action family에 엄격한 공유 `UserActionRequest`와
`UserActionResolution` 타입을 사용합니다. 전용
`volicord-user-action-service` 크레이트는 의미 의도, 검증, 정규 typed 본문 구성,
안정적인 source identity, 권한 정규화, lifecycle 해석, 해결, 영속화 계획, 구체화,
continuity 사실, adapter-neutral 의미 projection을 담당합니다. 이 크레이트는 작은
typed 연산 및 영속화 context를 받고 서비스 소유 typed 결과와 오류를 반환합니다.
Core 메서드, pipeline, 응답, CLI, MCP, command model, presentation 구현은 가져오지
않습니다.

`methods/user_action.rs`는 공개 요청과 해결 조율을 담당합니다.
`methods/user_action_read.rs`는 서비스 소유 중립 사실을 둘러싼 Core 승인 검사와
원래 결과 replay를 담당합니다. `continuity/user_action.rs`는 재사용 가능한 continuity
계획과 영속화 draft를 담당하고, `error_boundary/user_action.rs`는 메서드 응답
경계에서만 서비스 소유 typed 오류를 매핑합니다. `reconcile_changes.rs`를 포함한
다른 Core 메서드는 같은 서비스 크레이트를 호출하고 typed 결과를 각자 메서드 계획과
응답에 반영합니다.

Store의 `core_pipeline/user_actions.rs`는 유효 상태 읽기, 일관된 받은 편지함 해결
snapshot, 변경 불가능한 resolution 삽입, grouped mutation 적용, 물리 JSON과 저장
값을 opaque `StoredUserActionRequest`, `StoredUserActionResolution`, paired
`StoredUserActionRecordSet` 값으로 바꾸는 집중 decoding을 담당합니다. Store는 값을
반환하기 전에 중복 표현, 닫힌 영속 값, 요청-resolution 관계를 검증합니다. Raw row
표현과 손상 구성은 Store 내부에만 둡니다. 서비스는 전체 프로젝트 Store facade나
직렬화된 값 대신 집중된 `UserActionStoreReader` typed 읽기 capability를 사용합니다.

MCP adapter는 request/resume 경로를 호출하고 `CurrentUserActionFacts`를 다시 읽은
뒤 자체 safe protocol projection을 구성합니다. CLI는 `PendingUserActionFacts`를
사용합니다. `volicord-user-action-presentation`은 이 neutral fact에서 현재
의미 `UserActionResolutionForm`을 `CliUserActionInboxItem`, 폐쇄형 availability 및
capture-path 상태, CLI JSON Schema, recovery instruction으로 투영합니다. 명령은
실제 Clap 선언에서 도출한 typed `volicord-command-model` invocation을 사용합니다.

## 불변 조건

- Request 하나에는 resolution이 없거나 immutable resolution 하나만 있습니다.
- Request create/resume과 resolution은 서로 다른 Core operation입니다.
- Caller는 typed 의미 의도와 현재 도메인 사실을 제공하며 정규 request JSON을
  직접 만들지 않습니다.
- 정규 request body, basis, UserAction에서 파생한 continuity metadata는 Store
  경계까지 typed 상태를 유지합니다.
- Effective status는 stored resolution과 current basis에서 파생합니다.
- 정규화된 authority는 타입이 있는 완전한 프로젝트, `Task`, resolution
  identity를 노출하며 consumer는 문자열이나 주변 프로젝트 상태에서 이를 다시
  만들지 않습니다.
- Agent-facing projection은 complete resolving form이나 user-only resolution body를
  포함하지 않습니다.
- 서비스 fact 결과는 의미 좌표, lifecycle status, availability, safe resolution
  fact만 포함합니다. Command string, presentation label, CLI capture metadata,
  rendered instruction, MCP 이름의 envelope는 포함하지 않습니다.
- CLI resolution command는 이를 parsing하는 같은 Clap 선언에서 도출합니다.
- Local inbox는 resolution planning 전에 coherent Store snapshot 하나를 읽습니다.
- Resolution replay는 immutable authority state를 분기하지 못합니다.
- 유효하지 않은 영속 UserAction record는 일반 공개 Store record API로 조립할 수
  없습니다.
- 서비스는 Store가 검증한 typed record에서 의미 policy를 평가하며 영속 row
  일관성을 다시 구성하지 않습니다.

## 책임 경계

`volicord-types`는 dependency-safe request, 불변 resolution,
`UserActionResolutionIdentity`, `UserActionResolutionRef`, adapter-neutral
resolution form, basis, summary shape, 제품 경로, 의미 기반 `StateRecordRef`
생성자를 담당하며 CLI presentation helper는 담당하지 않습니다.
`volicord-user-action-service`는 재사용 UserAction 의미를 담당하고 공유 타입,
Store, 집중 유틸리티 라이브러리에만 의존합니다. Core는 현재 ID와 timestamp를
할당하고 invocation context를 검증하며 서비스를 호출하고 Store mutation
pipeline에 참여한 뒤 서비스 오류와 결과를 요청별 응답으로 매핑합니다. Store는
물리 영속화, 엄격한 row decoding, 영속 record 일관성, 검증된 record-set 구성,
snapshot 일관성을 담당합니다. 서비스 담당 invariant 오류는 유효한 typed fact의
불일치를 Store 손상과 구분합니다. Command model은 정규 CLI invocation 구성을 담당하고
`volicord-user-action-presentation`은 typed `Cli*` projection과 CLI JSON Schema를
담당합니다. CLI는 typed model의 직접 terminal rendering을 담당하며 MCP는 bounded
protocol projection과 adapter별 failure mapping을 담당합니다.

## 실행 흐름

1. Core 메서드가 typed 의미 행동 의도와 현재 도메인 사실을 UserAction 서비스에
   전달합니다.
2. 순수 검증이 의미 조합과 현재 좌표에 대한 typed validated intent를 반환합니다.
3. 서비스가 typed Store reader를 통해 나머지 현재 도메인 fact를 취득합니다.
4. 순수 body 구성이 정규 typed request body와 basis를 만듭니다.
5. Core가 영속 request ID와 operation identity를 제공하고, 서비스 identity와
   구체화 단계가 typed 공개 request, 검증된 Store record set, mutation plan을
   반환합니다.
6. 호출 메서드가 그 결과를 자신의 연산과 응답에 반영하거나 explicit resume
   projection을 반환합니다.
7. Store가 일반 Core commit으로 request를 지속 저장합니다.
8. MCP가 agent-safe summary와 continuation만 반환합니다.
9. CLI가 Task 하나의 neutral pending fact를 요청합니다.
10. Store가 project snapshot 하나에서 유효 typed record를 decode하고 서비스가 Core
    승인 경계를 통해 typed lifecycle 및 resolution-availability fact를 반환합니다.
11. 공유 presentation이 typed local CLI inbox item을 만들고 command model에서 정규
    typed resolution invocation을 얻습니다.
12. Core가 선택한 local-user answer를 검증하고 immutable resolution 하나를
    planning합니다.
13. Store가 resolution, derived record, event, replay response를 atomic하게
    commit합니다.

## 실패 동작

Malformed stored variant, missing basis 또는 form, stale coordinate, expiry, existing
conflicting resolution, invalid choice, provenance mismatch는 partial derived state 없이
실패합니다. Read-only status 계산은 시간이 지났다는 이유만으로 record를 변경하지
않습니다. Store는 물리 영속 row 또는 pairing failure를 손상으로 보고합니다. 유효한
typed fact 사이의 의미 projection 불일치는 서비스 invariant failure이며 table 또는
column identity를 담지 않습니다.

## 범위 제외

이 설계는 action kind, form, option semantic, effective status value, public method field,
delivery support, authority meaning을 정의하지 않습니다. Prompt capture나 MCP transport를
resolution channel로 만들지 않습니다.

## 구현 경로

- [`crates/volicord-types/src/schema.rs`](../../../../crates/volicord-types/src/schema.rs)와
  [`methods.rs`](../../../../crates/volicord-types/src/methods.rs):
  shared shape와 public result composition.
- [`crates/volicord-user-action-service/src/`](../../../../crates/volicord-user-action-service/src/):
  의미 model, typed 오류, 검증, 정규 본문 구성, identity, Store-aware service,
  영속화 계획, 구체화, 권한과 lifecycle 해석, 해결, continuity 사실, 중립
  projection, summary.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs)와
  [`user_action_read.rs`](../../../../crates/volicord-core/src/methods/user_action_read.rs):
  공개 메서드 조율, Core 소유 승인 및 replay.
- [`crates/volicord-core/src/continuity/user_action.rs`](../../../../crates/volicord-core/src/continuity/user_action.rs)와
  [`error_boundary/user_action.rs`](../../../../crates/volicord-core/src/error_boundary/user_action.rs):
  서비스가 도출한 continuity 계획과 집중된 메서드 경계 오류 매핑.
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
