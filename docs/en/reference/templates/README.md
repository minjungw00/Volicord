# Template Reference

## Used when

Use these files when you need the rendered Markdown shape for projection templates and display cards. The projection rules, authority boundaries, and freshness behavior are defined in [Document Projection Reference](../document-projection.md).

Owner boundary: templates are rendered shapes, not canonical state. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../../build/implementation-overview.md#documentation-acceptance-status).

Do not read this directory as a list of everything required for early implementation. The tier tables below separate core status output, minimum user-facing summaries, assurance reports, operations/export reports, and future/diagnostic projections.

## Output tiers

Projection and card shapes serve five staged output tiers:

| Output tier | What belongs here | Rule |
|---|---|---|
| Core status output | Minimal current status, blockers, next allowed action, refs, and freshness facts from Core state. | v0.1 may return plain structured output; a compact card is optional and does not imply full projection support. |
| User-facing MVP summaries | Current work status, user decision request, evidence summary, and close readiness / blocker summary. | Required to support v0.2 user value, but may be rendered through status/next text, compact cards, or minimal `TASK` sections. Final-acceptance and residual-risk facts stay distinct when relevant without adding required projection kinds. |
| Agency assurance reports | Approval, Manual QA, verification, waiver, and assurance card/report views when those profiles are enabled. | v0.3 profile scope. These views do not become v0.1 or minimum v0.2 requirements. |
| Operations/export reports | Export, release-handoff, projection freshness, artifact-integrity, and operator report views. | v0.4 profile scope when operations support is enabled; reports are readable views, not operational authority. |
| Future/diagnostic projections | Detailed Evidence Manifest, detailed Eval, Run Summary, TDD Trace, Module Map, Interface Contract, Journey Card or Journey Spine-style views, standalone Decision Packet Markdown, and design/domain-language maps. | Pull-on-demand or owner-promoted later-profile outputs; not mandatory for the first runnable slice or minimum user-facing MVP. |

## Template implementation classes

Template classes stage rendered shapes without changing authority:

