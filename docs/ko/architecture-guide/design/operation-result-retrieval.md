# 작업 결과 조회 설계

## 목적

이 설계는 원래 mutation을 반복하거나 다른 result store를 만들지 않고 Core가 immutable
replay response에서 한도가 있는 정확한 과거 작업 결과를 조회하는 방식을 설명합니다.

## 설계

일반 committed Core mutation은 직렬화한 공개 response를 Store가 담당하는 replay row에
저장합니다. 조회 가능한 projection은 그 row에서 파생한 `OperationResultRef`를
전달합니다. Read-only Core 메서드는 현재 invocation과 reference를 검증하고 Store에
정확한 scoped replay response를 요청하며 content fact를 확인한 뒤 UTF-8-safe page를
반환합니다.

완전한 body가 transport budget을 넘으면 MCP mutation projection은 간결하고 실행
가능한 result를 유지합니다. `committed_result_recovery.rs`와
`mutation_projection.rs`는 그 projection에서 operation-result reference를 보존하여
caller가 별도 read-only 메서드를 사용할 수 있게 합니다.

## 불변 조건

- Exact replay와 exact retrieval은 같은 immutable response body를 읽습니다.
- Result reference와 cursor는 locator이며 authorization credential이 아닙니다.
- 각 page는 bytes를 반환하기 전에 현재 access, result identity, cursor binding,
  integrity를 다시 검증합니다.
- Page boundary는 UTF-8 code point를 나누지 않습니다.
- Retrieval은 effect를 replay하거나 authority event 및 replay row를 추가하거나 project
  state를 전진시키지 않습니다.
- Historical output은 current Core authority와 구분됩니다.

## 책임 경계

Core는 invocation validation, reference와 cursor check, integrity decision, page
composition을 담당합니다. Store는 scoped immutable replay-row read를 담당합니다.
MCP는 compact mutation projection과 recovery coordinate 보존을 담당하며 historical
result를 다시 만들지 않습니다.

## 실행 흐름

1. 일반 committed mutation이 exact serialized response를 replay row에 저장합니다.
2. Core가 조회 가능한 projection을 위한 result reference를 파생합니다.
3. MCP adapter가 그 reference와 함께 complete 또는 compact projection을 반환합니다.
4. Read-only retrieval 메서드가 request를 검증하고 exact replay response를 읽습니다.
5. Core가 reference와 cursor를 확인하고 한도가 있는 UTF-8 page를 선택하며 필요하면
   next cursor를 반환합니다.

## 실패 동작

Missing, corrupt, malformed, cross-project, cross-actor, cross-result,
cursor-incompatible read는 partial page를 반환하기 전에 실패합니다. Response-budget
failure는 mutation replay, artifact substitution, duplicate result table을 만들지
않습니다.

## 범위 제외

이 설계는 공개 메서드 schema, page limit, error code, retention, access contract를
정의하지 않습니다. 일반 artifact, event, Runtime Home file download 아키텍처가
아니며 이전 result를 current 상태로 만들지 않습니다.

## 구현 경로

- [`crates/volicord-core/src/methods/operation_result.rs`](../../../../crates/volicord-core/src/methods/operation_result.rs):
  read-only planning, validation, paging.
- [`crates/volicord-store/src/core_pipeline/replay.rs`](../../../../crates/volicord-store/src/core_pipeline/replay.rs):
  scoped replay-row lookup.
- [`crates/volicord-mcp/src/committed_result_recovery.rs`](../../../../crates/volicord-mcp/src/committed_result_recovery.rs)와
  [`mutation_projection.rs`](../../../../crates/volicord-mcp/src/mutation_projection.rs):
  bounded mutation projection과 recovery coordinate.
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../../crates/volicord-mcp/src/tool_dispatch.rs):
  transport dispatch와 final projection 선택.

## 참조 담당 문서

정확한 동작은
[`volicord.get_operation_result`](../../reference/api/method-get-operation-result.md),
[API 코어 스키마](../../reference/api/schema-core.md),
[MCP 전송](../../reference/mcp-transport.md),
[저장소 기록](../../reference/storage-records.md),
[저장 효과](../../reference/storage-effects.md),
[저장소 버전 관리](../../reference/storage-versioning.md),
[보안](../../reference/security.md)에 남습니다.
