# Agent Integration Reference

## What this document helps you do

Use this reference to connect an agent surface to Harness without overstating what that surface can enforce.

It owns the common connector contract: capability tiers, capability profiles, generated manifest expectations, context push/pull principles, fallback semantics, Role Lens behavior, the reference surface contract, and connector conformance overview.

For the user-facing agent procedure, read [Agent Session Flow](../use/agent-session-flow.md). For surface-specific setup notes, read [Surface Cookbook](surface-cookbook.md).

This is reference documentation for future Harness behavior. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are implementing or reviewing a connector for an agent surface.
- You need to declare or audit a surface capability profile.
- You need to decide how a connected profile should display guarantee level, guard, freeze, fallback, or MCP availability.
- You are writing connector conformance coverage.
- You need to know which parts belong in a surface recipe instead of the common contract.

## Before you read

Read [Agent Session Flow](../use/agent-session-flow.md) for the user-facing procedure. For exact authority, consult only the relevant owner section when the current phase needs it: [Kernel Reference](kernel.md) for write and close authority, [MCP API And Schemas](mcp-api-and-schemas.md) for the current method contract, and [Security Threat Model Reference](security-threat-model.md) for MCP exposure, generated-file, stale-context, artifact, secret, and capability-overclaiming threats. This reference explains connector behavior and capability display, not kernel state transitions, and it is not an instruction to load all Reference docs by default.

## Main idea

A connector should keep the agent's context small and current, route state-changing actions through Harness, and describe only the guarantees its proven capability profile can actually provide. Cooperative or detective surfaces may hold or detect; only covered preventive paths with fixture-proven pre-tool blocking should be described as pre-execution blocking, and isolated paths should be described as separation rather than approval or verification.

## Integration in plain language

An agent surface is where the user talks to an agent. Harness is the local authority layer that keeps task state, write authority, evidence, verification, Manual QA, work acceptance, projections, and reconcile behavior outside the chat transcript.

A connector should give the agent small current context, route state changes through Harness MCP tools, capture what happened when the surface can do so, and name the actual guarantee level for the connected profile. Capability is concrete: it must be declared and proven for the actual host, target profile, version/configuration, workspace policy, capture path, and guard or isolation path in use. A surface name is never enough to claim a capability.

The common structure is:

```text
user conversation surface
  -> short always-on rules/context
  -> harness skill, command, or playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, or isolation layer
```

Always-on rules and context should stay short, current, and non-authoritative. The operational context budget is current Task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness. Static rules should say when to use Harness, where to read current status or current-position context, that Journey Card is used only when that projection/profile is enabled and fresh, that product writes require `prepare_write`, that user-owned judgment routes through Decision Packets, that clarification inspects repo/docs/current state before asking the user, that status must show what can actually be blocked and what can only be detected later, and that product writes hold when authoritative MCP is unavailable. They should not expand into schema dumps, full DDL, old task history, copied evidence bodies, full artifact contents, full projection bodies, future catalog material, or reference-contract replicas. The session procedure itself belongs in [Agent Session Flow](../use/agent-session-flow.md).

## What belongs in Use docs vs this Reference doc

