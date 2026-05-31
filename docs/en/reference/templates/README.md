# Template Reference

## Used when

Use these files when you need the rendered Markdown shape for projection templates and display cards. The projection rules, authority boundaries, and freshness behavior are defined in [Document Projection Reference](../document-projection.md).

Owner boundary: templates are rendered shapes, not canonical state. They do not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before documentation acceptance and a separate implementation-planning readiness decision. The first runnable target is v0.1 Core Authority Slice, with Kernel Smoke as its narrow conformance authoring profile. The first product MVP target is v0.2 User-Facing Harness MVP. v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack harden agency assurance, operations, and handoff behavior, and v1+ Expansion remains roadmap scope unless owner docs explicitly promote and prove it.

Do not read this directory as a list of everything required for early implementation. The tier tables below separate required early shapes from optional early shapes and future/diagnostic templates.

## Output tiers

Projection and card shapes serve three output tiers:

| Output tier | What belongs here | Rule |
|---|---|---|
| User-readable outputs | Current work status, user decision request, evidence summary, and close readiness / blocker summary. | Required for the user-facing MVP, but may be rendered through status/next text, compact cards, or minimal `TASK` sections. |
| Agent compact context | Minimal current state needed for the next safe step: active Task, scope, active Change Unit when relevant, pending user decision, evidence/close blockers, next action, refs, and freshness. | Keep compact; do not embed long history or detailed artifacts. |
| Reference/diagnostic outputs | Detailed manifests, traces, maps, Journey Card or Journey Spine views, run summaries, detailed Eval reports, export bundles, and operator reports. | Pull-on-demand or later-profile outputs; not mandatory for the first runnable slice or minimum user-facing MVP. |

## Template implementation classes

Template classes stage rendered shapes without changing authority:

