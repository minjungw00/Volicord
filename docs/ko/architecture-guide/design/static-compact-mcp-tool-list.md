# 정적 압축 MCP 도구 목록 설계

## 목적

이 설계는 현재 MCP adapter가 runtime schema와 mutation result를 한도 안에 두면서
선택한 protocol capability를 통해 하나의 closed mode-specific tool catalog를 투영하는
방식을 설명합니다.

## 설계

`volicord-types::tool_names::AgentToolId`는 Core method와 operational verification
tool을 위한 canonical identity catalog입니다. `volicord-mcp-protocol`은 reviewed
production profile 하나와 semantic capability를 선택합니다. `tool_registry.rs`는
canonical definition을 한 번 만들고 Connection mode로 filter한 뒤 선택한 capability를
통해 wire definition을 투영합니다.

Runtime request schema는 documentation-only example을 생략합니다. Tool result는 typed
canonical content, schema validation, compact mutation projection, bounded
committed-result recovery, owner-method-bearing next-action data를 사용합니다. Tool
ownership은 protocol revision별로 갈라지지 않습니다.

## 불변 조건

- 현재 tool catalog는 closed 상태이고 `AgentToolId`를 key로 사용합니다.
- Connection mode는 Task-phase discovery state를 만들지 않고 availability를 filter합니다.
- Protocol capability는 wire projection에 영향을 주지만 tool의 semantic owner를
  바꾸지 않습니다.
- Runtime schema는 documentation-only material을 생략하면서 required validation과
  authority field를 보존합니다.
- Compact mutation result는 actionable recovery 및 next-action coordinate를
  유지합니다.
- Generated contract snapshot은 derived check이며 다른 tool owner가 아닙니다.

## 책임 경계

`volicord-types`는 tool identity와 shared method/result type을 담당합니다.
`volicord-mcp-protocol`은 protocol profile과 capability selection을 담당합니다.
`volicord-mcp`는 canonical registry construction, mode filtering, schema projection,
dispatch, result wrapping을 담당합니다. Core와 집중 참조 담당 문서는 method behavior를
계속 담당합니다.

## 실행 흐름

1. MCP initialization이 지원되는 protocol profile을 선택합니다.
2. Adapter startup이 현재 Connection mode와 session context를 해석합니다.
3. `tools/list`가 canonical mode-specific catalog를 얻습니다.
4. 각 definition을 선택한 protocol capability로 투영합니다.
5. `tools/call`이 wire name을 `AgentToolId`로 해석하고 현재 argument type으로 decode한
   뒤 담당자에게 dispatch합니다.
6. Adapter가 complete, compact, recovery result를 검증하고 wrapping합니다.

## 실패 동작

Unknown tool, unsupported protocol profile, missing required catalog entry, invalid schema,
oversized projection, post-effect response failure은 typed protocol 또는 adapter 경로를
유지합니다. Adapter는 validation field를 조용히 버리거나 version-specific alternate
registry를 선택하거나 committed mutation을 다시 실행해 output을 만들지 않습니다.

## 범위 제외

이 설계는 public tool list, schema field, byte limit, Connection mode, `next_action`
meaning, protocol support를 정의하지 않습니다. State-dependent dynamic tool discovery를
제공하지 않습니다.

## 구현 경로

- [`crates/volicord-types/src/tool_names.rs`](../../../../crates/volicord-types/src/tool_names.rs):
  canonical tool identity와 verification role.
- [`crates/volicord-mcp-protocol/src/lib.rs`](../../../../crates/volicord-mcp-protocol/src/lib.rs):
  closed production profile과 semantic capability.
- [`crates/volicord-mcp/src/tool_registry.rs`](../../../../crates/volicord-mcp/src/tool_registry.rs):
  canonical definition, mode filtering, projection.
- [`crates/volicord-mcp/src/tool_dispatch.rs`](../../../../crates/volicord-mcp/src/tool_dispatch.rs),
  [`mutation_projection.rs`](../../../../crates/volicord-mcp/src/mutation_projection.rs),
  [`committed_result_recovery.rs`](../../../../crates/volicord-mcp/src/committed_result_recovery.rs):
  dispatch와 bounded result 경로.

## 참조 담당 문서

정확한 동작은 [MCP 전송](../../reference/mcp-transport.md),
[Agent Connection](../../reference/agent-connection.md),
[API 메서드](../../reference/api/methods.md), 집중 API 스키마 및 메서드 담당 문서에
남습니다.
