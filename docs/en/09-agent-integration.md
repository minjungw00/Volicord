# Agent Integration

## Document Role

This document owns the common integration contract for connecting an agent surface to the harness. It defines capability tiers, capability profiles, generated manifest expectations, context push/pull principles, fallback semantics, role-lens behavior, the reference surface contract, and connector conformance overview.

The main body is product-name-neutral. Surface-specific recipes live in [Appendix B](appendix/B-surface-cookbook.md).

This document does not define kernel state transitions, MCP request/response schemas, SQLite DDL, a capability gate, operational fixture details, or surface-specific cookbooks.

## Integration Goal

The integration goal is that a user can talk naturally with an agent while the harness supplies bounded work, state recording, evidence, verification, Manual QA, acceptance, projection, and reconcile flow behind the scenes.

```mermaid
flowchart TB
  Conversation["natural conversation"] --> Bounded["bounded work"]
  Bounded --> Harness["harness-supported flows"]
  Harness --> State["state recording"]
  Harness --> Evidence["evidence"]
  Harness --> Verification["verification"]
  Harness --> ManualQA["Manual QA"]
  Harness --> Acceptance["acceptance"]
  Harness --> Projection["projection"]
  Harness --> Reconcile["reconcile"]
```

An integrated surface should help the agent:

- start with status or intake
- classify advisor, direct, or work mode
- shape work into scoped Change Units
- shape and update the Autonomy Boundary for what the agent may do without another user decision
- check design-quality policies when they apply
- call MCP tools for state changes
- respect `prepare_write` and returned Write Authorization before product writes
- show the Write Authority Summary separately from Autonomy Boundary
- express guard, freeze, and careful-mode requests as capability-scoped safety controls, not as optimistic authority claims
- request or show Decision Packets for blocking product judgment
- offer role-based review lenses when they help steer the agent without creating authority
- record runs, artifacts, evidence, user decisions, QA, and acceptance
- distinguish approval, product decision, QA waiver, verification waiver, residual-risk acceptance, and final acceptance
- make known close-relevant residual risk visible before any successful close
- launch or package detached verification
- refresh or reconcile projections

## Common Integration Structure

```text
user conversation surface
  -> short always-on rules/context
  -> harness skill, command, or playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, or isolation layer
```

```mermaid
flowchart LR
  Surface["user surface"] --> Rules["rules / skill / playbook"]
  Rules --> MCP["harness MCP"]
  MCP --> Core["harness Core"]
  Core --> Layer["adapter / hook / sidecar / validator / isolation"]
  Layer --> Facts["observable facts or stronger enforcement"]
```

### Always-On Rules

Always-on rules should be short. They should tell the agent when to use the harness, where to read status or the Journey Card, and that product writes require `prepare_write`.

```mermaid
flowchart TD
  Start["agent receives work"] --> Resume{"significant work resumes?"}
  Resume -- yes --> Card["show status or Journey Card"]
  Resume -- no --> Classify["classify advisor / direct / work"]
  Card --> Classify
  Classify --> Judgment{"blocking product judgment?"}
  Judgment -- yes --> Packet["show or request Decision Packet"]
  Judgment -- no --> Write{"product write intended?"}
  Packet --> Resolved{"compatible user judgment resolved for affected operation?"}
  Resolved -- no --> HoldJudgment["hold or ask one blocking question"]
  Resolved -- yes --> Write
  Write -- no --> Proceed["continue within current contract"]
  Write -- yes --> MCPAvailable{"authoritative MCP available?"}
  MCPAvailable -- no --> Hold["hold product writes"]
  MCPAvailable -- yes --> Prepare["call prepare_write and show Write Authority Summary"]
  Prepare --> Allowed{"compatible Write Authorization?"}
  Allowed -- yes --> ProductWrite["write within authorized scope"]
  Allowed -- no --> Hold
```

Always-on rules should also preserve user agency:

