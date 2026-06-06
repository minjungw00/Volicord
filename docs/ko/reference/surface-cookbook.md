# Surface Cookbook 참조

## 이 문서로 할 수 있는 일

이 참조는 활성 reference local surface recipe를 검토하고, 다른 surface 메모가 owner 승격 전 MVP 범위로 들어오지 않게 하는 데 씁니다.

이 문서는 surface마다 달라질 수 있는 local setup note, generated file name, MCP configuration hint, capture/guard/isolation option, common fallback, conformance risk를 담당합니다. 공통 surface contract와 활성 reference `capability_profile`은 [Agent 통합 참조](agent-integration.md)가 담당합니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 활성 reference local surface recipe를 작성하거나 리뷰할 때.
- later surface note를 공통 surface contract와 분리해서 유지해야 할 때.
- capture, guard, isolation, fallback, conformance risk를 보장 수준보다 강하게 말하지 않고 설명해야 할 때.

## 읽기 전에

공통 surface contract, capability profile, 단계별 맥락 profile은 [Agent 통합 참조](agent-integration.md)를 읽습니다. Local access, API error, conformance 질문에는 현재 필요한 owner section만 [런타임 아키텍처](runtime-architecture.md), [MVP API](api/mvp-api.md), [API Errors](api/errors.md), [운영과 Conformance 참조](operations-and-conformance.md)에서 사용합니다. 이 cookbook은 프롬프트 묶음이 아니며 connector에게 전체 Reference 문서를 불러오라고 요구하지 않습니다.

## 핵심 생각

surface 이름만으로 보장 수준을 추론하면 안 됩니다. 활성 MVP는 하나의 reference local surface profile을 대상으로 하며, 그 profile이 입증한 field가 guarantee level을 결정합니다.

