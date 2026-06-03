# Template Reference

## Used when

Use these files when you need the rendered Markdown shape for projection templates and display cards. The projection rules, authority boundaries, and freshness behavior are defined in [Document Projection Reference](../document-projection.md).

Authority rule:

- Projection is derived from Core-owned state records and artifact references.
- Projection is not Core state.
- User edits to a projection are input only; they are not automatically accepted state.
- Chat and Markdown cannot override Core state.

Owner boundary: templates are rendered shapes, not canonical state. Current repository phase and implementation handoff status are tracked in [Implementation Overview](../../build/implementation-overview.md#documentation-acceptance-status).

This directory is a catalog of shapes, not a stage checklist. A template existing in the repository does not mean that template is required in the current stage.

## Projection audiences

Keep these audiences separate:

| Audience | Shape | Rule |
|---|---|---|
| User-facing compact card | [Compact Status Card](compact-status-card.md) | The v0.2 First User-Value Slice projection: one small current-state card. |
| Agent compact context/reference payload | Structured refs, blocker labels, source clocks, freshness, and next-action hints | Derived support payload. Compact by default; no full report bodies unless pulled for a phase-specific reason. |
| Future/diagnostic reports | `TASK`, Journey Card/Spine, Run Summary, detailed Evidence Manifest, detailed Eval, full Manual QA, TDD Trace, Domain Language, Module Map, Interface Contract, Design, Export, full Approval Card, and other polished reports | Later/profile or diagnostic output. Display-only, never authority. |

## v0.2 First User-Value Slice projection

The v0.2 First User-Value Slice projection is one compact status card. It must show:

- current Task summary
- work shape
- current scope and non-goals
- pending user judgments
- active blockers
- next safe actions
- known evidence or evidence gaps
- close blockers
- visible residual risk
- guarantee level
- source/freshness references

The card must be readable for users and compact for agents. It should not dump schema fields, DDL, event logs, full artifacts, full reference docs, full Evidence Manifests, or full report bodies.

## Template-to-stage matrix

| Template | Audience | First active stage | Authority status | Notes |
|---|---|---|---|---|
| [Compact Status Card](compact-status-card.md) | User-facing compact card; agent compact context source | v0.2 First User-Value Slice projection; optional as v0.1 status rendering | Derived display only | The only v0.2 First User-Value Slice projection shape. Plain structured output is still enough for v0.1. |
| [Decision Packet display](decision-packet.md) | User judgment prompt/display | v0.2 when user judgment flow is active | Derived display over `state.sqlite.decision_packets`; not standalone authority | Required judgments can appear through status/next or resources. Standalone `DEC` Markdown is optional later. |
| [TASK](task.md) | Continuity/reference report | Later/profile or diagnostic | Derived display only | Not the v0.2 First User-Value Slice projection. Expanded continuity sections are later polish. |
| [DIRECT-RESULT](direct-result.md) | Compact direct-work result | Later/profile when direct-work display is active | Derived display only | Optional convenience shape; not needed for the compact status card MVP. |
| [APR](approval.md) | Sensitive-action approval report | v0.3 agency assurance profile | Displays Approval and Decision Packet refs; does not grant approval | Use only after approval support/profile is active. |
| [Approval Card](approval-card.md) | Sensitive-action approval prompt/card | v0.3 agency assurance profile | Displays approval boundary; does not authorize writes | Full approval card is not v0.2 First User-Value Slice. |
| [MANUAL-QA](manual-qa.md) | Manual QA report | v0.3 agency assurance profile | Displays `manual_qa_records`/`qa_gate`; does not perform QA | Full Manual QA projection is later/profile scope. |
| [Manual QA Card](manual-qa-card.md) | Manual QA prompt/card | v0.3 agency assurance profile | Displays QA requirement/waiver refs; does not record QA | Full Manual QA card is later/profile scope. |
| [Verification Result Card](verification-result-card.md) | Verification/Eval display | v0.3 agency assurance profile | Displays Eval/gate refs; does not verify by itself | Compact assurance display when verification profile is active. |
| [RUN-SUMMARY](run-summary.md) | Diagnostic run report | Future/diagnostic or owner-promoted profile | Derived display over Run/artifact refs | Not required for v0.2. |
| [EVIDENCE-MANIFEST](evidence-manifest.md) | Detailed evidence report | Future/diagnostic or owner-promoted profile | Displays evidence records and artifact refs; not evidence itself | v0.2 card shows evidence summary/gaps only. |
| [EVAL](eval.md) | Detailed verification report | Future/diagnostic or owner-promoted profile | Displays Eval refs; does not create assurance | Detailed Eval is not v0.2. |
| [TDD-TRACE](tdd-trace.md) | TDD diagnostic/reference | Future/diagnostic or owner-promoted profile | Displays TDD refs; not a gate by itself | Later policy/profile output. |
| [DOMAIN-LANGUAGE](domain-language.md) | Stewardship/reference report | Future/diagnostic or owner-promoted profile | Displays `domain_terms`; not term authority | Later reference view. |
| [MODULE-MAP](module-map.md) | Stewardship/reference report | Future/diagnostic or owner-promoted profile | Displays `module_map_items`; not module authority | Later reference view. |
| [INTERFACE-CONTRACT](interface-contract.md) | Stewardship/reference report | Future/diagnostic or owner-promoted profile | Displays `interface_contracts`; not contract authority | Later reference view. |
| [DESIGN](design.md) | Design/reference report | Future/diagnostic or owner-promoted profile | Displays design records/proposals; not design authority | Later standalone projection. |
| [JOURNEY-CARD](journey-card.md) | Journey/resume diagnostic card | Future/diagnostic or owner-promoted profile | Derived current-position display only | v0.2 uses compact status card instead. |
| [EXPORT](export.md) | Operations/export report | v0.4 operations/export profile | Lists snapshots and artifact refs; not Core state or artifact authority | Optional handoff/report output. |

`Future/diagnostic projections` means later-profile or diagnostic scope, not automatically v1+ only.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are readable views from owner records and refs. They must not redefine kernel fields, MCP schemas, SQLite DDL, gate behavior, or artifact integrity rules.

Rendered placeholders, labels, table columns, and example front matter keys are template bindings for display. A binding must either show an existing owner record field or ref, or be a derived display summary from the source records named by the template. If the source record or ref does not exist, render `none`, `unknown`, `not_required`, or an unavailable/blocking note instead of inventing state.

Compact authority displays should prefer a short refs line when several sources are relevant: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, and `freshness=`. These labels point to existing refs, redaction state, and projection freshness only; they are not schema fields or authority records.

Derived display summaries include approval boundary lines such as `approval_covers`, `approval_does_not_cover`, and `secret_exposure_boundary`; close context, close blockers, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, and judgment-context-related summaries. These names are not new canonical records, schema fields, DDL columns, `ProjectionKind` values, gates, authority inputs, or authority paths. The labels themselves must not be used as validator inputs; validators consume the owner records, refs, gates, artifacts, or Decision Packets those labels summarize.

Rendered examples should make that boundary visible to the reader. `source_state_version` names the state clock used for the render, `projection_version` or projection status names the render/template/job view, and `updated_at` names when the view was produced. Freshness lines say whether the view still matches its source records; they are not task results, gate values, approval, acceptance, evidence, close readiness, or Core state rollback.

Managed blocks are projector-owned display. Direct edits inside managed blocks are drift and should become reconcile candidates, not state changes. Human-editable sections such as `User Notes and Proposals` are proposal surfaces: they become state only through proposal -> reconcile item -> accepted Core state-changing action with the relevant `state.sqlite.task_events` row, or they remain rejected, deferred, or note-only content.

Any template that renders artifact refs must preserve `redaction_state`. Large logs, diffs, traces, screenshots, bundles, recordings, and sensitive artifact bodies are referenced by `ArtifactRef`, not embedded by default. `secret_omitted` entries may show safe notes or handles and may support only visible nonsecret evidence; `blocked` entries show the committed metadata-only notice as unavailable input. Templates must not inline, reconstruct, summarize, or export omitted secret/PII values or blocked raw payload bytes.

Display fields such as `redaction_availability_summary`, omitted or blocked impact lines, and `Downstream Effect` columns are rendered summaries only. They are derived from `ArtifactRef.redaction_state`, owner records, and downstream gate, evidence, QA, verification, projection, export, or Release Handoff status.

Decision Packet visibility does not depend on a standalone `DEC` Markdown projection. Required surfaces can show active Decision Packets through the compact status card, status/next responses, judgment-context resources, decision-packet resources, or a dedicated prompt. `TASK` may also show them when a later continuity profile is active. Standalone `DEC` is only an optional rendered view when that projection is enabled.

Decision Packet displays may include canonical schema fields and reader-facing shape fields such as decision title, `judgment_category`, `judgment_route`, `display_depth`, why this is needed now, what the user is judging, concise options or detailed trade-offs, recommendation, uncertainty, deferral consequence, and residual risk when relevant. `judgment_route` is the owner path and recorded-answer route. `display_depth` is schema-owned prompt depth; validators use it with `judgment_route` to validate the matching `judgment_payload` and route-specific required fields. Values are `simple`, `tradeoff`, `high-risk`, and `close-affecting`. `judgment_category` is a schema-owned enum with values `product_ux`, `technical_architecture`, `security_privacy`, `scope_autonomy`, `qa_verification`, `work_acceptance`, `residual_risk`, or `mixed`; validators should validate the enum, while templates may render those values as Product / UX, Technical architecture, Security / privacy, Scope / autonomy, QA / verification, Work acceptance, Residual risk, or Mixed. If a judgment is cross-cutting, templates should render secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up instead of treating the category as exclusive. Friendly labels derived from `display_depth` or `judgment_category` help readers, but they are not schema fields, `ProjectionKind` values, gates, owner records, validator inputs, close aggregation rules, authority paths, or replacements for `judgment_route`.

Display cards should distinguish three different problems: a stale projection means the readable view may lag behind its source records, stale state or stale evidence means the underlying state, baseline, or artifact inputs have moved or become insufficient, and MCP unavailable means the surface cannot reach the required Harness/Core capability. Only the owner records and Core transitions can change state.

Close and assurance displays must keep distinct labels for self-checked work, `detached_verified` assurance, waived verification, QA waiver, and residual-risk accepted `completed_with_risk_accepted` close. They may appear in the same compact card, but should not be collapsed into "done," "verified," or "accepted" without the owner refs that support each state.

## Future/Diagnostic Projection Templates

- [DESIGN](design.md)
- [DIRECT-RESULT](direct-result.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TASK](task.md)
- [TDD-TRACE](tdd-trace.md)

## Core Status Output

- [Compact Status Card](compact-status-card.md)

## User Judgment Prompt Shapes

- [Decision Packet user judgment request display shape](decision-packet.md), not standalone `DEC` Markdown

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