- show the current Journey Card before significant work resumes
- do not ask for broad approval when a Decision Packet is required
- ask one blocking question at a time, with a recommendation and uncertainty when available
- allow AFK implementation only when active Change Unit scope, Autonomy Boundary latitude, any granted sensitive approval, and compatible `prepare_write` / Write Authorization before actual product writes all apply
- treat the Autonomy Boundary as judgment latitude, not write authority
- show the Write Authority Summary when work is about to write
- hold product writes if authoritative MCP is unavailable
- keep planning direction, product trade-offs, QA waiver, verification risk acceptance, and final acceptance human-held

Write Authority Summary displays the current write boundary from the active scoped Change Unit's scope, `prepare_write`, approval, allowed paths/tools/commands/network/secrets, and compatible Decision Packet refs that remove product-judgment blockers. Decision Packets do not authorize writes by themselves. The Autonomy Boundary only says what judgment the agent may exercise without another user decision.

They should not contain full state transition tables, MCP schemas, full templates, long design playbooks, or all historical project context.

### Skill Or Playbook Layer

The skill/playbook layer teaches procedure:

- when to call status, intake, and next
- how to use `recommended_playbooks` from status/next as optional stage-router guidance
- how to classify advisor/direct/work
- how to ask shaping questions
- how to form a Change Unit
- how to shape or update the Autonomy Boundary
- how to request or show Decision Packets for blocking product judgment
- how to show the Write Authority Summary before writes and record the compatible Write Authorization with the run
- how to record user decisions
- how to distinguish approval, product decision, QA waiver, verification waiver, residual-risk acceptance, and final acceptance
- how to record TDD trace, evidence, Manual QA, and acceptance
- how to run the two review stages: Spec Compliance Review first, then Code Quality / Stewardship Review
- how to expose Role Lens commands or prompts as non-authoritative playbook guidance
- how to make known close-relevant residual risk visible before any successful close, require accepted Residual Risk refs for risk-accepted close, and record acceptance only after close-relevant residual risk is visible
- why work verification must be detached
- how to handle stale projection and reconcile

Stage routing may use recommended playbooks such as shared-design, product-review, eng-review, design-review, security-review, tdd-loop, spec-review, code-quality-review, qa-review, guard-check, release-handoff, or browser-qa-candidate. These recommendations live inside the skill/playbook layer. They are display guidance only: they do not mutate state, authorize writes, satisfy gates, create evidence, verify work, waive QA, accept risk, or close a Task. If a recommended playbook proposes product judgment, the surface should route to an existing Decision Packet or the normal Decision Packet request path.

#### Role Lens

Role Lens is a non-authoritative skill or playbook surface that helps the user steer the agent from a familiar review posture. Initial lenses are:

- `product-review`
- `eng-review`
- `design-review`
- `security-review`
- `qa-review`
- `release-handoff`

A connector may expose these as slash commands, buttons, prompt snippets, or recommended playbooks. The lens name selects a review posture; it does not select an authority path. Role Lens output should reuse existing display and routing shapes rather than inventing parallel records. It may produce:

- a `DecisionPacketCandidate` or a route to an existing Decision Packet
- a validator finding candidate or suggested `ValidatorResult` route for an actual validator/check to emit
- an evidence requirement
- a Manual QA requirement
- a residual-risk candidate
- release handoff report input
- a recommended next playbook

Role Lens output must not mutate canonical state by itself, authorize writes, grant approval, satisfy a Decision Packet, waive QA or verification, accept residual risk, accept the result, close a Task, or upgrade assurance. When the lens identifies work that needs state change, the surface routes through the normal MCP tool and Core path: request a Decision Packet, record evidence, record Manual QA, launch or record verification, request acceptance, or close only when the relevant gates and blockers permit it.

`recommended_playbooks` is the normal status/next integration point for Role Lens suggestions. For example, status may recommend `security-review` when sensitive authentication scope is active, `design-review` when UI/UX or visual policy is relevant, `qa-review` when Manual QA is likely, or `release-handoff` near close. These recommendations remain display guidance even when they are useful and well named.

