# 외부 사용자 판단 권한 설계

## 목적

이 설계는 agent가 input을 요청하고 나중에 agent-safe projection에서 작업을 계속할 수
있게 하면서 user-owned judgment resolution을 Agent Connection 밖에 두는 구현 방식을
설명합니다.

## 설계

Core는 사용자 소유 동작 하나를 엄격한 `UserActionRequest` 하나와 최대 하나의 immutable
`UserActionResolution`으로 표현합니다. MCP adapter는 agent-facing 요청과 resume
경로만 노출합니다. Local CLI inbox는 typed `UserChannelInboxProjection`을 얻고 저장된
capture form을 표시한 뒤 Core를 통해 명시적인 local-user resolution을 제출합니다.

Core policy 모듈은 현재 basis, action relevance, actor provenance, close 또는 write
compatibility를 평가합니다. Adapter는 chat text, summary text, model이 작성한
recommendation에서 judgment를 추론하지 않습니다.

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

Core는 typed request/resolution transition과 policy 평가를 담당합니다.
`volicord-cli`는 local User Channel presentation과 명시적인 choice collection을
담당합니다. `volicord-mcp`는 agent-safe projection과 fallback guidance를 담당합니다.
Store는 엄격한 request 및 resolution record와 coherent resolution snapshot을
담당합니다.

## 실행 흐름

1. Agent-facing Core 호출이 현재 request를 만들거나 resume합니다.
2. MCP는 agent-safe summary와 continuation route만 투영합니다.
3. CLI inbox가 Task 하나의 현재 typed User Channel projection을 Core에 요청합니다.
4. Local user가 저장된 capture form에서 명시적인 action 하나를 선택합니다.
5. Core가 actor provenance, basis, expiry, 현재 work coordinate를 다시 검증합니다.
6. Store가 immutable resolution과 관련 authority event를 commit합니다.
7. 이후 agent 호출은 safe current projection만 관찰합니다.

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

- [`crates/volicord-core/src/methods/user_action.rs`](../../../../crates/volicord-core/src/methods/user_action.rs):
  request, inbox, resolution planning.
- [`crates/volicord-core/src/policy/user_action_relevance.rs`](../../../../crates/volicord-core/src/policy/user_action_relevance.rs)와
  [`policy/close_readiness.rs`](../../../../crates/volicord-core/src/policy/close_readiness.rs):
  현재 relevance와 authority 평가.
- [`crates/volicord-cli/src/user_command.rs`](../../../../crates/volicord-cli/src/user_command.rs):
  local User Channel 조율.
- [`crates/volicord-mcp/src/user_action_projection.rs`](../../../../crates/volicord-mcp/src/user_action_projection.rs):
  agent-safe compound projection.
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