| Class | Templates | Rule |
|---|---|---|
| Core status output | [Compact Status Card](compact-status-card.md) or equivalent status/blocker response shape | Minimal read-only status from current Core state when an implementer chooses a card shape. A plain structured response is enough; no persisted Markdown projection job or template renderer is required. |
| User-facing MVP summaries | [TASK](task.md) minimal continuity summary; [Decision Packet display/card shape](decision-packet.md) for user decision requests, not the standalone `DEC` `ProjectionKind`; optional [DIRECT-RESULT](direct-result.md) for active direct-work profiles | Enough to show current status, user decision request, evidence summary, and close readiness/blockers. Final-acceptance and residual-risk facts stay distinct when relevant without adding required projection kinds. Standalone persisted `DEC` Markdown remains optional unless the standalone Decision Packet projection feature is enabled. |
| Agency assurance reports | [APR](approval.md), [Approval Card](approval-card.md), [MANUAL-QA](manual-qa.md), [Manual QA Card](manual-qa-card.md), [Verification Result Card](verification-result-card.md) | Use only when the corresponding approval, Manual QA, waiver, verification, or assurance profile is active. |
| Operations/export reports | [EXPORT](export.md) | Use only when export, release-handoff, or operations report support is enabled. Standalone Markdown reports do not replace Core state or artifact refs. |
| Future/diagnostic projections | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [TDD-TRACE](tdd-trace.md), [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [DESIGN](design.md), [JOURNEY-CARD](journey-card.md), and standalone `DEC` Markdown when enabled | Detailed reference, diagnostic, stewardship, map, trace, persisted Journey Card, Journey Spine-style, standalone Decision Packet, and detailed Evaluation views. Keep available for owner-promoted later profiles without making them mandatory early scope. |

v0.1 Core Authority Slice does not require broad template rendering or a full projection renderer. Its required output is the structured status/blocker response named by Build, which may be rendered through this compact card only if that is the simplest implementation choice. v0.2 User-Facing Harness MVP needs enough derived output for users to understand current work status, user decisions, evidence, and close blockers. Final acceptance and residual risk remain separate Core meanings when relevant, but they should fit inside those minimal summaries instead of becoming extra required projection kinds. This is supporting display scope, not the stage's primary identity; it does not require Run Summary, Evidence Manifest, detailed Eval, TDD Trace, Journey Card, Module Map, Interface Contract, or Export projection polish.

`Future/diagnostic projections` means later-profile or diagnostic scope, not automatically v1+ only.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are readable views from owner records and refs. They must not redefine kernel fields, MCP schemas, SQLite DDL, gate behavior, or artifact integrity rules.

Rendered placeholders, labels, table columns, and example front matter keys are template bindings for display. A binding must either show an existing owner record field or ref, or be a derived display summary from the source records named by the template. If the source record or ref does not exist, render `none`, `unknown`, `not_required`, or an unavailable/blocking note instead of inventing state.

Compact authority displays should prefer a short refs line when several sources are relevant: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, and `freshness=`. These labels point to existing refs, redaction state, and projection freshness only; they are not schema fields or authority records.

Derived display summaries include approval boundary lines such as `approval_covers`, `approval_does_not_cover`, and `secret_exposure_boundary`; close context, close blockers, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, and judgment-context-related summaries. These names are not new canonical records, schema fields, DDL columns, `ProjectionKind` values, gates, authority inputs, or authority paths. The labels themselves must not be used as validator inputs; validators consume the owner records, refs, gates, artifacts, or Decision Packets those labels summarize.

Rendered examples should make that boundary visible to the reader. `source_state_version` names the state clock used for the render, `projection_version` or projection status names the render/template/job view, and `updated_at` names when the view was produced. Freshness lines say whether the view still matches its source records; they are not task results, gate values, approval, acceptance, evidence, close readiness, or Core state rollback.

Managed blocks are projector-owned display. Direct edits inside managed blocks are drift and should become reconcile candidates, not state changes. Human-editable sections such as `User Notes and Proposals` are proposal surfaces: they become state only through proposal -> reconcile item -> accepted Core state-changing action with the relevant `state.sqlite.task_events` row, or they remain rejected, deferred, or note-only content.

Any template that renders artifact refs must preserve `redaction_state`. Large logs, diffs, traces, screenshots, bundles, recordings, and sensitive artifact bodies are referenced by `ArtifactRef`, not embedded by default. `secret_omitted` entries may show safe notes or handles and may support only visible nonsecret evidence; `blocked` entries show the committed metadata-only notice as unavailable input. Templates must not inline, reconstruct, summarize, or export omitted secret/PII values or blocked raw payload bytes.

Display fields such as `redaction_availability_summary`, omitted or blocked impact lines, and `Downstream Effect` columns are rendered summaries only. They are derived from `ArtifactRef.redaction_state`, owner records, and downstream gate, evidence, QA, verification, projection, export, or Release Handoff status.

Decision Packet visibility does not depend on a standalone `DEC` Markdown projection. Required surfaces must still show active Decision Packets through `TASK`, status/next responses, judgment-context resources, and decision-packet resources. Standalone `DEC` is only an optional rendered view when that projection is enabled.

Decision Packet displays may include canonical schema fields and reader-facing shape fields such as decision title, `decision_kind`, `decision_profile`, `judgment_domain`, why this is needed now, what the user is deciding, concise options or detailed trade-offs, recommendation, uncertainty, deferral consequence, and residual risk when relevant. `decision_kind` is the lifecycle/gate route. `decision_profile` is schema-owned; validators use it to select and validate the matching Decision Packet `profile_payload` branch and profile-specific required fields, and templates use it to choose display depth for a concise `minimal_decision` or a fuller profile such as `product_ux_tradeoff`, `architecture_tradeoff`, `approval_shaped`, `waiver`, `acceptance`, `residual_risk_acceptance`, `reconcile`, or `mixed`. `judgment_domain` is a schema-owned enum with values `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, or `mixed`; validators should validate the enum, while templates may render those values as Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, or Mixed. If a decision is cross-cutting, templates should render secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of treating the domain as exclusive. Friendly labels derived from `decision_profile` or `judgment_domain` help readers, but they are not schema fields, `ProjectionKind` values, gates, owner records, validator inputs, close aggregation rules, authority paths, or replacements for `decision_kind`.

Display cards should distinguish three different problems: a stale projection means the readable view may lag behind its source records, stale state or stale evidence means the underlying state, baseline, or artifact inputs have moved or become insufficient, and MCP unavailable means the surface cannot reach the required Harness/Core capability. Only the owner records and Core transitions can change state.

Close and assurance displays must keep distinct labels for self-checked work, `detached_verified` assurance, waived verification, QA waiver, and residual-risk accepted `completed_with_risk_accepted` close. They may appear in the same compact card, but should not be collapsed into "done," "verified," or "accepted" without the owner refs that support each state.

## Future/Diagnostic Projection Templates

- [DESIGN](design.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TDD-TRACE](tdd-trace.md)

## Core Status Output

- [Compact Status Card](compact-status-card.md)

## User-Facing MVP Summary Shapes

- [TASK](task.md) minimal continuity summary
- [Decision Packet user decision request display shape](decision-packet.md)
- [DIRECT-RESULT](direct-result.md), only when direct-work compact result display is active

## Agency Assurance Report Shapes

- [APR](approval.md)
- [Approval Card](approval-card.md)
- [MANUAL-QA](manual-qa.md)
- [Manual QA Card](manual-qa-card.md)
- [Verification Result Card](verification-result-card.md)

## Operations/Export Report Shapes

- [EXPORT](export.md)

## Notes

This directory is the active reference location for projection template bodies and display card shapes.
