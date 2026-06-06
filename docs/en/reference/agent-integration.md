# Agent Integration Reference

Use this reference when connecting an agent surface to future Harness behavior with low context cost, honest guarantee display, and preserved user-owned judgment. This repository is still documentation-only and in documentation review; this file describes planned Harness behavior and does not imply that a runtime server or connector implementation exists.

For what the agent says in a user session, read [Agent Guide](../use/agent-guide.md). For exact Core, API, schema, storage, projection, security, conformance, and operations contracts, pull only the owner section needed for the next action.

## 1. Owns / Does Not Own

This reference owns:

- agent surface capability profiles
- guarantee-level display for connected surfaces
- context push/pull rules and always-on context budget
- phase-relevant context profiles for cheap retrieval
- user judgment request behavior at the connector boundary
- Role Lens non-authority behavior when a surface uses a lens
- fallback behavior when Core, MCP, projections, or capabilities are unavailable
- compact surface recipes that help an agent choose context
- the connector conformance boundary

This reference does not own:

- user-facing session procedure; see [Agent Guide](../use/agent-guide.md)
- user-facing explanation of scope, evidence, QA, final acceptance, residual risk, and close; see [User Guide](../use/user-guide.md)
- Core state transitions, gates, `prepare_write`, Write Authorization, `record_run`, or `close_task`; see [Core Model Reference](core-model.md)
- public MCP method contracts, schemas, or public errors; see [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md)
- Storage DDL, persisted state, and artifact layout; see [Storage](storage.md)
- projection/template authority and active rendered template bodies; see [Projection And Templates Reference](projection-and-templates.md)
- threat model and guarantee-level meanings; see [Security Reference](security.md)
- future fixture shape or assertion authority; see [Conformance Reference](conformance.md)
- operator commands and diagnostics as active Reference scope; future candidates stay in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates)
- future connector marketplaces, hosted-agent assumptions, broad connector ecosystems, or cross-surface orchestration

Surface recipes in this document are integration guidance. They do not create Core state authority, write authority, evidence, verification, QA, sensitive-action approval, final acceptance, residual-risk acceptance, close readiness, or any new security boundary.

<a id="role-lens-behavior"></a>

Role Lens behavior, when present, is read-only posture guidance. A lens may recommend a judgment request, evidence collection, verification, Manual QA, sensitive-action permission, residual-risk handling, scope update, or next playbook, but the recommendation has no authority until an owner API/Core path records the underlying action.

<a id="capability-profiles"></a>

## 2. Capability Profile

Surface name is not capability. A connector must use a `capability_profile` scoped to the actual host, version/configuration, workspace policy, MCP posture, capture path, guard path, and separation boundary in use.

A `capability_profile` is not write authority and does not bypass active Task scope, active Change Unit scope, `prepare_write`, single-use cooperative Write Authorization, `record_run`, or Core close rules. Capability affects blocked reasons, fallback behavior, validator results, and guarantee display. `allowed` and `blocked` are Harness compatibility outcomes unless a proven preventive profile names the covered operation.

The active reference profile is intentionally small:

```yaml
capability_profile:
  surface_id: reference-local-mcp
  surface_name: Reference local MCP surface
  mcp_available: true
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
```

Exact public tool and resource contracts belong to the API owners. The connector may summarize the available subset, but it should not duplicate full method schemas in prompt context.

Refresh the profile when the surface version, MCP configuration, hooks, permissions, workspace policy, generated files, managed blocks, capture path, QA capture path, redaction policy, artifact retention, local access posture, guard wrapper, isolation wrapper, or conformance basis changes.

<a id="generated-manifest-expectations"></a>

Generated rules, skills, MCP snippets, adapter files, and managed blocks need a connector manifest. The manifest records generated paths, managed block ids and hashes, MCP exposure posture, display-safe handles, profile freshness, drift, and fallback behavior. It must not store raw tokens, secrets, private config values, blocked payload bytes, or canonical Task state.

## 3. Guarantee Levels

