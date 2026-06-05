# Later-Profile Template Catalog

## Used when

Use these templates only when a later owner profile, diagnostic path, Assurance Profile, Operations Profile, export/release-handoff path, or explicit drill-down needs a detailed rendered body. They are not MVP-1 requirements.

The MVP-1 compact set is limited to four user-facing outputs, [status-card](../status-card.md), [judgment-request](../judgment-request.md), [run-evidence-summary](../run-evidence-summary.md), and [close-result](../close-result.md), plus one agent-facing packet, [agent-context-packet](../agent-context-packet.md).

Authority rule: all templates in this folder are rendered views only. They do not create state, evidence, approval, final acceptance, residual-risk acceptance, QA, verification, Write Authorization, close readiness, or close.

## Output tiers

| Tier | Templates | Rule |
|---|---|---|
| Full judgment and sensitive-action displays | [DEC / Decision Packet](decision-packet.md), [APR](approval.md), [Approval Card](approval-card.md) | Use for complex/later-profile judgment or committed Approval displays. MVP-1 uses [judgment-request](../judgment-request.md). |
| Detailed evidence, run, and verification reports | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [Verification Result Card](verification-result-card.md) | Use when the corresponding evidence, Eval, or assurance profile is active. MVP-1 uses [run-evidence-summary](../run-evidence-summary.md). |
| Manual QA and assurance displays | [MANUAL-QA](manual-qa.md), [Manual QA Card](manual-qa-card.md) | Use when a Manual QA profile is active. |
| Continuity and diagnostic reports | [TASK](task.md), [DIRECT-RESULT](direct-result.md), [JOURNEY-CARD](journey-card.md), [DESIGN](design.md) | Use for later continuity, diagnostic, or full-report views. MVP-1 uses only the four user-facing root outputs plus the agent-facing root packet: [status-card](../status-card.md), [judgment-request](../judgment-request.md), [run-evidence-summary](../run-evidence-summary.md), [close-result](../close-result.md), and [agent-context-packet](../agent-context-packet.md). |
| Stewardship/reference reports | [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [TDD-TRACE](tdd-trace.md) | Use when owner profiles promote stewardship, TDD, or reference projections. |
| Operations/export reports | [EXPORT](export.md) | Use only when the Operations Profile export or release-handoff owner path is active. |

## Template implementation classes

Later/full-profile templates are pull-on-demand. They must not be loaded into default agent context and must not be treated as a stage checklist.

When a later profile is inactive, root MVP-1 views should show the relevant compact summary, ref, absence, blocker, or unavailable note instead of pulling a detailed template body.

## Notes

Files in this folder retain their detailed bodies for future owner profiles. Their presence preserves design material; it does not expand Engineering Checkpoint or MVP-1 scope.