Two-stage review procedure should keep the stages visibly separate:

1. Spec Compliance Review checks whether the requested work is complete under current Harness authority: acceptance criteria, Change Unit completion conditions, scope/write authority compatibility, Decision Packet compatibility, evidence coverage, and residual-risk visibility.
2. Code Quality / Stewardship Review checks whether the implementation is maintainable: domain language, module/interface boundary, vertical slice shape, feedback loop or TDD trace, codebase stewardship, context hygiene, and follow-up risk.

Findings from either stage should route to validator results, evidence gaps, Decision Packet candidates, Change Unit update recommendations, residual-risk candidates, or close blockers. Same-session review may be useful self-checking, but it is not detached verification and must not display `assurance_level=detached_verified`; detached verification still needs a valid independence boundary and Eval path.

Core and validators enforce policy. The skill is guidance, not authority.

### MCP Layer

MCP is the preferred state boundary. Public tool names and schemas are owned by the MCP API document. Integration docs may reference tool intent, but connectors must use the schemas from `05-mcp-api-and-schemas.md`.

### Adapter, Hook, Sidecar, Validator, Isolation

Adapters and sidecars translate surface behavior into observable facts or stronger enforcement:

- artifact capture
- command output capture
- changed-path detection
- generated file drift detection
- projection freshness detection
- approval and scope guard support
- same-session verification guard support
- evaluator read-only or fresh-context support
- Manual QA capture support

These layers can improve guarantee level, but they do not create a kernel capability gate.

## Capability Tiers

| Tier | Meaning | Typical capability |
|---|---|---|
| `T0 Context` | Surface can read harness principles | rules/context file |
| `T1 Skill` | Surface can follow a harness procedure | skill, command, prompt, playbook |
| `T2 MCP` | Surface can call harness tools and resources | MCP server connection |
| `T3 Capture` | Surface can return diffs, logs, and run output reliably | structured output, wrapper, adapter |
| `T4 Guard` | Surface can block or interrupt out-of-scope files, commands, network, or secrets before execution | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | Surface can run verification or risky work in a separate boundary | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | Surface can structure browser, screenshot, walkthrough, workflow-recording, or Manual QA artifacts | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

Normal interactive harness use is most natural at `T2` or higher. Reliable detached verification usually needs `T3` capture plus a real independence boundary. High-risk work should use `T4` guard or `T5` isolation when available. `T6` improves UI/UX evidence but is not required for MVP when a human QA note can be recorded.

```mermaid
flowchart TB
  T0["T0 Context"] --> T1["T1 Skill"]
  T1 --> T2["T2 MCP"]
  T2 --> T3["T3 Capture"]
  T3 --> T4["T4 Guard"]
  T4 --> T5["T5 Isolation"]
  T5 --> T6["T6 QA Capture"]
```

### T6 QA Capture Semantics

`T6 QA Capture` describes the connected surface's ability to structure browser or workflow QA artifacts. It is useful for Manual QA profiles such as `browser_smoke`, `workflow`, `ui_quality`, and `accessibility`, but it is not a kernel gate and is not required for MVP.

A profile that claims browser QA capture should name the supported capture types and fallback behavior. Candidate capture types include screenshot, console log, network trace, accessibility snapshot, and workflow recording. Captured files must follow redaction and secret/PII handling before durable storage and should be registered as artifact refs attached to the Manual QA record or Feedback Loop execution, commonly with artifact kinds such as `qa_capture`, `screenshot`, `log`, or `other` according to the API artifact schema.

Browser QA Capture does not replace Manual QA judgment when human taste, experience quality, copy, accessibility interpretation, or visual review is required. It also does not replace detached verification unless the Eval path independently satisfies verification independence requirements. If a surface cannot capture a browser workflow, the connector should fall back to human Manual QA notes and manually supplied artifacts.

## Capability Profile

Harness connectors must use a capability profile rather than assuming behavior from a product or surface name.