| Class | Templates | Rule |
|---|---|---|
| Required for v0.1 Core Authority Slice | [Compact Status Card](compact-status-card.md) or equivalent status/next/blocker response shape | Minimal read-only status from current Core state. No persisted Markdown projection job is required. |
| Required for user-facing MVP | [TASK](task.md) minimal continuity summary; [Decision Packet display/card shape](decision-packet.md) for user decision requests, not the standalone `DEC` `ProjectionKind` | Enough to show current status, user decision request, evidence summary, and close readiness/blockers. Standalone persisted `DEC` Markdown remains optional unless the standalone Decision Packet projection feature is enabled. |
| Optional early | [APR](approval.md), [Approval Card](approval-card.md), [DIRECT-RESULT](direct-result.md), [MANUAL-QA](manual-qa.md), [Manual QA Card](manual-qa-card.md), [Verification Result Card](verification-result-card.md) | Use only when the corresponding approval, direct-work, Manual QA, or verification profile is active. |
| Future / diagnostic | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [TDD-TRACE](tdd-trace.md), [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [DESIGN](design.md), [EXPORT](export.md), [JOURNEY-CARD](journey-card.md) | Detailed reference, diagnostic, handoff, stewardship, map, trace, or export views. Keep available for v0.3 Agency Assurance Pack, v0.4 Operations & Handoff Pack, or other owner-promoted later profiles without making them mandatory early scope. |

v0.1 Core Authority Slice does not require broad template rendering. v0.2 User-Facing Harness MVP needs enough derived output for users to understand scope, user decisions, evidence, close readiness, final acceptance, and residual risk; it does not require Run Summary, Evidence Manifest, detailed Eval, TDD Trace, Journey Card, Module Map, Interface Contract, or Export projection polish.

`Future / diagnostic` means later-profile or diagnostic scope, not automatically v1+ only.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are readable views from owner records and refs. They must not redefine kernel fields, MCP schemas, SQLite DDL, gate behavior, or artifact integrity rules.

Rendered placeholders, labels, table columns, and example front matter keys are template bindings for display. A binding must either show an existing owner record field or ref, or be a derived display summary from the source records named by the template. If the source record or ref does not exist, render `none`, `unknown`, `not_required`, or an unavailable/blocking note instead of inventing state.

Compact authority displays should prefer a short refs line when several sources are relevant: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, and `freshness=`. These labels point to existing refs, redaction state, and projection freshness only; they are not schema fields or authority records.

Derived display summaries include approval boundary lines such as `approval_covers`, `approval_does_not_cover`, and `secret_exposure_boundary`; close context, close blockers, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, and judgment-context-related summaries. These names are not new canonical records, schema fields, DDL columns, `ProjectionKind` values, gates, authority inputs, or authority paths. They must not be used as validator input except through the owner records, refs, gates, artifacts, or Decision Packets they summarize.

Rendered examples should make that boundary visible to the reader. `source_state_version` names the state clock used for the render, `projection_version` or projection status names the render/template/job view, and `updated_at` names when the view was produced. Freshness lines say whether the view still matches its source records; they are not task results, gate values, approval, acceptance, evidence, close readiness, or Core state rollback.

Managed blocks are projector-owned display. Direct edits inside managed blocks are drift and should become reconcile candidates, not state changes. Human-editable sections such as `User Notes and Proposals` are proposal surfaces: they become state only through proposal -> reconcile item -> accepted Core state-changing action with the relevant `state.sqlite.task_events` row, or they remain rejected, deferred, or note-only content.

Any template that renders artifact refs must preserve `redaction_state`. Large logs, diffs, traces, screenshots, bundles, recordings, and sensitive artifact bodies are referenced by `ArtifactRef`, not embedded by default. `secret_omitted` entries may show safe notes or handles and may support only visible nonsecret evidence; `blocked` entries show the committed metadata-only notice as unavailable input. Templates must not inline, reconstruct, summarize, or export omitted secret/PII values or blocked raw payload bytes.

Display fields such as `redaction_availability_summary`, omitted or blocked impact lines, and `Downstream Effect` columns are rendered summaries only. They are derived from `ArtifactRef.redaction_state`, owner records, and downstream gate, evidence, QA, verification, projection, export, or Release Handoff status.

Decision Packet visibility does not depend on a standalone `DEC` Markdown projection. Required surfaces must still show active Decision Packets through `TASK`, status/next responses, judgment-context resources, and decision-packet resources. Standalone `DEC` is only an optional rendered view when that projection is enabled.

Decision Packet displays may include reader-facing shape fields such as decision title, `decision_profile`, `judgment_domain`, why this is needed now, what the user is deciding, concise options or detailed trade-offs, recommendation, uncertainty, deferral consequence, and residual risk when relevant. `decision_profile` is schema-owned and controls whether the display is a concise `minimal_decision` or a fuller profile such as `product_ux_tradeoff`, `architecture_tradeoff`, `approval_shaped`, `waiver`, `acceptance`, `residual_risk_acceptance`, `reconcile`, or `mixed`. `judgment_domain` is schema-owned and uses `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, or `mixed`; templates may render those values as Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, or Mixed. If a decision is cross-cutting, templates should render secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of treating the domain as exclusive. These labels help readers, but they are not `ProjectionKind` values, gates, owner records, validator inputs, close aggregation rules, or authority paths.

Display cards should distinguish three different problems: a stale projection means the readable view may lag behind its source records, stale state or stale evidence means the underlying state, baseline, or artifact inputs have moved or become insufficient, and MCP unavailable means the surface cannot reach the required Harness/Core capability. Only the owner records and Core transitions can change state.

Close and assurance displays must keep distinct labels for self-checked work, `detached_verified` assurance, waived verification, QA waiver, and residual-risk accepted `completed_with_risk_accepted` close. They may appear in the same compact card, but should not be collapsed into "done," "verified," or "accepted" without the owner refs that support each state.

## Future / Diagnostic Templates

- [DESIGN](design.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [EXPORT](export.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TDD-TRACE](tdd-trace.md)

## Required Early Shapes

- [Compact Status Card](compact-status-card.md)
- [TASK](task.md) minimal continuity summary
- [Decision Packet user decision request display shape](decision-packet.md)

## Optional Early Shapes

- [APR](approval.md)
- [Approval Card](approval-card.md)
- [DIRECT-RESULT](direct-result.md)
- [MANUAL-QA](manual-qa.md)
- [Manual QA Card](manual-qa-card.md)
- [Verification Result Card](verification-result-card.md)

## Notes

This directory is the active reference location for projection template bodies and display card shapes.