Guarantee level display follows [Security Reference](security.md#honest-guarantee-display). This reference owns how a connector maps a `capability_profile` to what the user sees.

| Level | Connector display rule |
|---|---|
| `cooperative` | Say the surface is expected to follow Harness instructions. Holds are by instruction, not physical blocking. |
| `detective` | Say Harness can observe supported after-action facts such as changed paths, logs, artifacts, or drift and then mark state stale, partial, blocked, or failed. |
| `preventive` | Name the fixture-proven hook, wrapper, permission layer, policy engine, or sidecar path and the exact operations it can block before execution. |
| `isolated` | Name the documented separation boundary. Do not imply OS sandboxing, permission isolation, or tamper-proof storage unless that exact mechanism is proven. |

The reference local MCP profile can display `cooperative` behavior and limited `detective` behavior only where changed-path or artifact-gap observation supports it. Because `pre_tool_blocking_supported=false` and `isolation_supported=false`, it must not claim `preventive` or `isolated` behavior.

Guard, freeze, and careful-mode labels are display labels over the actual profile. They must say what can actually stop before execution and what can only be detected later. They are not sensitive-action approval, verification, QA, final acceptance, residual-risk acceptance, close readiness, or a Core gate.

Do not claim current MVP security guarantees beyond the profile and owner docs. Harness does not provide default OS permissions, arbitrary-tool sandboxing, tamper-proof local files, pre-tool blocking, or security isolation.

<a id="context-pushpull-principles"></a>

## 4. Context Push/Pull

Connectors should push compact current context and pull larger owner docs only when the next action needs them. A context packet is operational support for the next agent action. It is not agent memory, chat history, a full report, a cached projection body, or a complete Reference dump.

Retrieval-cost rules:

- Do not inject the full Reference set by default.
- Do not inject full schemas by default.
- Do not inject full Storage DDL, full templates, full projection bodies, complete histories, full event logs, raw artifact contents, raw logs, raw screenshots, raw traces, or unrelated Roadmap material by default.
- Do not inject future/later catalog material by default.
- Pull the owner section needed for the next action, then stop.
- Choose one language for a normal work prompt. Do not load English and Korean paired docs for the same `doc_id` into one prompt; bilingual review should compare targeted sections rather than pushing both full paired documents.

Status cards, projections, rendered templates, retrieved context, recommendations, and chat memory are read-only. They can point the agent to owner refs, but they cannot authorize writes, satisfy gates, create evidence, resolve user judgments, grant sensitive-action approval, perform verification, record QA, accept the result, accept residual risk, repair projection freshness, or close a Task.

Token savings must not hide user-owned judgments, scope limits, blockers, safety boundaries, evidence gaps, close blockers, or close-relevant residual risk.

## 5. Always-On Context Budget

Always-on context should fit on one screen or less. Include only current, actionable state:

- current Task summary, or explicit `none` / `unknown`
- work shape
- scope and non-goals
- pending user judgments
- active blockers
- next safe actions
- evidence gaps
- close blockers
- residual-risk summary
- guarantee level, or the unavailable/capability condition when Core or required MCP cannot answer
- source refs and freshness

Do not put full reference material, full schemas, full DDL, full projection text, complete artifact bodies, unrelated templates, future catalogs, stale or unrelated task history, or full logs in always-on context.

## 6. Phase-Relevant Context Profiles

Use the narrowest profile that answers the next question.

| Phase | Pull only this |
|---|---|
| Session start / resume | Current `harness.status`, current task/status resources, and [Agent Guide: Report status for the user's next decision](../use/agent-guide.md#8-report-status-for-the-users-next-decision). |
| Planning / clarification | Current repo/docs/state refs and [Agent Guide: Clarify without endless planning loops](../use/agent-guide.md#4-clarify-without-endless-planning-loops). |
| Write preparation | Current scope/state, [Agent Guide: Check scope before product writes](../use/agent-guide.md#6-check-scope-before-product-writes), and only the `prepare_write` owner sections needed for the intended write. |
| Execution / run recording | Current write authorization, run/evidence refs, and [Agent Guide: Record evidence after meaningful action](../use/agent-guide.md#7-record-evidence-after-meaningful-action). |
| Evidence review | Current evidence refs, artifact refs, freshness facts, missing evidence, and the exact evidence or projection owner section only when needed. |
| Close readiness | Current owner records, blockers, residual-risk summary, and [Agent Guide: Close work honestly](../use/agent-guide.md#10-close-work-honestly). |
| User judgment request | Current judgment refs or candidates, consequences, uncertainty, and [Agent Guide: Request user judgment narrowly](../use/agent-guide.md#5-request-user-judgment-narrowly). |
| Recovery / error | Current availability/freshness state, [Fallback Behavior](#8-fallback-behavior), and the specific error owner section. |

If the action needs a strict contract, link or retrieve the owner section. Do not paste full owner docs into ordinary prompts.

## 7. Judgment Request Behavior

Agents preserve user-owned judgment. A connector may help format the request, collect the response, and route the record through the owner API path, but it must not decide for the user.

A judgment request should preserve:

- the decision the user is being asked to make
- the available options
- consequences and trade-offs
- uncertainty or missing evidence
- the agent recommendation, if any
- what the agent is not deciding for the user
- whether the prompt is compact (`presentation=short`) or a full Decision Packet style presentation (`presentation=full`)

Agents must not decide final acceptance, sensitive-action approval, QA waiver, verification risk acceptance, or residual-risk acceptance for the user. They also must not silently make user-owned product decisions, material technical decisions, or scope-expansion decisions. A broad "looks good" or "continue" message does not substitute for any required judgment path.

Judgment records are separate from evidence, verification, Manual QA, final acceptance, residual risk, and close readiness. None of those records substitutes for another.

## 8. Fallback Behavior

Fallbacks are described by guarantee level and risk, not by surface brand.

| Fallback | Use when | Boundary |
|---|---|---|
| Cooperative | The surface can follow instructions but cannot enforce them. | Hold product writes by instruction when authoritative MCP or write-scope checks are unavailable. |
| Detective | Harness can observe supported facts after action. | Mark state stale, partial, blocked, or failed and require repair, reconcile, or fresh evidence. |
| Capability insufficient | A requested write, capture, guard, isolation, or guarantee depends on an unsupported field. | Return `CAPABILITY_INSUFFICIENT` or a structured blocked reason; lower the displayed guarantee. |
| MCP unavailable | The surface or call path cannot reach authoritative Core. | Use stable public `MCP_UNAVAILABLE` behavior and do not claim state mutation. |
| Local access mismatch | The caller or transport is outside the registered local profile. | Use `LOCAL_ACCESS_MISMATCH` with display-safe diagnostics; do not introduce a surface-specific `UNAUTHORIZED` code. |

`MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE` are diagnostic conditions. `MCP_UNAVAILABLE` remains the stable public availability code.

While Core is unreachable, do not invent Core state, Write Authorization, gate status, approvals, evidence, final acceptance, residual-risk acceptance, projection repair, or close readiness from chat memory, generated files, cached projections, stale status text, or operator prose.

Projection staleness is separate from Core state. If the connector can read current Core state directly, it may continue from that state. Actions that depend on stale projections must refresh or reconcile first.

Documentation-maintenance edits in this repository are governed by [Authoring Guide](../maintain/authoring-guide.md), not by runtime Harness procedures. They do not create runtime state, Write Authorization, evidence, QA, acceptance, residual-risk acceptance, close readiness, projections, `task_events`, or runtime transitions.

## 9. Surface Recipes

Surface recipes are compact integration notes that help an agent decide what context to include. They are not separate reference owners and must not grow into long surface-specific workflows.

Keep a recipe to:

- the target `capability_profile`
- generated or managed instruction/config paths, if any
- MCP posture and display-safe handles
- surface-specific capability differences that require profile refresh
- capture, guard, or isolation facts proven by that profile
- fallback behavior when a required capability is missing
- conformance status for that profile

Do not include generic Core rules, public API schemas, full Reference docs, future connector ambitions, hosted-agent assumptions, audit notes, unrelated Roadmap items, full projection bodies, or long setup tutorials.

<a id="reference-local-surface"></a>

Reference local MCP recipe:

```yaml
surface_kind: reference_local_mcp
target_profile: reference-local-mcp
mcp_posture: local-only registered project, or owner-approved alternative
context_strategy: compact always-on context plus phase-relevant owner pulls
write_behavior: cooperative prepare_write discipline before product writes
run_behavior: record_run with summary and owner-registered artifact refs
capture_boundary:
  native_capture: unsupported in the minimum reference profile
  fallback_capture: manual artifact attachment
guarantee_boundary:
  default_level: cooperative
  max_level: detective only for supported after-action observation
  can_block_before_execution: false
  isolation_supported: false
fallbacks:
  - hold product writes by instruction when MCP/Core is unavailable
  - lower claims or return CAPABILITY_INSUFFICIENT for unsupported capabilities
conformance_smoke_status: planned_not_run
```

Because `pre_tool_blocking_supported=false`, "hold" language means cooperative scope discipline plus detective changed-path validation when available. It does not mean preventive guard behavior.

<a id="connector-conformance-overview"></a>

## 10. Connector Conformance Boundary

Connector conformance proves that a declared profile can uphold this common contract at its stated capability level. It does not prove a broad connector ecosystem, hosted registry, remote/shared MCP exposure, cross-surface orchestration, implementation readiness, runtime conformance for this documentation repository, or final documentation acceptance.

The active smoke target is the reference `capability_profile`, not a connector marketplace. Until runtime fixtures exist and run, `conformance_smoke_status` must remain `planned_not_run`.

Reference-surface checks include:

- status with and without an active Task
- compact current-position status before significant resume when the Use procedure requires it
- guarantee display derived from actual profile fields
- no `preventive` or `isolated` claim when the profile cannot prove it
- `prepare_write` allowed/blocked compatibility outcomes without OS-permission wording
- single-use cooperative Write Authorization only after `prepare_write.decision=allowed`
- `record_run` with summary and owner-registered artifact refs
- MCP-unavailable product-write hold
- `CAPABILITY_INSUFFICIENT` or an equivalent blocked reason for unsupported capabilities
- read-only recommendations unless a later Core mutation path records the action

Future fixture shape and assertion authority are owned by [Conformance Reference](conformance.md). Operational commands and diagnostics are later candidates in [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates) until a future owner promotes them.