| Area | Owner |
|---|---|
| What the agent shows, asks, and says during a user session | [Agent Session Flow](../use/agent-session-flow.md) |
| User-facing explanation of scope, evidence, verification, QA, residual risk, and close | [User Guide](../use/user-guide.md) |
| Common connector contract, capability profiles, manifests, context model, fallback semantics, Role Lens, reference surface, conformance overview | This reference |
| Concrete surface recipes for Codex, Claude Code, Gemini, GitHub Copilot, and Cursor | [Surface Cookbook](surface-cookbook.md) |
| Public MCP request/response schemas | [MCP API And Schemas](mcp-api-and-schemas.md) |
| Kernel state transitions and write/close rules | [Kernel Reference](kernel.md) |
| Guarantee level meanings and security control expectations | [Security Threat Model Reference](security-threat-model.md#honest-guarantee-display) |
| Runtime placement for guarantee display | [Runtime Architecture Reference](runtime-architecture.md#guarantee-levels) |
| Security assets, trust boundaries, threat categories, and control categories | [Security Threat Model Reference](security-threat-model.md) |

## Capability Tiers

| Tier | Meaning | Typical capability |
|---|---|---|
| `T0 Context` | Surface can read Harness principles. | rules/context file |
| `T1 Skill` | Surface can follow a Harness procedure. | skill, command, prompt, playbook |
| `T2 MCP` | Surface can call Harness tools and resources. | MCP server connection |
| `T3 Capture` | Surface can return diffs, logs, and run output reliably. | structured output, wrapper, adapter |
| `T4 Guard` | Surface can block or interrupt covered out-of-scope files, commands, network, or secrets before execution when fixture coverage proves that concrete path for the profile. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | Surface can run verification or risky work behind a documented separation boundary. Worktrees and fresh evaluator bundles may provide verification independence or stale-context control; sandboxing, permission isolation, locked-down runners, process boundaries, or container boundaries require exact profile proof. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | Surface can structure browser, screenshot, walkthrough, workflow-recording, or Manual QA artifacts. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

Normal interactive Harness use is most natural at `T2` or higher. Reliable detached verification usually needs `T3` capture plus a real independence boundary. High-risk work should use a fixture-proven `T4` guard or `T5` isolation when available. `T6` improves UI/UX evidence, but it does not replace Manual QA judgment, work acceptance, or detached verification, and it is not required by the Engineering Checkpoint/default reference posture or Assurance Profile / Operations Profile staged Manual QA coverage when human Manual QA notes and manually supplied artifacts can be recorded.

For Engineering Checkpoint and MVP-1, connectors should assume cooperative/detective behavior unless the concrete profile proves otherwise. `T4` and `T5` rows describe stronger future or profile-specific capabilities; they do not imply OS-level isolation, arbitrary-tool sandboxing, tamper-proof local files, or pre-tool blocking for the MVP-1 User Work Loop by default.

`T6 QA Capture` profiles must name supported capture types and fallback behavior. Candidate capture types include screenshot, console log, network trace, accessibility snapshot, and workflow recording. Captured files must follow redaction and secret/PII handling before durable storage and should be registered as artifact refs attached to the Manual QA record or feedback loop execution.

## Capability Profiles

Harness connectors must use a capability profile instead of assuming behavior from a product or surface name. A profile is scoped to the actual host/profile that will run the work, including the detected version, MCP configuration, hook/permission/workspace policy posture, capture method, QA capture method, redaction policy, artifact retention behavior, and conformance or operator-check basis that makes the declaration current. A different host, profile, version, configuration, permission model, capture path, or conformance result requires a refreshed profile before the connector can claim the same capability.

```yaml
surface_id: SURF-0001
surface_kind: generic_agent
target_profile: local_cli
detected_version: optional string
capability_profile_version: 1
last_verified_at: 2026-05-06T10:05:00+09:00
support_tier: T2
guarantee_level: cooperative
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  hooks: false
  pre_tool_guard: false
  explicit_permissions: false
  changed_path_detection: validator
  fresh_verify: manual_bundle
  worktree_isolation: false
  local_sidecar: false
  browser_qa_capture: false
  screenshot_capture: false
  console_log_capture: false
  network_trace_capture: false
  accessibility_snapshot_capture: false
  workflow_recording_capture: false
risks:
  - no pre-tool guard
fallbacks:
  - cooperative prepare_write discipline
  - changed_paths validator
  - manual verification bundle
  - human Manual QA notes and manually supplied QA artifacts
```

Target profile values may include:

- `local_cli`
- `ide_chat`
- `ide_agent`
- `cloud_agent`
- `extension`
- `custom_agent`
- `manual_bundle`

Every capability profile must state MCP exposure posture at a contract level. The exact field names are connector-owned, but the profile must make these facts visible:

- whether the Engineering Checkpoint baseline and staged-delivery `local_only` posture is in effect
- the assumed local transport, such as localhost TCP, local socket, in-process/stdio, process-scoped configuration material, or equivalent local IPC
- the access-control material class, such as bind scope, socket path class, process pipe/stdio, per-project token handle, process-scoped config handle, or equivalent local control, without raw token, secret, or private configuration values
- the access-control contract that keeps unrelated callers from using the endpoint
- whether remote or shared MCP exposure is disabled, unsupported, or explicitly enabled by the profile
- for any beyond-local exposure, the owner-doc and conformance-promotion basis, secret/PII handling policy, redaction or omission behavior, guarantee display, and conformance coverage that prove the exposure does not silently upgrade authority

The security reason for these fields is owned by [Security Threat Model Reference](security-threat-model.md); this reference owns how connector profiles report them.

Capability profiles must be refreshed when detected version, MCP config, hooks, permissions, workspace policy, generated files or managed blocks, conformance result, capture method, QA capture method, browser test environment, redaction policy, artifact retention behavior, access-control material class, local bind/reachability posture, or isolation/guard wrapper behavior changes. Beyond-local exposure remains outside the Engineering Checkpoint baseline and staged delivery until promoted by owner docs and conformance; connector prose must not present it as the safe Engineering Checkpoint or staged-delivery default.

## Capability Profile Examples

These are examples of profile shapes. A tier or example does not automatically upgrade a concrete surface's guarantee level. A concrete connector must prove the capability for its actual host/profile before claiming it.

### Cooperative MCP Profile

```yaml
surface_id: SURF-0001
surface_kind: generic_agent
target_profile: ide_chat
support_tier: T2
guarantee_level: cooperative
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  pre_tool_guard: false
  changed_path_detection: validator
  fresh_verify: manual_bundle
  worktree_isolation: false
fallbacks:
  - cooperative prepare_write
  - changed_paths validator
  - manual verify bundle
```

### Detective Capture Profile

```yaml
surface_id: SURF-0002
surface_kind: generic_agent
target_profile: local_cli
support_tier: T3
guarantee_level: detective
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: true
  artifact_capture: wrapper
  pre_tool_guard: false
  changed_path_detection: sidecar
  command_output_capture: wrapper
  fresh_verify: manual_bundle
  worktree_isolation: false
fallbacks:
  - sidecar changed-file watcher
  - artifact integrity check
  - fresh evaluator instructions
```

### Guarded Local Profile

```yaml
surface_id: SURF-0003
surface_kind: generic_agent
target_profile: local_cli
support_tier: T4
guarantee_level: preventive
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: true
  artifact_capture: wrapper
  hooks: true
  pre_tool_guard: true
  explicit_permissions: true
  changed_path_detection: sidecar
  command_output_capture: wrapper
  fresh_verify: fresh_session
  worktree_isolation: optional
fallbacks:
  - sidecar guard for fixture-proven covered operations
  - approval card
  - fresh evaluator profile
```

### Isolated Verification Profile

```yaml
surface_id: SURF-0004
surface_kind: manual_bundle
target_profile: manual_bundle
support_tier: T5
guarantee_level: isolated
capabilities:
  mcp_tools: false
  mcp_resources: false
  structured_output: true
  artifact_capture: bundle
  pre_tool_guard: read_only_bundle
  changed_path_detection: bundle_manifest
  fresh_verify: fresh_worktree
  worktree_isolation: true
fallbacks:
  - read-only evaluator bundle
  - operator record_eval
```

## Guarantee Levels

Integration uses the guarantee levels defined in [Security Threat Model Reference](security-threat-model.md#honest-guarantee-display) and applies them to connected surface profiles, current enforcement paths, and fallback choices.

This reference owns how connector profiles report and display those levels. It must not infer a stronger level from a surface name, product name, recipe name, or mode label, and it must not treat guarantee level as Approval, Write Authorization, verification, QA, work acceptance, residual-risk acceptance, close readiness, or a kernel gate.

The Engineering Checkpoint and MVP-1 User Work Loop should display the reference surface as cooperative/detective unless a fixture-proven guard or documented separation boundary is promoted and proven for the operation being described. Future preventive or isolated profiles may be documented, but they must stay labeled as future/profile-specific until owner docs and conformance promote them.

Stage display defaults mirror the [Security Threat Model stage map](security-threat-model.md#guarantee-levels-by-stage):

| Stage | Connector display default |
|---|---|
| Engineering Checkpoint | Show cooperative discipline and limited detective checks around `prepare_write`, Write Authorization, `record_run`, changed paths, and the minimal artifact/evidence ref. Do not imply default pre-tool blocking or isolation. |
| MVP-1 User Work Loop | Show user-visible blockers, MCP availability, close readiness, decision/evidence gaps, and whether the surface can only hold by instruction or detect later. |
| Assurance Profile | Show stronger separation of verification, Manual QA, waivers, residual risk, work acceptance, and stewardship findings, still as cooperative/detective unless a stronger profile is proven. |
| Operations Profile | Show operator diagnostics, generated-file drift, projection freshness, artifact integrity, recover/export posture, and honest guarantee limits as detective/reporting behavior unless exact coverage is proven. |
| Roadmap | Show preventive or isolated only for the named covered operation or separation boundary with owner-doc promotion and conformance proof. |

| Level | Display responsibility |
|---|---|
| `cooperative` | Show that the surface is expected to follow Harness decisions; holds are by instruction, and Harness does not claim physical blocking before execution. |
| `detective` | Show that Harness can observe changed paths, logs, artifacts, or projection drift after action and mark state stale, blocked, partial, or failed; display this as detection, not prevention. |
| `preventive` | Show the fixture-proven hook, wrapper, permission layer, policy engine, or sidecar path and the covered operations it can block before execution. |
| `isolated` | Show the documented separation boundary used for risky work or verification. A worktree or fresh evaluator bundle can provide scope, freshness, or blast-radius separation, but it is not automatically an OS sandbox, permission boundary, or tamper-proof security boundary unless the profile proves that exact isolation mechanism. Do not present isolation alone as approval, acceptance, verification, risk acceptance, close, or assurance upgrade. |

Guard, freeze, and careful-mode labels are safety-control labels over the actual profile, not authority tiers. Their display must say what can actually be blocked before execution and what can only be detected later.

| User wording | Actual boundary |
|---|---|
| Freeze | A visible scope hold or stricter next-action posture around current work. On cooperative profiles it is an instruction to hold. On detective profiles it may be paired with post-action validation. It is hard prevention only when fixture-proven pre-tool blocking covers the operation; persistent owner-record changes still route through the normal Core path. |
| Guard | Cooperative, detective, preventive, or isolated control posture according to the proven profile and current control path. Use preventive wording only for covered operations with fixture-proven pre-tool blocking. |
| Careful mode | Stricter `prepare_write`, scope, evidence, status refresh, and user-question posture. It is not a new authority tier, does not block by itself, and does not satisfy gates or decisions. |

## Generated Manifest Expectations

Connectors may generate rules, skills, MCP config snippets, prompts, or local adapter files. Every generated or managed path, managed block, MCP config snippet, and profile freshness marker must be recorded in a connector manifest.

The manifest must:

- name generated and managed paths, including MCP config snippets and local adapter files
- record managed block ids and hashes
- record the capability profile used when generated, including `capability_profile_version`, `detected_version`, `last_verified_at`, and the conformance result or operator check that made it current
- record the target surface profile and MCP tool/resource scope
- record the MCP exposure posture, access-control material class, bind/reachability posture, profile freshness basis, and display-safe handle or fingerprint when needed, without storing raw token, secret, private configuration values, omitted secret values, or blocked payload bytes
- record configured capture, QA capture, guard, and isolation mechanisms without claiming more than the profile proves
- record manual artifact capture and manual verification bundle fallbacks when native capture or isolation is unavailable
- record creation and update times
- mark the profile or generated block stale when the surface version, MCP config, hooks, permissions, workspace policy, wrapper, sidecar, generated file, managed file, conformance result, capture method, QA capture method, redaction policy, or artifact retention behavior changes
- detect drift before overwriting human edits
- keep the existing file or managed block unchanged when drift is detected unless an explicit reconcile or reconnect decision authorizes replacement
- route drift to reconcile when needed and report that the edited generated file is not canonical Task state

The manifest concept is common. Surface-specific generated filenames belong in [Surface Cookbook](surface-cookbook.md).

## Context Push/Pull Principles

For future connector design, implementation agents should receive a compact always-on Harness context envelope every turn and pull larger references only when needed. The envelope is current operational state, not agent memory, chat history, old projection text, or a complete reference dump. It should use ids, one-line summaries, and freshness markers. This is a design contract target; it is not evidence that this documentation repository already implements an agent context API.

The always-on operational contract should fit on one screen or less. It includes only:

- current Task summary, or explicit `none` / `unknown`
- work shape: read/advice work, small change, tracked work, or the schema-owned `advisor` / `direct` / `work` value when diagnostic detail is useful
- scope and non-goals
- pending user judgments
- active blockers
- next safe actions
- evidence gaps
- close blockers
- residual-risk summary
- guarantee level
- source refs and freshness

A field may be `none`, `unknown`, or a one-line ref summary when the item is absent or not relevant. The connector should not add broader role text, full phase history, schema detail, or procedure text to the always-on envelope by default; those remain connector setup or phase-specific pull context.

Always-on injection is prohibited from including full reference docs, full public schemas, full Storage DDL, the full historical event log, full projection bodies, full artifact contents, raw logs/screenshots/diffs/traces, unrelated templates, Future Fixture Catalog or other future-catalog content, old task history, or unrelated Roadmap material. Those stay pull-on-demand for the exact phase and next action that needs them.

Stale agent memory, stale chat history, remembered recommendations, and pulled context may point the agent toward refs to inspect, but they cannot authorize writes, satisfy gates, close tasks, accept results, waive QA or verification, accept residual risk, replace current owner records, or repair stale projections. Projections can summarize state and refs, but they are not authority. When state matters, the connector should retrieve current Core state or a compact context derived from current Core state; any older context that matters to authority must first be refreshed or reconciled through the owning Core path.

Keep the static always-on rule compass to these ten items or fewer:

1. Read current status or current-position context before significant work or resume; use Journey Card only when that projection/profile is enabled and fresh.
2. Product/runtime/code writes require compatible `prepare_write` and Write Authorization.
3. User-owned product or material technical judgment routes through Decision Packets.
4. Approval is not product judgment, work acceptance, or residual-risk acceptance.
5. Projection is readable output, not canonical state.
6. Evidence uses artifact refs and state refs, not pasted logs or copied evidence bodies as authority.
7. Same-session review is self-checking context, not detached verification.
8. MCP unavailable means no authoritative state update, gate update, evidence, work acceptance, residual-risk visibility, residual-risk acceptance, projection-repair, or close claim.
9. Show blockers and close-relevant residual risk before acceptance or close.
10. Pull Reference docs, schemas, historical records, later-profile resource bodies, and large artifacts only when the next action needs them.

Token savings must not starve the agent of user-owned judgments, blockers, scope limits, safety boundaries, evidence gaps, close blockers, or close-relevant residual-risk information needed for correct behavior. Judgment requests in particular must include enough context for informed user judgment: the decision, judgment category, route, display depth, concise options or chosen outcome, consequences, uncertainty, affected scope, relevant refs, what the agent is not deciding for the user, and what the answer does not settle. Higher-risk or close-affecting prompts additionally include recommendation, affected gates or acceptance criteria, consequences of deferral, and other route-specific context.

These are the compact current-state envelope fields. They are not a public schema and do not prove that a context API is implemented; they define the compactness target for future connector payloads and prompt-sized context.

Always-on envelope contract:

| Envelope item | Push shape |
|---|---|
| Current Task summary | Task id/title or explicit `none` / `unknown`; one-line current-position summary only. |
| Work shape | Read/advice work, small change, tracked work, or `advisor` / `direct` / `work` when diagnostic precision is useful. |
| Scope and non-goals | One-line in-scope and out-of-scope summary, with refs rather than full Change Unit or Autonomy Boundary bodies. |
| Pending user judgments | Judgment request or Decision Packet refs and one-line questions, or `none`. |
| Active blockers | Primary blocker and any follow-on blocker that changes the next action. |
| Next safe actions | The next action and smallest unblocker if blocked. |
| Evidence gaps | Missing, stale, blocked, or insufficient evidence summary tied to acceptance criteria or current claim. |
| Close blockers | Close blocker refs or explicit `none` when close/acceptance depends on that fact. |
| Residual-risk summary | Known close-relevant residual risk summary and refs, `none`, or `not_visible`. |
| Guarantee level | Actual connected profile level and the guard or detection behavior it can prove. Do not infer this from a surface name. |
| Source refs and freshness | Source state version, owner refs, projection/card freshness when used, connector profile freshness when guarantee is shown, and stale/unavailable warnings. |

Attach only source refs or one-line summaries when relevant; do not add their bodies to the always-on envelope:

- Write Authorization, sensitive-action permission refs, approval-shaped Decision Packet refs in minimum MVP-1, Approval refs only when the later Approval profile is active, evidence summary refs, Evidence Manifest refs only when that profile is active, Eval, Manual QA, work-acceptance, Run, report, artifact, and residual-risk refs
- relevant policy, TDD trace, stewardship, module/interface, and domain refs only when the active profile or current question needs them

Keep these refs-first and pull the body only when needed:

- Evidence, Run, Eval, and Manual QA records
- artifacts, logs, screenshots, diffs, workflow recordings, and large traces
- older PRDs, old designs, closed issues, stale docs, old projections, and moved-path notes
- module maps, interface contracts, domain language, coding standards, and TDD guidance

Refs-first means the connector should push stable ids, paths, hashes, summaries, outcomes, and freshness, not paste large bodies into the default prompt. Embed excerpts only when the next safe action requires inspecting the content, and keep the excerpt tied to its source ref. Retrieved, indexed, remembered, or summarized context follows the same rule: it can tell the agent what to inspect next, but it remains pull-only context until an owner path records an actual state change. It must not authorize writes or create Write Authorization, resolve Decision Packets, grant Approval, satisfy gates, create evidence, perform or record verification, record QA, waive QA or verification, accept results, accept residual risk, update projection freshness, or close tasks.

Use context profiles so agents do not load the whole documentation set. Each profile narrows both the current state envelope and the owner sections the connector may pull. MCP resource pulls follow the staged [Read-only resources](mcp-api-and-schemas.md#read-only-resources) map: Engineering Checkpoint uses only the current project/current task/status subset, MVP-1 adds decision-packet and judgment-context reads only when user judgment context is needed, and Assurance Profile/Operations Profile/future resources stay profile-gated or pull-on-demand. A connector may push refs, one-line summaries, and freshness markers from enabled resources, but it should not inject full resource outputs by default. The default exclusions apply to every profile unless that exact phase and next action require a specific section: full [Storage And DDL](storage-and-ddl.md) DDL, the full [Conformance Fixtures Reference](conformance-fixtures.md) or [Future Fixture Catalog](future-fixture-catalog.md), any future catalog, the full [Template Reference](templates/README.md) set, unrelated [Roadmap](../roadmap.md) items, old task history, historical event logs, full projection bodies, old projections, full artifact contents, raw logs/screenshots/diffs/traces, full MCP schemas, and full Reference documents.

| Profile | Minimal current state | Minimal docs or owner references | Do not load by default | User-visible summary | Authority and freshness |
|---|---|---|---|---|---|
| Session start | Current Task summary or explicit `none`/`unknown`, likely work shape, scope/non-goals when known, active blockers, pending user judgments, next safe action, evidence gaps, close blockers, residual-risk summary, guarantee level, and source/freshness refs. | [Agent Session Flow: Session start](../use/agent-session-flow.md#session-start), [Resume](../use/agent-session-flow.md#resume), current `harness.status` / `harness.status.next_actions` or optional `harness.next` output, and [Document Projection: Freshness and failure rules](document-projection.md#freshness-and-failure-rules) only when projection freshness affects the next action. | Full task history, full reference docs, full schemas, old projections, unrelated templates, unrelated Roadmap, future catalog. | Show the plain current position, primary blocker, pending user judgment, next safe action, and what the surface can block or only detect. | Read current Core status or state-derived current-position context when state matters. Use Journey Card only when that projection/profile is enabled and fresh. If Core/MCP is unavailable, say so instead of using memory as state. |
| Planning/clarification (Discovery) | Goal, user value when known, scope and non-goals, acceptance cues, answerable facts inspected from repo/docs/current state, missing information, blocking questions, useful non-blocking questions, assumptions, tracked uncertainty, user-owned judgment candidates, active blockers, and next allowed clarification action. | [User Guide: What the agent should answer first](../use/user-guide.md#what-the-agent-should-answer-first), [Agent Session Flow: Intake](../use/agent-session-flow.md#intake), [Scope and Write Boundary](../use/agent-session-flow.md#scope-and-write-boundary), and relevant current Task/Change Unit/Shared Design refs. Pull [Kernel: Judgment route boundaries](kernel.md#judgment-route-boundaries) only when a user-owned route must be separated. | Whole module maps, old PRDs/designs, design-policy catalogs, unrelated templates, full Storage DDL, full Conformance catalog, future catalog. | Show answerable facts, the next blocking question when any, useful non-blocking questions parked for later, recommendation, uncertainty, deferral effect, and what can safely continue. | Inspect current repo/docs/state refs before asking. Mark unavailable or stale sources and do not turn stale design prose, chat memory, or projections into authority. |
| Write preparation | Active Task and Change Unit, intended paths/tools/commands/network/secrets summary, scope and out-of-bounds areas, Autonomy Boundary, pending judgments or sensitive-action permission needs, later Approval needs only when that profile is active, baseline/state freshness, Write Authority Summary, guarantee/MCP availability. | [Agent Session Flow: Product writes](../use/agent-session-flow.md#product-writes), [Kernel: prepare_write](kernel.md#prepare_write), [Kernel: Write Authorization](kernel.md#write-authorization), and [`harness.prepare_write`](mcp-api-and-schemas.md#harnessprepare_write) for the relevant method only. Pull security owner sections only for the sensitive category at issue. | Full Kernel/reference docs, unrelated schemas, historical event logs, large diffs/logs, full Storage DDL, future catalog. | Show what would change, what is blocked, write-authority status, and the smallest unblocker. | Use current Core state and exact `prepare_write` inputs. Refresh or reconcile on stale baseline, stale projection, state conflict, changed intended paths, or changed sensitive category. |
| Execution/run recording | Consumed Write Authorization or no-write basis, changed-path summary, command/tool summary, run outcome, artifact refs with integrity/freshness, redaction/omission/block notes, and immediate next action. | [Agent Session Flow: Evidence and checks](../use/agent-session-flow.md#evidence-and-checks), [Kernel: record_run](kernel.md#record_run), [`harness.record_run`](mcp-api-and-schemas.md#harnessrecord_run), and [Document Projection: Artifact reference rendering](document-projection.md#artifact-reference-rendering) only for display or artifact-ref questions. | Full logs, raw diffs, screenshots, traces, bundles, artifact inventories, full projection bodies, full Template set, future catalog. | Show what ran, what changed, what was recorded, and what was redacted, omitted, blocked, stale, or unsupported. Keep raw artifact bodies pull-only. | Record from current run/artifact refs and current state. Do not present an audit or violation Run as satisfying evidence, QA, verification, work acceptance, or close. |
| Evidence review | Known evidence summary, `evidence_summary_ref` when present, Run refs, ArtifactRefs, visible evidence gaps, stale or insufficient support, redaction/integrity notes, affected acceptance criteria or claims, and next evidence action. Include an Evidence Manifest ref only when the full Evidence Manifest profile is active. | [Agent Session Flow: Evidence and checks](../use/agent-session-flow.md#evidence-and-checks), [`harness.record_run`](mcp-api-and-schemas.md#harnessrecord_run), artifact-ref display rules, and [Kernel: Evidence Manifest](kernel.md#evidence-manifest) only when that profile is active. | Full evidence bodies, full logs, raw diffs, screenshots, traces, bundles, artifact inventories, full projection bodies, full Template set, future catalog. | Show known coverage and gaps tied to criteria or claims, with refs and redaction/integrity notes. | Mark evidence stale when baseline, paths, sensitive-action permission / Approval, artifact integrity, or relevant owner records changed. A visible artifact ref is not evidence sufficiency by itself. |
| User judgment request | Exact user-owned judgment, judgment category, internal route, display depth, display-depth-appropriate options or chosen outcome, consequences, uncertainty, affected scope, relevant refs or explicit absence, what the agent is not deciding for the user, what the answer does not settle, and next action after the answer. Higher-risk or close-affecting prompts also show trade-offs, recommendation, affected gates/acceptance criteria, and consequence of deferral. | [Agent Session Flow: Blocking User-Owned Judgments](../use/agent-session-flow.md#blocking-user-owned-judgments), [Kernel: Decision Packet](kernel.md#decision-packet), [Decision Gate](kernel.md#decision-gate), and the [`harness.request_user_judgment`](mcp-api-and-schemas.md#harnessrequest_user_judgment) section only when exact API fields are needed. | Broad approval language, unrelated judgments, full evidence bodies, full logs, full schema references, full Template set, future catalog. | Show enough context for informed judgment: exact choice, options, consequences, uncertainty, scope, refs, what the agent is not deciding, and what the answer does not settle. | Source from a current Decision Packet or current state-derived judgment candidate. If a ref is stale or missing, label it before asking. Replay ids, routing metadata, and enum detail stay secondary unless they clarify the boundary. Token saving must not remove the context the user needs to decide. |
| Close readiness | Scope match, acceptance criteria, evidence coverage, verification status, Manual QA status, work-acceptance need/status, residual-risk visibility and accepted refs when relevant, close blockers, projection freshness, and smallest unblocker. | [Agent Session Flow: Close](../use/agent-session-flow.md#close), [Verification, Manual QA, Residual Risk, Work Acceptance](../use/agent-session-flow.md#verification-manual-qa-residual-risk-work-acceptance), [Kernel: close_task](kernel.md#close_task), [Gates](kernel.md#gates), [Waiver semantics](kernel.md#waiver-semantics), [`harness.close_task`](mcp-api-and-schemas.md#harnessclose_task), and [Document Projection: Freshness and failure rules](document-projection.md#freshness-and-failure-rules) when readable close context is used. | Generic all-done rollups, full report bodies, full historical logs, unrelated templates, full Conformance catalog, full projection bodies. | Show the close basis before acceptance or close: scope, user judgments, evidence, verification, Manual QA, residual risk, acceptance status, primary blocker, and smallest unblocker. | Read current Core gates, owner records, evidence/artifact refs, and projection freshness. A stale projection can summarize a blocker but cannot become authority. Exact `close_task` payloads stay internal unless a blocker needs exact detail. |
| Recovery/error | Primary error or blocker, owner, last safe/current state known, stale or unavailable source, affected authority claims, next recovery action, and whether writes/close must hold. | [Agent Session Flow: Resume](../use/agent-session-flow.md#resume), [Reading status and blockers](../use/agent-session-flow.md#reading-status-and-blockers), [Fallback Semantics](#fallback-semantics), [MCP API: Error taxonomy](mcp-api-and-schemas.md#error-taxonomy), [State conflict behavior](mcp-api-and-schemas.md#state-conflict-behavior), and [Document Projection: Freshness and failure rules](document-projection.md#freshness-and-failure-rules) when stale readable context is involved. Pull specific operations/recovery owner sections only when operator repair is the next action. | Historical event logs, stack traces, full artifacts, unrelated status, full Storage DDL, full Conformance catalog, unrelated Roadmap. | Show the plain blocker, owner, affected authority claim, smallest recovery step, and whether writes or close must hold. | Re-read Core when possible. If Core is unreachable, do not invent state from agent memory, chat history, cached projection, old status text, or operator prose. |

Requirements-clarification phrases such as "safe next-work candidate" and "work split" are context proposal/support phrases, not standalone schemas, canonical record types, gate values, projection kinds, or authority paths.

For user-facing mode display, connectors should lead with read/advice work, small change, or tracked work. These labels are derived display text, not schema fields, enum values, canonical record types, projection kinds, gate values, or authority paths. If an envelope or context bundle mentions a work-shape display label, it means the derived display label for the current schema mode, not a new API field unless a future schema owner explicitly defines one. The schema-owned values remain `advisor`, `direct`, and `work` for state, conformance, and API payloads. Display translation must not reduce product-write authority checks, user-owned judgment routing, sensitive-action Approval, evidence, QA, verification, work acceptance, residual-risk visibility, residual-risk acceptance, or close rules.

Context profiles are context discipline, not new schemas or gates. Moving from one phase to another changes what the connector pushes by default; it does not authorize writes, resolve decisions, create evidence, perform verification, accept risk, or close a Task.

The compact status card renders the envelope for "where are we and what happens next?" Judgment-context is separate. Use judgment-context only when user judgment is needed, and include the judgment question, judgment category, route, display depth, route-appropriate options or chosen outcome, consequences, uncertainty, relevant refs, what the agent is not deciding for the user, and recommendation or deferral effect when the active depth requires them, without turning the full evidence or artifact body into always-on context.

Status, next-action, recommendation, and recommended-playbook outputs are read-only guidance. They may recommend `prepare_write`, a Decision Packet, a Change Unit update, evidence collection, verification, QA, reconcile, or close attempt, but they do not mutate state, satisfy gates, authorize writes, create evidence, perform verification, record Manual QA, waive QA or verification, record work acceptance, record residual-risk acceptance, close Tasks, or upgrade assurance by themselves. State effects happen only when the recommended action later runs through the existing MCP/Core mutation path.

Evaluators should receive a tighter verification bundle: acceptance criteria, changed files, approval scope, relevant Decision Packets, residual risk summary, Autonomy Boundary, deferred decisions, codebase stewardship refs, evidence summary refs, Evidence Manifest refs only when that profile is active, required TDD trace refs, Manual QA requirement, artifact refs, freshness state, and forbidden patterns.

A later Context Index may help retrieve relevant projections, artifact refs, repo files, docs, or notes. Until owner docs promote it, it is a read-only context provider, not a connector authority path. Context Index remains a roadmap candidate; see [Roadmap: Candidate Inventory](../roadmap.md#candidate-inventory).

## Fallback Semantics

Fallbacks are described by guarantee level and risk, not by surface name.

| Fallback | Use when | Boundary |
|---|---|---|
| Cooperative | The surface can follow instructions but cannot enforce them. | Tell the agent to use `prepare_write`, hold on blocked decisions, and record runs. Product/runtime/code writes pause by instruction if authoritative MCP is unavailable or write scope cannot be checked. |
| Detective | Harness can observe changed files, logs, projection drift, or artifact gaps after action. | Validators may mark state stale, partial, blocked, or failed and require repair, reconcile, or fresh verification. |
| Preventive | A fixture-proven hook, permission layer, wrapper, policy engine, or sidecar can block before execution. | Claim only the operations that the fixture-proven blocking path actually covers. |
| Isolated | Risk requires separation. | Use the documented separation boundary named by the connector profile. Fresh sessions, fresh worktrees, and evaluator bundles can support verification independence or stale-context control; sandboxing, permission isolation, locked-down runners, process boundaries, or container boundaries are security-isolation claims only when the profile proves that exact mechanism. Do not treat separation as approval, acceptance, verification, risk acceptance, close, or assurance upgrade unless the relevant owner path records that result. |

If MCP is unavailable, the connector must not claim authoritative state updates. `MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE` are diagnostic conditions, not additional public `ErrorCode` values. `MCP_UNAVAILABLE` remains the stable public availability code.

`MCP_SERVER_UNAVAILABLE` means the tool call cannot reach Core, so no authoritative Core response is possible from that call path. A connector must not invent Core state, Write Authorization, gate status, evidence, work acceptance, residual-risk acceptance, or close readiness from chat memory, generated files, cached projections, old status/next recommendations, or operator prose while Core is unreachable. `SURFACE_MCP_UNAVAILABLE` means Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required tools. Product/runtime/code writes hold until MCP is reconnected or diagnosed. A non-runtime documentation-maintenance edit may proceed only when the repository phase and Authoring Guide explicitly put that docs batch in scope. Exact path allowlists and batch boundaries for docs work are normal maintainer editing controls, not Harness runtime override capabilities or capability profiles. Documentation-maintenance edits do not create or update Core state, Core authorization, Write Authorization, evidence, verification, QA, Acceptance, residual-risk acceptance, close readiness, projections, `task_events`, or runtime state transitions. Cooperative surfaces hold by instruction; detective surfaces may also report after-action mismatches; stronger profiles may block before execution only when a fixture-proven guard covers the operation or may claim isolation only when a documented separation boundary is actually in use and proven.

If MCP works but pre-tool guard is weak, low-risk direct work may proceed with cooperative `prepare_write` and detective changed-path validation. Medium/high-risk work must not rely on cooperative-only claims when the assessed threat/control path requires preventive or isolated controls. The [Security Threat Model](security-threat-model.md) names the security reason; connector profiles, operations, API, kernel, and conformance owners define the exact behavior.

If native capture is unavailable, the connector should fall back to manual artifact capture: named artifact refs for diffs, logs, screenshots, workflow notes, command output, or QA notes supplied by the user or operator. If native isolation or fresh evaluator support is unavailable, it should fall back to a manual verification bundle with acceptance criteria, changed files, relevant refs, artifact refs, freshness state, and forbidden patterns. These fallbacks are explicit evidence routes, not upgrades to preventive or isolated guarantee levels.

Projection staleness is reported separately from state. If `source_state_version` is older than the canonical state, unknown, or missing where it is expected, the connector should warn that readable projection context may be stale. A connector may continue from canonical state if it can read state directly, but actions that depend on Markdown projection should refresh or reconcile first and should not treat the stale projection as authority.

## Role Lens Behavior

Role Lens is a non-authoritative skill or playbook surface that helps the user steer the agent from a familiar review posture. Initial lenses are:

- `product-review`
- `eng-review`
- `design-review`
- `security-review`
- `qa-review`
- `release-handoff`

A connector may expose these as slash commands, buttons, prompt snippets, or recommended playbooks. The lens name selects a review posture; it does not select an authority path.

Role Lens output may surface or recommend routes for:

- a `DecisionPacketCandidate` or a route to an existing Decision Packet
- a validator finding candidate or suggested `ValidatorResult` route for an actual validator/check to emit
- an evidence requirement
- an Eval or verification need
- a Manual QA requirement
- a sensitive-action permission need, or a later Approval need when that profile is active
- a residual-risk candidate
- a Change Unit update recommendation when appropriate
- release handoff report input
- a recommended next playbook

These are display and routing outputs until an existing Core/MCP state-changing path records the underlying action. Role Lens output, like status/next recommendation output, must not introduce schemas or canonical records, mutate canonical state by itself, satisfy gates, authorize writes, grant Approval, satisfy a Decision Packet, create evidence, perform verification, record Manual QA, waive QA or verification, record work acceptance, record residual-risk acceptance, close a Task, or upgrade assurance. When a lens identifies work that needs a state change, the surface routes through the normal MCP tool and Core path.

Two-stage review display should keep the stages visibly separate:

| Stage | Question |
|---|---|
| Spec Compliance Review | Is the requested work complete under current Harness authority: acceptance criteria, Change Unit completion conditions, scope/write authority compatibility, Decision Packet compatibility, evidence coverage, and residual-risk visibility? |
| Code Quality / Stewardship Review | Is the implementation maintainable: domain language, module/interface boundary, vertical slice shape, feedback loop or TDD trace, codebase stewardship, context hygiene, and follow-up risk? |

Same-session review may be useful self-checking, but it is not detached verification and must not display `assurance_level=detached_verified`.

## AFK and Public Commitment Display

AFK, unattended, or "continue while I am away" instructions are connector display and posture concerns; they do not create new authority. A connector should keep AFK work inside the active Change Unit, active Autonomy Boundary, granted sensitive-action permission, and compatible `prepare_write` / Write Authorization before actual product writes. Minimum MVP-1 uses an approval-shaped Decision Packet for that permission; later Approval profiles may use Approval records.

The surface should stop and show the smallest unblocker before scope expansion, an Autonomy Boundary breach, a new sensitive action without compatible sensitive-action permission, residual-risk acceptance, work acceptance, QA or verification waiver, public API or module contract change, release/support promise, documentation promise that changes what readers may rely on, or another public commitment that requires user-owned product or material technical judgment.

Display the stop according to the capability profile. On cooperative profiles, the connector instructs the agent to hold. On detective profiles, it may also describe after-action validation that can detect and report mismatches. Preventive wording is allowed only for operations covered by fixture-proven pre-tool blocking. Isolated wording is allowed only when the work uses the documented separation boundary named and proven by the connector profile.

## Reference Surface Contract

Engineering Checkpoint uses only the reference-surface support needed to exercise one local project registration and the Core authority path. That path should demonstrate the kernel rather than broad ecosystem support. Later bullets in this section are profile targets, not Engineering Checkpoint requirements.

Engineering Checkpoint minimum reference expectations:

- `T2 MCP` available for the Engineering Checkpoint public tool/resource subset needed by the Engineering Checkpoint, including only the current project/current task/status resources needed for the first authority loop, not the full later-profile MCP surface documented in MCP API And Schemas
- local-only or otherwise owner-approved access posture for the registered project surface
- cooperative `prepare_write` before product writes and compatible Write Authorization before any product-write Run
- detective changed-path and artifact validation after runs
- no default OS sandbox, arbitrary-tool sandboxing, tamper-proof local files, or pre-tool blocking claim
- run summary and at least one manually supplied or captured artifact/evidence ref sufficient for the minimal authority loop
- actual pre-action stop versus after-action detection status when guard, freeze, or careful-mode labels are shown

Later profile expectations:

| Profile | Connector support target |
|---|---|
| MVP-1 User Work Loop | User-readable status/next cards, Decision Packet display, pending user judgment routing, evidence and close readiness summaries, work-acceptance separation, and residual-risk visibility when relevant. |
| Assurance Profile | Evidence Manifest support, manual verification bundle or fresh evaluator instructions, Manual QA note/artifact support, artifact integrity status for captured or manually supplied artifacts, and assurance/QA/waiver displays. |
| Operations Profile | Connector manifest for generated files, managed blocks, MCP config snippets, and profile freshness; projection freshness and reconcile flow; operator smoke for MCP availability, surface capability mismatch, generated-file drift, artifact integrity, artifact/capture fallback, stale context, evaluator bundle freshness, projection freshness, and security/threat-model categories named in [Operations And Conformance Reference](operations-and-conformance.md#doctor). |

Reference surface behavior details and surface-specific setup belong in [Surface Cookbook](surface-cookbook.md) only when they name a concrete surface.

## Connector Conformance Overview

Connector conformance should prove that a profile can uphold the common contract at its declared capability tier. The scenarios below are an aggregate profile map, not a single Engineering Checkpoint checklist.

Engineering Checkpoint connector checks:

- status with and without an active Task
- compact current-position status shown before significant work resumes when required by the Use procedure; persisted Journey Card output is a later/diagnostic profile
- basic Change Unit scope for the selected path/tool/command, without full vertical/horizontal exception policy
- Autonomy Boundary breach stops or reports a structured blocker; Decision Packet routing is later-profile unless that profile is enabled
- `prepare_write` allowed and structured-blocker paths
- Write Authorization created for allowed writes and exposed through Write Authority Summary
- write-capable `record_run` consumes a compatible Write Authorization
- `record_run` with a minimal artifact/evidence ref
- local-only MCP default, with off-profile remote or shared exposure held, failed, or reported as capability-insufficient
- MCP unavailable product-write hold
- status recommendations, and `next` recommendations when `harness.next` is included in the active profile, remain read-only guidance unless the recommended action follows the existing Core mutation path
- if a connected Engineering Checkpoint surface also exposes Role Lens or recommended-playbook output, that output is read-only guidance and does not become a Engineering Checkpoint requirement

Later profile scenarios:

- intake classification into `advisor`, `direct`, or `work`, with user-facing display rendered as read/advice work, small change, or tracked work when shown to users
- work shaping that checks answerable facts first, separates blocking questions from useful non-blocking questions, and routes user-owned judgments with options and consequences
- full Change Unit vertical/horizontal exception handling
- one blocking question at a time when possible, with recommendation, uncertainty, and deferral effect when available, without turning clarification into a full questionnaire
- Decision Packet shown instead of broad approval for blocking user-owned judgment
- public commitments route to Decision Packet or another existing owner path when they require user-owned product or material technical judgment
- AFK work remains covered by active Change Unit scope, Autonomy Boundary latitude, any granted sensitive-action permission that applies, and compatible `prepare_write` / Write Authorization before actual product writes, with stop wording matched to the proven guarantee level
- sensitive-action permission request, granted, denied, and expired paths; minimum MVP-1 uses approval-shaped Decision Packets, while later Approval profiles may use Approval records
- `record_run` with artifacts and evidence update
- `DIRECT-RESULT` projection
- verification launch or manual verification bundle
- same-session verification guard
- evaluator bundle freshness before detached verification
- Manual QA required, passed, failed, and waived
- QA waiver with product/user risk routes through Decision Packet
- acceptance required and recorded
- approval, QA, verification waiver, work acceptance, and residual-risk acceptance remain distinct judgments
- close-relevant residual risk visible before acceptance or successful close
- risk-accepted close additionally requires accepted Residual Risk refs
- stale projection and reconcile flow
- stale projection write hold/status
- generated file drift detection
- safe non-overwrite behavior for generated files and managed blocks, with drift routed to reconcile
- connector manifest profile freshness and stale capability profile detection
- profile refresh after version, MCP config, hook, permission, workspace policy, generated-file, conformance-result, capture-method, QA-capture-method, redaction-policy, or artifact-retention changes
- capability fallback when a required tier is missing
- surface capability mismatch holds unsafe writes and reports honest guarantee limits
- stale PRDs, stale chat memory, and other pull-only context do not authorize writes, satisfy gates, accept results, or close tasks until reconciled through owner paths
- artifact integrity mismatch keeps dependent evidence, verification, export, or close readiness claims stale, blocked, or insufficient

Exact fixture format is owned by [Conformance Fixtures Reference](conformance-fixtures.md), and operational commands are owned by [Operations And Conformance Reference](operations-and-conformance.md).