Generic capability profile 규칙은 [Agent 통합 참조](agent-integration.md#capability-profiles)를 봅니다.

Surface recipe는 local-access public error code나 OS 수준 보안 보장을 새로 정의하지 않습니다. [런타임 아키텍처](runtime-architecture.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [보안 참조](security.md), [Agent 통합](agent-integration.md)의 Engineering Checkpoint/default reference local-only MCP posture를 따릅니다. 활성 recipe는 cooperative/detective 표현만 사용합니다. Preventive와 isolated option은 구체적인 profile이 승격되고 입증되기 전까지 향후/profile별 범위입니다. Recipe text만으로 guarantee level은 올라가지 않으며, 하네스 기록, 상태 전이, 범위, 사용자 판단, 증거, 닫기 가능 여부를 넘어서는 권한을 만들지 않습니다. Recipe는 raw token, secret, private configuration value를 노출하면 안 되며, surface name만으로 remote, shared, non-loopback, forwarded, tunneled MCP exposure를 암시하지 않습니다. MCP/Core에 닿을 수 없으면 `MCP_UNAVAILABLE` 또는 `MCP_SERVER_UNAVAILABLE` 같은 operations diagnostic을 사용합니다. 도달 가능한 local caller나 access mode가 registered profile 밖이면 표시해도 안전한 진단 세부 정보를 포함해 `LOCAL_ACCESS_MISMATCH`를 사용합니다. Recognized surface/profile이 required capability를 충족하지 못하면 `CAPABILITY_INSUFFICIENT`를 사용합니다. Surface별 `UNAUTHORIZED` public code를 도입하지 않습니다.

## Recipe shape

각 활성 recipe에는 surface별 내용만 둡니다.

- 활성 `capability_profile` field value 또는 owner section link
- generated file 또는 instruction
- MCP configuration hint
- local-only MCP exposure posture와 local transport 전제
- host/profile별 capability 차이
- manual artifact attachment path
- fallback instruction
- guarantee boundary note
- conformance smoke expectation. 단, pass claim은 하지 않음

Generic kernel rule, public API schema, policy contract, 전체 Reference 문서, 관련 없는 template, historical log, 전체 Storage DDL, 전체 Conformance catalog, 관련 없는 Roadmap 항목, projection 본문 전체를 여기서 반복하지 않습니다. 공통 contract가 cooperative, detective, preventive, isolated의 의미를 정합니다. Recipe는 그 동작을 제공할 수 있는 surface별 path만 이름 붙입니다. Guard, freeze, careful-mode label은 connected profile의 실제 capability 위에 얹힌 label로만 쓸 수 있습니다. Recipe가 이런 label을 쓰면 그 동작이 scope hold인지, post-action detector인지, fixture-proven pre-tool block인지, documented separation boundary인지 밝혀야 합니다. 이런 label은 write를 authorize하거나, gate를 충족하거나, verification을 기록하거나, acceptance를 기록하거나, 새 authority tier를 만들지 않습니다.

Generated 또는 managed recipe output은 [Agent 통합 참조](agent-integration.md#generated-manifest-기대사항)의 connector manifest contract를 따라야 합니다. Recipe는 surface별 file, config snippet, managed block을 이름 붙일 수 있지만 drift는 overwrite 전에 보고합니다. Existing file 또는 managed block은 reconcile 또는 explicit reconnect decision이 replacement를 선택하기 전까지 그대로 두며, drift된 generated file은 canonical Task state로 취급하지 않습니다.

아래 `guarantee_boundary` block은 recipe 문서용 설명일 뿐 public schema, DDL shape, canonical Capability Profile field가 아닙니다. Connector는 [Agent 통합 참조](agent-integration.md) contract에 맞는 경우에만 같은 사실을 Capability Profile 또는 Connector Manifest에 기록할 수 있습니다. Surface Cookbook은 surface별 path와 예시를 이름 붙일 뿐 guarantee level을 다시 정의하지 않습니다.

Recipe가 `fallback_isolation` 아래에 manual verification bundle을 적을 때는 verification/evaluator fallback input이라는 뜻입니다. Manual verification bundle만으로 연결된 surface가 `preventive` 또는 `isolated`로 올라가지 않습니다. `isolated` guarantee에는 여전히 문서화되고 입증된 separation boundary가 필요합니다. Worktree 또는 fresh bundle은 verification independence나 stale-context control을 뒷받침할 수 있습니다. OS sandbox, permission isolation, hard process/container isolation, tamper-proof security에는 connector profile이 exact mechanism을 이름 붙이고 증명해야 합니다. Isolation은 Approval, QA, final acceptance, residual-risk acceptance, close, verification result와 분리됩니다.

## Reference Local Surface

```yaml
surface_kind: reference_local_mcp
target_profiles:
  - local_mcp_reference
generated_files_or_instructions:
  - 선택한 surface에 필요할 때만 짧은 managed agent-instruction block
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - local-only registered project posture
  - active capability_profile에 listed된 public tools와 resources만 사용
  - generated MCP config path, managed hash, capability_profile freshness를 connector manifest에 기록
capability_profile_fields:
  surface_id: reference-local-mcp
  surface_name: Reference local MCP surface
  mcp_available: true
  public_tools_available: active MVP-1 method set, with Engineering Checkpoint using a subset
  resources_available: Engineering Checkpoint and MVP-1 read-only resource subset
  cooperative_prepare_write_supported: true
  changed_path_detection_supported: true
  artifact_capture_supported: false
  manual_artifact_attachment_supported: true
  command_observation_supported: false
  network_observation_supported: false
  secret_access_observation_supported: false
  pre_tool_blocking_supported: false
  isolation_supported: false
  max_guarantee_level: detective
  conformance_smoke_status: planned_not_run
guarantee_boundary:
  default_level: cooperative
  max_level: detective only for supported after-action observation
  can_block_before_execution: false
  can_detect_after_action: owner path가 관찰할 때 changed paths와 artifact gaps
  native_capture: none in the minimum reference profile
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact attachment
  fallback_isolation: active reference profile에서 지원하지 않음
capture_guard_isolation_options:
  - changed_paths validator
  - summary와 artifact ref를 위한 explicit record_run discipline
  - native capture가 unavailable일 때 manual artifact attachment
  - pre-tool blocking claim 없음
  - isolation claim 없음
common_fallbacks:
  - cooperative prepare_write discipline
  - detective changed-path validation
  - manual artifact attachment
  - MCP/Core 또는 required capability가 unavailable이면 product write를 instruction으로 hold
conformance_risks:
  - runtime fixture가 materialize되고 실행되기 전에는 conformance_smoke_status를 passed로 보고하지 않음
  - native artifact capture, command observation, network observation, secret access observation, pre-tool blocking, isolation은 unsupported
  - unsupported capability는 guarantee display를 낮추거나 CAPABILITY_INSUFFICIENT / structured blocked reason을 반환해야 함
  - required Harness record/check path 또는 required capability가 unavailable이면 product write가 조용히 진행되면 안 됨
```

Reference local surface는 active MVP path에 충분합니다. MCP, Core authority check, 한 번만 쓰는 협력형 Write Authorization, `record_run`, manual artifact, honest guarantee display를 통해 kernel을 검증하기 때문입니다. 이것은 broad connector platform이 아닙니다.

Reference-surface wording은 "이 task를 이 paths로 hold한다" 또는 "이 profile이 실행 전에 막을 수 있는지와 실행 뒤에만 감지할 수 있는 것을 보여준다" 같은 표현을 노출할 수 있습니다. `pre_tool_blocking_supported=false`이므로 hold는 cooperative scope discipline과, 가능할 때의 detective changed-path validation으로 설명합니다. Preventive guard로 설명하지 않습니다.

## Later Surface Notes

아래 named surface는 active MVP requirement가 아닙니다. Owner가 나중에 승격할 때 참고할 note입니다.

| Surface note | Later-only boundary |
|---|---|
| Codex | 향후 profile에서 `AGENTS.md`, skill, command, MCP snippet을 사용할 수 있습니다. Instruction text만으로는 cooperative입니다. Pre-tool blocking, native capture, isolation에는 exact profile proof가 필요합니다. |
| Claude Code | `PreToolUse` 같은 hook 개념은 promoted profile이 concrete host/version의 covered behavior를 증명한 뒤에만 사용할 수 있습니다. 그 전까지 guard/freeze wording은 cooperative 또는 detective입니다. |
| Gemini | Extension 또는 CLI-wrapper note는 나중에 사용할 수 있습니다. Extension wording만으로는 guard가 아니며, context는 compact하게 유지해야 합니다. |
| GitHub Copilot | IDE behavior와 cloud behavior를 흐리지 않습니다. Task wrapper 또는 cloud policy path에는 별도 promoted profile과 proof가 필요합니다. |
| Cursor | Project rule은 instruction context입니다. IDE permission 또는 sidecar behavior는 preventive claim 전에 증명해야 합니다. |

이 note들은 connector marketplace, hosted connector registry, broad connector ecosystem, cross-surface orchestration plan이 아닙니다. 나중에 profile이 이 중 하나를 승격하면 exact `capability_profile` field, fallback behavior, conformance expectation, 필요한 경우 redaction/secret handling, 그리고 connector output이 Core authority를 대체하지 않는다는 규칙을 정의해야 합니다.
