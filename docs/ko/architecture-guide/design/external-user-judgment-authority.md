# 외부 사용자 판단 권한 설계

## 목적

이 설계는 agent가 input을 요청하고 나중에 agent-safe projection에서 작업을 계속할 수
있게 하면서 user-owned judgment resolution을 Agent Connection 밖에 두는 구현 방식을
설명합니다.

## 설계

Core는 사용자 소유 동작 하나를 엄격한 `UserActionRequest` 하나와 최대 하나의 immutable
`UserActionResolution`으로 표현합니다. MCP adapter는 agent-facing 요청과 resume
경로만 노출합니다. Local CLI inbox는 typed adapter-neutral
`PendingUserActionFacts`를 얻고 공유 presentation으로 의미
`UserActionResolutionForm`을 command-model invocation이 포함된 typed CLI inbox
model로 표시한 뒤 Core를 통해 명시적인 local-user resolution을 제출합니다.

UserAction 서비스는 타입으로 표현한 행동 의도를 검증하고 정규 request를
구성·구체화하며, 현재 권한을 해석하고 도메인 소유 pending 상태와 User Channel
안내 대신 semantic pending, current, availability, safe resolution fact를
투영합니다. UserAction 메서드 모듈은 request/resume 및 resolution 조율,
User Channel 권한 검사, replay, neutral fact read를 담당합니다. Core는 CLI
command, channel label, rendered instruction, capture metadata, MCP envelope를
구성하지 않습니다.

UserAction 서비스는 현재 basis와 action relevance를 평가합니다. Core는 invocation
provenance와 close 또는 write compatibility를 평가합니다. Adapter는 chat text,
summary text, model이 작성한 recommendation에서 judgment를 추론하지 않습니다.

## 불변 조건

- Agent Connection은 user-owned action을 요청하거나 resume할 수 있지만 해결할 수
  없습니다.
- Local-user provenance는 User Channel 경계에서 파생합니다.
- Request 하나에는 최대 하나의 immutable resolution이 있습니다.
- Agent-safe projection은 private form, note, path, command, user-only result material을
  제외합니다.
- Policy 기반 close 평가는 추론한 사용자 답변과 구분됩니다.
- Prompt capture와 diagnostic observation은 judgment authority를 만들지 않습니다.

## 책임 경계

UserAction 서비스는 공유 request 구성과 구체화, 권한 해석, lifecycle 정책, action
relevance, 영속화 매핑, continuity 파생, adapter-neutral fact를 담당합니다. Core
메서드 모듈은 typed 공개 request/resolution transition, User Channel 읽기,
transaction 순서, 요청별 결과 구성을 담당합니다. 나머지 Core policy 모듈은 각자의
집중된 순수 정책 평가를 담당합니다. `volicord-command-model`은 정규 CLI syntax를
담당합니다. `volicord-user-action-presentation`은 공유 CLI 지향 inbox와 recovery
presentation을 담당합니다. `volicord-cli`는 terminal rendering과 명시적인 choice
collection을 담당합니다. `volicord-mcp`는 agent-safe protocol projection, neutral
failure mapping, 공유 CLI fallback 부착을 담당합니다. Store는 엄격한 request 및
resolution record와 coherent resolution snapshot을 담당합니다.

## 실행 흐름

1. Agent-facing Core 호출이 현재 request를 만들거나 resume합니다.
2. MCP는 agent-safe summary와 continuation route만 투영합니다.
3. CLI inbox가 Core에 현재 adapter-neutral pending fact를 요청합니다.
4. 공유 presentation이 의미 resolution form을 투영하고 정규 command-model
   invocation을 도출합니다.
5. Local user가 typed CLI presentation에서 명시적인 action 하나를 선택합니다.
6. Core가 actor provenance, basis, expiry, 현재 work coordinate를 다시 검증합니다.
7. Store가 immutable resolution과 관련 authority event를 commit합니다.
8. 이후 agent 호출은 MCP의 safe current projection만 관찰합니다.

## 실패 동작

Request가 stale, expired, superseded, corrupt, already-resolved 상태이거나 provenance가
다르면 새 resolution 없이 실패합니다. Concurrent matching replay는 기존 immutable
결과를 반환하며 충돌하는 input은 request를 분기하지 못합니다. Adapter fallback
output은 form이나 answer를 꾸며 내지 않습니다.

## 범위 제외

이 설계는 user identity, authentication, non-repudiation, judgment kind, option meaning,
close policy를 정의하지 않습니다. 모든 host에 UI 하나를 요구하지 않으며 일반 chat을
User Channel로 만들지 않습니다.

## 구현 경로

- [`crates/volicord-user-action-service/src/`](../../../../crates/volicord-user-action-service/src/):
  typed request 구성과 구체화, 현재 권한 해석, lifecycle 및 relevance 의미,
  영속화 매핑, continuity 파생, neutral semantic fact.
- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs):
  직접 request 및 resolution 메서드 조율. 인접한 `user_action_read.rs`와
  `user_action_continuity.rs`는 User Channel read/replay와 continuity 영속화
  조율을 담당합니다.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs):
  local User Channel 조율과 terminal rendering.
- [`crates/volicord-command-model/src/lib.rs`](../../../../crates/volicord-command-model/src/lib.rs)와
  [`crates/volicord-user-action-presentation/src/lib.rs`](../../../../crates/volicord-user-action-presentation/src/lib.rs):
  정규 CLI syntax와 공유 CLI 지향 presentation.
- [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  agent-safe compound protocol projection과 neutral failure mapping.
- [`crates/volicord-store/src/core_pipeline/user_actions.rs`](../../../../crates/volicord-store/src/core_pipeline/user_actions.rs):
  엄격한 request, snapshot, resolution persistence.

## 참조 담당 문서

정확한 권한과 메서드 동작은 [Core 모델](../../reference/core-model.md),
[사용자 행동 요청](../../reference/api/method-request-user-action.md),
[사용자 행동 해결](../../reference/api/method-resolve-user-action.md),
[사용자 행동 스키마](../../reference/api/schema-user-action.md),
[Agent Connection](../../reference/agent-connection.md),
[관리 CLI](../../reference/admin-cli.md),
[보안](../../reference/security.md)에 남습니다.