```mermaid
flowchart TB
  Profile["Capability Profile"]
  Profile --> Identity["surface_id, surface_kind, target_profile"]
  Profile --> Support["support_tier"]
  Profile --> Guarantee["guarantee_level"]
  Profile --> Caps["capabilities"]
  Profile --> Risks["risks"]
  Profile --> Fallbacks["fallbacks"]
  Profile --> Verified["last_verified_at"]
  Caps --> CapabilityCheck["surface_capability_check"]
  Guarantee --> Display["guarantee display"]
```

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

Capability profiles must be refreshed when version, MCP config, hooks, permissions, workspace policy, generated files, conformance result, capture method, QA capture method, browser test environment, redaction policy, or artifact retention behavior changes.

## Guarantee Levels

Integration uses the guarantee levels defined in [04-runtime-architecture.md](04-runtime-architecture.md#guarantee-levels) and applies them to connected surface profiles, current enforcement paths, and fallback choices.

This document owns how connector profiles report and display those levels. It must not infer a stronger level from a surface name, and it must not treat guarantee level as approval, verification, QA, acceptance, or a kernel gate.

## Guarantee Display Requirements

Every status or `prepare_write` result that relies on surface behavior should expose the actual guarantee level. Display the level as a property of the connected profile and current enforcement path, not as a promise inferred from a surface name.

```mermaid
flowchart TD
  Result["status or prepare_write result"] --> Profile["connected capability profile"]
  Result --> Enforcement["current enforcement path"]
  Profile --> Level{"actual guarantee_level"}
  Enforcement --> Level
  Level --> Cooperative["cooperative: show instruction-following limitation"]
  Level --> Detective["detective: show after-action detection limitation"]
  Level --> Preventive["preventive: show blocking path"]
  Level --> Isolated["isolated: show separation boundary"]
  Level --> Rule["not approval, verification, QA, acceptance, or a kernel gate"]
  SurfaceName["surface name"] --> SurfaceRule["must not infer guarantee_level from name"]
```

User-visible examples:

| Level | Example display text |
|---|---|
| `cooperative` | "This surface is expected to follow Harness decisions, but Harness may not physically block an out-of-scope write before it happens. Changed-path validation can detect violations afterward." |
| `detective` | "Harness can observe changed paths or artifacts after action and mark scope/evidence/projection stale or blocked." |
| `preventive` | "A hook, wrapper, permission layer, or sidecar can block a violating write before execution." |
| `isolated` | "Risky work or verification runs in a separate worktree, sandbox, process, or equivalent boundary." |

Rules:

- Do not imply cooperative means preventive.
- Do not imply a surface name guarantees a level.
- Guarantee level is not approval, verification, QA, acceptance, or a kernel gate.

## Guard And Freeze Safety Controls

Guard, freeze, and careful-mode language is user-facing safety-control language. Connectors may expose it as slash commands, buttons, prompt snippets, status actions, or recommended playbooks, but the display must name the actual capability and guarantee level behind the control.

```mermaid
flowchart TD
  Request["user safety-control request"] --> Freeze["freeze<br/>hold or request narrowed posture"]
  Request --> Guard["guard<br/>detect or enforce"]
  Request --> Careful["careful mode<br/>stricter posture"]
  Freeze --> Scope["display, next action, prepare_write hold, or routed owner update"]
  Guard --> Capability["connected capability profile and current enforcement path"]
  Careful --> Discipline["prepare_write, scope checks, evidence, and user questions"]
  Capability --> Guarantee["display actual guarantee level"]
  Guarantee --> Limitation["show limitation; command name is not the guarantee"]
```

`Freeze` means a user-visible hold or narrowed posture around current work. A freeze request may hold product writes, make the next action stricter, require a fresh Journey Card before resuming, or cause `prepare_write` to block or hold when the existing scope is incompatible with the requested posture. It does not by itself mutate the active Change Unit, allowed paths, Autonomy Boundary, AFK stop conditions, or related owner records, and it does not create Write Authorization, approval, acceptance, evidence, QA, verification, or a canonical close transition.

If a freeze needs to persistently narrow active Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions, or related owner records, the connector must route through the existing public Core state-changing path, Decision Packet route, or owner-record update path already defined for that record. The command label "freeze" is not a direct mutation path.

`Guard` means the surface adds an enforcement or detection layer according to its proven capability profile. A guard may be cooperative, detective, preventive, or isolated. The word "guard" must not imply that out-of-scope writes are physically blocked unless the connected profile has a proven pre-execution blocking path for the requested operation.

`Careful mode` is a stricter posture around existing authority checks. It should mean more explicit `prepare_write` use, narrower scope checks, more cautious evidence mapping, earlier Journey Card refreshes, and more willingness to ask the user one blocking question. It is not a new authority tier, not approval, not verification, and not a shortcut around `prepare_write`.

User-facing guarantee boundaries:

| User wording | Actual guarantee boundary |
|---|---|
| Freeze on `T2` / `cooperative` | The agent is instructed to hold or use a narrower posture; any persistent owner-record change still needs the normal Core path. There is no preventive claim. |
| Guard on `T3` / `detective` | Changed-path, log, artifact, or projection validation can detect violations after the fact and mark state stale, blocked, partial, or failed. |
| Guard on `T4` / `preventive` | A hook, wrapper, permission layer, policy engine, or sidecar can block the out-of-scope write or command before execution for the covered operation. |
| Guard or verify on `T5` / `isolated` | Risky work or verification happens inside a separate worktree, sandbox, process, or equivalent boundary. |

Surface and command names are labels only. A connector may call a button "Freeze" or a playbook "guard-check", but status, next, and `prepare_write` displays must still show whether the current path is cooperative, detective, preventive, or isolated and what that level can and cannot guarantee.

## Generated Manifest Concept

Connectors may generate rules, skills, MCP config snippets, prompts, or local adapter files. Every generated or managed path must be recorded in a connector manifest.

Manifest responsibilities:

- name generated paths
- record managed block hashes
- record capability profile used when generated
- record surface target profile
- record creation and update times
- detect drift before overwriting human edits
- route drift to reconcile when needed

The manifest concept is common. Surface-specific generated filenames belong in Appendix B.

```mermaid
flowchart LR
  Generated["generated or managed paths"] --> Manifest["connector manifest"]
  Manifest --> Hashes["managed block hashes"]
  Manifest --> Profile["capability profile and target profile"]
  Manifest --> Drift["drift detection before overwrite"]
  Drift --> Reconcile["reconcile item when needed"]
```

## Push And Pull Context

Implementation agents should receive small current context and pull larger references only when needed.

```mermaid
flowchart LR
  Current["current state and evidence"] --> Push["usually pushed"]
  LongRefs["longer references"] --> Pull["usually pulled"]
  Push --> Agent["implementation agent"]
  Pull --> Agent
  Agent --> Bundle["tighter evaluator bundle"]
  Bundle --> Evaluator["detached evaluator"]
```

Usually push:

- Journey Card
- active Decision Packet summary
- Autonomy Boundary summary
- Write Authority Summary
- active scoped Change Unit
- acceptance criteria snapshot
- approval status
- latest evidence manifest/run refs
- residual risk summary when close or acceptance is near

Usually pull:

- older PRDs
- old designs
- closed issues
- long logs
- module maps
- interface contracts
- domain language
- coding standards
- TDD guidance

Evaluators should receive a tighter verification bundle that includes:

- acceptance criteria
- changed files
- approval scope
- Decision Packets relevant to the work, including resolved, pending, or close-relevant packets
- residual risk summary
- Autonomy Boundary
- deferred decisions and follow-up constraints
- codebase stewardship refs, including relevant domain, module, and interface records
- evidence manifest
- TDD trace if required
- Manual QA requirement
- artifact refs
- forbidden patterns

This context model supports the Context Hygiene policy: current state and evidence are preferred over stale chat or old docs.

A later Context Index may help retrieve relevant projections, artifact refs, repo files, docs, or notes, but it is a read-only context provider, not a connector authority path. Main integration docs should point to [Appendix C](appendix/C-later-roadmap.md#context-index) for that concept; indexed or retrieved context must not authorize writes, resolve Decision Packets, grant approval, satisfy gates, create evidence, perform or record verification, record QA, waive QA or verification, accept residual risk, accept the result, upgrade assurance, enqueue or refresh projections, change projection freshness, declare implementation readiness, or close Tasks.

## Direct Fast Path

For small direct work, the agent should keep Harness mostly invisible: define a narrow active scope, call `prepare_write`, make the change, record changed paths, self-check evidence, and close if no blocker appears.

If scope, risk, uncertainty, or file spread grows, escalate the same Task to `work` instead of turning direct mode into broad autonomy.

## Fallback Semantics

Fallbacks are described by guarantee level and risk, not by surface name.

```mermaid
flowchart TD
  Need["fallback needed"] --> PreBlock{"pre-execution block available?"}
  PreBlock -- yes --> Preventive["preventive fallback"]
  PreBlock -- no --> Observe{"reliable after-action observation?"}
  Observe -- yes --> Detective["detective fallback"]
  Observe -- no --> Follow{"surface can follow instructions?"}
  Follow -- yes --> Cooperative["cooperative fallback"]
  Follow -- no --> Insufficient["capability insufficient"]
  Need --> Separation{"risk requires separation?"}
  Separation -- yes --> Isolated["isolated fallback"]
```

### Cooperative Fallback

Use when the surface can follow instructions but cannot enforce them. The connector tells the agent to call `prepare_write`, hold on blocked decisions, and record runs. Product writes must be paused if authoritative MCP is unavailable or the write scope cannot be checked.

### Detective Fallback

Use when the harness can observe changed files, logs, projection drift, or artifact gaps after the action. Validators may mark state stale, partial, blocked, or failed and require repair, reconcile, or fresh verification.

### Preventive Fallback

Use when a hook, permission layer, wrapper, policy engine, or sidecar can block a violating edit, command, network call, or secret access before it happens.

### Isolated Fallback

Use when risk requires separation. The connector launches work or verification in a separate worktree, sandbox, process, or manual evaluator bundle. This is the preferred fallback for detached verification when same-session review would not qualify.

### MCP Unavailable

If MCP is unavailable, the connector must not claim authoritative state updates. `MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE` are diagnostic conditions, not additional public `ErrorCode` values. When either condition is surfaced through `ToolError`, use the API-owned error selection and details shape: `MCP_UNAVAILABLE` remains the stable public availability code, while surface-side availability or capability cases may use `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind` according to context. `MCP_SERVER_UNAVAILABLE` means the tool call cannot reach Core, so no authoritative Core response is possible; the caller must reconnect or diagnose before claiming state changes. `SURFACE_MCP_UNAVAILABLE` means Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required MCP tools. For product/runtime/code writes, the safe behavior is to hold the write and direct the user/operator to reconnect or diagnose MCP. Stronger profiles may also enforce a preventive block.

A pre-MVP Harness documentation-authoring batch may proceed only under an explicit `DOCS_AUTHORING_OVERRIDE` with an exact path allowlist. The connector must label this as a documentation-maintainer override, not Core authorization, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, or a canonical state transition. Product/runtime/code writes still hold when authoritative MCP is unavailable.

```mermaid
flowchart TD
  Problem["MCP unavailable condition"] --> ReachCore{"tool call can reach Core?"}
  ReachCore -- no --> Server["diagnostic<br/>MCP_SERVER_UNAVAILABLE"]
  ReachCore -- yes --> Surface["diagnostic<br/>SURFACE_MCP_UNAVAILABLE"]
  Server --> NoState["no authoritative Core response"]
  Surface --> Diagnosed["Core or operator can observe unusable or stale surface MCP"]
  NoState --> Hold["hold product/runtime/code writes"]
  Diagnosed --> Hold
  Hold --> DocsOverride{"explicit DOCS_AUTHORING_OVERRIDE with exact allowlist?"}
  DocsOverride -- yes --> DocsOnly["documentation-maintainer override for listed docs only"]
  DocsOverride -- no --> Reconnect["reconnect or diagnose MCP"]
```

### Weak Guard

If MCP works but pre-tool guard is weak, low-risk direct work may proceed with cooperative `prepare_write` and detective changed-path validation. Medium/high-risk work should require stricter validation, sidecar guard, explicit approval, detached verification, or isolation.

### Projection Stale

Projection staleness is reported separately from state. A connector may continue from canonical state if it can read state directly, but actions that depend on the Markdown projection should refresh or reconcile first.

### Capability Insufficient

The connector should name the missing capability, not the product name. Example:

```text
The connected profile does not provide pre-tool guard. This work needs sidecar guard, another profile, or a smaller active scoped Change Unit.
```

## Reference Surface Contract

The MVP targets one reference surface. The reference surface should demonstrate the kernel rather than broad ecosystem support.

Minimum reference expectations:

- `T2 MCP` available for public tools and resources
- cooperative `prepare_write` before product writes
- detective changed-path and artifact validation after runs
- run summary and artifact capture sufficient for evidence manifests
- manual verification bundle or fresh evaluator instructions
- Manual QA note artifact support
- connector manifest for generated files
- conformance smoke covering common state and fallback paths

Reference surface behavior details and product-specific setup belong in Appendix B only when they name a concrete surface.

## Connector Conformance Overview

Connector conformance should prove that a profile can uphold the common contract at its declared capability tier.

Overview scenarios:

```mermaid
flowchart TB
  Conformance["connector conformance"]
  Conformance --> Status["status, intake, Journey Card"]
  Conformance --> Shaping["shared design, scope, one blocking question"]
  Conformance --> Decisions["Decision Packets and Autonomy Boundary"]
  Conformance --> Writes["prepare_write, Write Authority Summary, record_run authorization"]
  Conformance --> Evidence["artifacts, evidence, verification, same-session guard"]
  Conformance --> QA["Manual QA, acceptance, residual-risk visibility"]
  Conformance --> Drift["projection, generated file drift, reconcile"]
  Conformance --> Fallbacks["capability fallback and MCP unavailable hold"]
```

- status with and without an active Task
- current Journey Card shown before significant work resumes as required
- intake classification into advisor/direct/work
- work shaping with shared design and decisions
- Change Unit scope and vertical/horizontal exception handling
- one blocking question with recommendation and uncertainty when available
- Decision Packet shown instead of broad approval for blocking product judgment
- Autonomy Boundary breach stops or routes to Decision Packet
- AFK work remains covered by active Change Unit scope, Autonomy Boundary latitude, any granted sensitive approval that applies, and compatible `prepare_write` / Write Authorization before actual product writes
- `prepare_write` allowed and blocked paths
- Write Authorization created for allowed writes and exposed through Write Authority Summary
- write-capable `record_run` consumes a compatible Write Authorization
- sensitive approval request, granted, denied, and expired paths
- `record_run` with artifacts and evidence update
- direct result projection
- verification launch or manual verification bundle
- same-session verification guard
- Manual QA required, passed, failed, and waived
- QA waiver with product/user risk routes through Decision Packet
- acceptance required and recorded
- acceptance focus includes residual risk visibility before acceptance is requested
- Known close-relevant residual risk must be visible before any successful close
- Risk-accepted close additionally requires accepted Residual Risk refs
- Acceptance, when required, can be recorded only after close-relevant residual risk is visible
- stale projection and reconcile flow
- generated file drift detection
- capability fallback when a required tier is missing
- MCP unavailable product-write hold

Exact fixture format and operational commands are owned by operations and conformance docs.
