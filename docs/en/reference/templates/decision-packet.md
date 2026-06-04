# DEC Template

## Used when

Use `DEC` only when standalone full-format Decision Packet presentation is enabled for a specific `user_judgment` record. The ordinary MVP-1 path is a compact judgment request through status, next-action, or user-judgment resources. Small unblockers should fit on one screen and should not expose this full template unless the user asks for drill-down.

Supported user-facing display labels are:

- Product/UX judgment
- Technical judgment
- Sensitive action approval
- Work acceptance
- Residual risk acceptance

Boundary: projection template only; it does not authorize runtime/server implementation or generated operational outputs. Shared phase and projection rules live in [Template Reference](README.md#used-when).

Implementation tier: optional full-format judgment presentation. A standalone persisted `DEC` Markdown projection remains optional unless the standalone Decision Packet projection feature is enabled. "Decision Packet" is the presentation label; `user_judgment` is the canonical record family.

## Source records

- `state.sqlite.user_judgments`
- related Task and Change Unit refs
- `judgment_type`, `presentation`, and `display_label`
- related `decision_gate` state and user-judgment events
- `approval_scope` for `judgment_type=sensitive_action_approval`, plus Approval records only when a later Approval profile is active
- related reconcile records, if applicable and enabled by a later profile
- residual risk refs
- evidence summaries, Run refs, ArtifactRefs, and visible evidence gaps in minimum MVP-1; Evidence Manifest refs only when the full Evidence Manifest profile is active
- Write Authorization, sensitive-action permission, Eval, Manual QA, work-acceptance context, residual-risk refs, ArtifactRefs, redaction state, and projection freshness when displayed as related authority context
- affected scope display inputs: product areas, screens or flows, modules, interfaces, paths, acceptance criteria, gates, and sensitive categories
- projection freshness inputs

Legacy names such as `decision_packet_id`, `judgment_category`, `judgment_route`, and `display_depth` may appear only in migration notes or compatibility drill-down. New templates, examples, and fixtures should use `user_judgment_id`, `judgment_type`, `presentation`, `display_label`, and `record_kind=user_judgment`.

Sensitive-action approval display bullets such as "what this approval covers," "what this approval does not cover," and "secret exposure boundary" are derived display summaries from `judgment_payload.approval_scope`, the related `user_judgment` ref, linked Approval records only when that later profile is active, and current write or close context. They explain the boundary only; they do not settle separate user-owned judgment, create Write Authorization, or imply a committed Approval record in minimum MVP-1. Sensitive action approval displays must not look like work acceptance or residual risk acceptance.

A resolved user judgment grants sensitive-action permission only when it uses `judgment_type=sensitive_action_approval` with compatible `approval_scope`. Other user judgment resolutions may settle product/UX judgments, technical judgments, work acceptance, residual-risk acceptance, or later-profile waiver/reconcile choices, but they do not grant sensitive-action permission.

`presentation=short` is the default for simple unblockers and compact prompts. `presentation=full` is the full-format Decision Packet-style presentation for complex, high-risk, close-affecting, reconcile, or later-profile judgments. Presentation changes how much context is rendered; it does not change authority.

## Rendered sections

- Why Now
- Current State
- Judgment Type And Presentation
- Sensitive Action Approval Context, If Applicable
- What User Is Judging
- What Agent May Decide Without User
- Autonomy Boundary Impact, If Any
- Affected Scope And Boundaries
- Options
- Recommendation
- Consequence Of Deferring
- Minimum Context To Judge
- User Judgment
- Residual-Risk Acceptance, If Applicable
- Follow-Up
- References

A sufficient rendered Decision Packet answers one user-owned judgment, not broad permission. The exact public request and response fields are owned by [`harness.request_user_judgment`](../api/mvp-api.md#harnessrequest_user_judgment), and the canonical authority rules are owned by [User Judgment](../kernel.md#user-judgment) and [Decision Gate](../kernel.md#decision-gate). This template may summarize existing user judgment fields, but it must not add schema fields, gates, or alternate authority.

The user-facing question should ask for the judgment directly: choose an option, defer it with the stated consequence, reject the path, grant or deny sensitive action approval, accept or reject the result, accept or reject a named residual risk, or record a later-profile waiver/reconcile outcome when enabled. Use "approve" only for Sensitive action approval or a later Approval record. If several judgments are pending, render separate prompts or separate lines; do not combine approval, work acceptance, and residual risk acceptance into one answer.

**Example content cues:**

Use the same rendered sections for these common full-format user judgment shapes. These cues are not extra template sections.

- Tiny unblocker (`judgment_type=product_choice`, `presentation=short`): choose whether a button label should say "Save" or "Update" inside an already scoped settings copy change. Put the concise choice, scope, refs, and non-effects under What User Is Judging and References. Do not force a full architecture-tradeoff layout.
- Product/UX judgment (`judgment_type=product_choice`): failed-login feedback as inline layer, toast, or modal; failed-login wording as generic, specific, or hybrid. Put flow, interruption, accessibility, copy, product tone, and user-risk differences under Options and Recommendation.
- Technical judgment (`judgment_type=technical_choice`): session cookie, bearer/JWT token, OAuth/OIDC provider, or social-login provider integration. Put revocation, CSRF/XSS exposure, client compatibility, implementation cost, identity-provider boundaries, and migration impact under Options and Minimum Context To Judge.
- Technical judgment (`judgment_type=technical_choice`): dependency adoption, schema/data-model migration, public API/interface direction, module boundary changes, privacy/logging policy, QA expectation, verification expectation, waiver, scope/autonomy expansion, or reconcile choice when that later profile is active.
- Sensitive action approval (`judgment_type=sensitive_action_approval`): dependency install, secret access, network write, destructive write, or other scoped sensitive step. Put the approval boundary under Sensitive Action Approval Context and do not treat it as resolving a product/UX or technical judgment.
- Work acceptance (`judgment_type=work_acceptance`): put the final result, evidence status, Manual QA and verification status, and close-relevant residual-risk visibility under Current State and Minimum Context To Judge. Do not treat work acceptance as approval for new sensitive actions, additional writes, deployment, merge, or residual-risk acceptance.
- Residual risk acceptance (`judgment_type=residual_risk_acceptance`): put the visible limitation, existing evidence, risk refs the user is being asked to accept, and remaining follow-up under Current State, Minimum Context To Judge, Residual-Risk Acceptance, and Follow-Up.
- Broad "go ahead" answers: show why the prompt asks for this specific judgment type and option. A generic consent phrase does not resolve product/UX judgment, technical judgment, sensitive action approval, work acceptance, or residual risk acceptance unless this prompt records that exact judgment.

**Rendered example: minimal judgment**

```text
Judgment request: Settings label wording
Record: user_judgment_id=UJ-0001
Judgment type: product_choice
Presentation: short
Display label: Product/UX judgment
Question: Should this scoped settings label say "Save" or "Update"?
Scope/refs: settings form copy in CU-04; source ref TASK-012/CU-04; no sensitive action or close-risk ref.
Choice to record: Save | Update
Does not settle: broader settings flow behavior, localization strategy, work acceptance, residual-risk acceptance, or write authority.
```

**Rendered example: sensitive action approval**

```text
Judgment request: Dependency install approval
Record: user_judgment_id=UJ-0002
Judgment type: sensitive_action_approval
Presentation: short
Display label: Sensitive action approval
Question: Do you grant this named dependency install/update action for this task?
Approval scope: named install command or dependency-file update; named manifest/lockfile paths; current task and approval window only.
Covers: the scoped sensitive action.
Does not cover: resolving whether the dependency is the right architecture direction, future installs, unrelated product writes, QA or verification waiver, work acceptance, or residual-risk acceptance.
Separate judgments required: use `judgment_type=technical_choice` if the dependency choice itself is still user-owned judgment.
Refs: approval scope refs, prepare-write candidate refs, dependency comparison refs, and affected file refs when available.
```

**Rendered example: full technical trade-off**

```text
Judgment request: Login session architecture
Record: user_judgment_id=UJ-0003
Judgment type: technical_choice
Presentation: full
Display label: Technical judgment
Question: Which session model should this login work use?
Options: server-side session cookie; client-held bearer/JWT; OAuth/OIDC provider plus local session strategy; social-login provider integration.
Recommendation: server-side session cookie for a first-party web app unless current requirements need third-party identity, non-browser clients, or social sign-in now.
Uncertainty: existing session middleware, revocation requirements, SSO requirement, CSRF posture, and migration constraints.
Deferral consequence: read-only inspection and UI scaffolding can continue only if they do not commit to storage, token lifetime, provider, or middleware behavior.
Refs: auth model refs, affected acceptance criteria, security evidence refs when available, and any residual-risk or migration refs.
```

## Full template

````md
---
doc_type: user_judgment_decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
user_judgment_id: UJ-0001
task_id: TASK-0001
change_unit_id: CU-01
judgment_type: product_choice
presentation: full
display_label: Product/UX judgment
status: pending_user
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# UJ-0001 Judgment Request Title

> Projection view: rendered from `source_state_version` at `updated_at`; displays `user_judgment_id` and related refs from state. Editing this Markdown does not resolve the judgment; answers are recorded through `harness.record_user_judgment`.

## Why Now
- trigger:
- blocker:
- affected operation:
- why this cannot proceed under current state:

## Current State
- task state:
- active change unit:
- current gates:
- latest evidence:
- residual risk:
- source refs: judgment={user_judgment_id}; write={write_authorization_ref|none}; sensitive_action_permission={user_judgment_ref|approval_ref_when_profile_active|none}; evidence={evidence_summary_ref|evidence_manifest_ref_when_profile_active|none}; eval={eval_ref|none}; manual_qa={manual_qa_ref|none}; acceptance={work_acceptance_user_judgment_ref|none}; residual_risk={residual_risk_refs|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

## Judgment Type And Presentation
- judgment_type: product_choice | technical_choice | sensitive_action_approval | work_acceptance | residual_risk_acceptance
- presentation: short | full
- display_label: Product/UX judgment | Technical judgment | Sensitive action approval | Work acceptance | Residual risk acceptance
- final recorded answer:
- what this judgment can record:
- what this judgment cannot record:
- generic consent handling:

## Sensitive Action Approval Context, If Applicable
- card label: Sensitive action approval
- judgment_type=sensitive_action_approval scope:
- linked approval record (later profile only):
- sensitive categories:
- what this approval covers:
- what this approval does not cover:
- must not be rendered as: work acceptance or residual-risk acceptance
- separate user-owned judgment still required:
- approval boundary:
- write authorization boundary:
- secret exposure boundary:

## What User Is Judging
- judgment type:
- display label:
- user-facing question:
- decision:
- what this decision settles:
- what this decision does not settle:
- why broad approval is insufficient:

## What Agent May Decide Without User
- implementation detail:
- code organization inside granted scope:
- evidence collection:
- follow-up proposal:

## Autonomy Boundary Impact, If Any
- current boundary impact:
- proposed boundary update:
- user judgment required:

## Affected Scope And Boundaries
- in scope:
- out of bounds:
- affected product areas:
- affected screens or flows:
- affected modules/interfaces/paths:
- affected acceptance criteria:
- affected gates:
- sensitive categories:

## Options
### Option A
- choice:
- trade-offs:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high

### Option B
- choice:
- trade-offs:
- benefits:
- costs:
- risks:
- reversibility: reversible | partially_reversible | irreversible | unknown
- confidence: low | medium | high

## Recommendation
- recommended option:
- rationale:
- confidence:
- what would change the recommendation:

## Consequence Of Deferring
- what can continue:
- what remains blocked:
- close impact:

## Minimum Context To Judge
- evidence visible:
- unknowns:
- QA/verification state:
- residual risk visibility:
- close or write impact:

## User Judgment
- selected option:
- value: selected | rejected | deferred | granted | denied | expired | accepted
- note:
- decided by:
- decided at:
- broad consent check: "proceed", "go ahead", and "looks good" do not automatically become sensitive-action approval, work acceptance, or residual-risk acceptance.

## Residual-Risk Acceptance, If Applicable
- named risk:
- visible risk refs:
- acceptance scope:
- consequence of accepting:
- follow-up:

## Follow-Up
- required before write:
- required before close:
- suggested follow-up:

## References
- task:
- change unit:
- user judgment:
- write authority:
- evidence:
- verification:
- Manual QA:
- residual risk:
- artifacts:
- projection freshness:
````

## Notes

This template is a rendered shape, not canonical state. User judgment visibility required by the active stage/profile can come through compact status cards, status/next responses, judgment-context resources, user-judgment resources, or a dedicated prompt. Standalone `DEC` projection remains optional.

Decision Packet projections should keep authority context refs compact and explicit. Displaying a Write Authorization, sensitive-action permission ref, evidence summary, Evidence Manifest when its profile is active, Eval, Manual QA, work acceptance, residual-risk visibility, residual-risk acceptance, artifact, redaction, or freshness ref in this template does not make the prose authoritative for that record.

Decision Packet cards should display one judgment type at a time. Sensitive action approval prompts use approval language, work acceptance prompts use work-acceptance language, and residual-risk acceptance prompts name the specific risk being accepted.
