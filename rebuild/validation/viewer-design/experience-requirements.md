# Provisional Viewer experience requirements

Status: technically exercised disposable target; qualitative acceptance **pending**.
This is a review composition, not an active Product contract, framework decision,
production implementation plan or approval to migrate. The exact artifact and
current feedback and observations are in [responsive-review.md](responsive-review.md);
the earlier [journey review](journey-review.md) remains historical evidence.

The [Product charter](../../docs/design/product-charter.md),
[decisions](../../docs/design/open-decisions.md),
[architecture](../../docs/design/architecture.md),
[domain](../../docs/design/domain-model.md),
[projection owner](../../docs/design/projections-and-documents.md),
[privacy owner](../../docs/design/privacy-and-provider-boundary.md),
[acceptance scenarios](../../docs/design/acceptance-scenarios.md) and
[validation owner](../../docs/design/validation-plan.md) retain authority.
In particular, the active projection owner requires opening critical limits and
independent check/review/acceptance facts. This trial's explanation-first placement
and shortcut do **not** replace that contract. Future adoption must resolve that
specific difference and obtain the applicable human review before implementation.

## Outcomes and composition

| Destination / task | Provisional composition and required behavior | What remains to integrate or decide |
| --- | --- | --- |
| Overview: understand the Project and find Work | Concise navigation, attributed Purpose when present, current/completed/remaining classifications, next steps and exact omissions; readable Work title, state and summary, one reading action | The trial's A/B/C historical catalog and charter quotation are authored review material, not the production Overview adapter. Preserve canonical Purpose absence; never infer Purpose from a Goal. Category bounds, unanswered Questions and gaps must use the maintained projection. |
| Work: explain problem and mechanism | Document title and attributed lead, problem, mechanism, conditional observations and recorded next direction; full original Goal and provenance through native disclosures | Preserve complete canonical facts, historical failures, limits and original quotations. Generated interpretation and canonical result/state/verification/review/acceptance are separate. Critical opening information needs human review and owner resolution; a completed historical Work cannot label a distinct open control completed. |
| Compare conditions and results | Compact semantic table quoting the admitted command observations, followed by the full explanations and citations. Each row reaches its exact explanation; conditions do not imply time sequence or a patch | This trial quotes only three independently retained Click stdout rows. Arbitrary condition tables need supported structured input; do not extract an authoritative result from generated prose or invent before-states. Full paragraphs remain present, so the table adds document length. |
| Scoped Code: choose relevant evidence | Choose a purpose from a specific claim; no arbitrary first Source. Desktop shows a nonmodal right-side evidence panel beside the original document and keeps the selected paragraph visible. Work name, exact paragraph, locator/range, code and provenance remain. At 900px and below it uses a full-screen dialog; code is 13px through 900px and 12px at 600px and below | The desktop sidebar and smaller mobile code follow explicit user feedback. Desktop document controls remain interactive, without a modal focus trap or backdrop. Long lines still scroll horizontally; smaller text does not guarantee every line fits. Exact Work/claim/Source identities must come from the maintained projection. |
| Return and keyboard | Escape or the visible return link restores the same paragraph and saved scroll. History, direct Source fragments, direct Goal/audit fragments, mobile focus containment/background inertness work; desktop permits focus to move between document and panel. Long quotation/code regions are keyboard focusable | Real screen-reader behavior and native tab zoom remain unobserved. Do not claim accessibility conformance from DOM and keyboard checks alone. |
| Missing/degraded explanation or Source | Keep canonical Goal/state and independent facts readable. Show the specific missing/unavailable state; never fill an explanation or select unrelated repository code. Missing before-state means no diff. Preserve usable historical information with its actual scope | Goal-only fixture has no Work-bound code, result, Checkpoint verification or direction. Its code task correctly remains unavailable. B/Click admission rejects unsupported/stale/withheld/incomplete readbacks; this trial does not implement every production lifecycle screen. |
| Source/provenance disclosure | Retain full Goal, selected-Work audit, exact identities/revisions, source byte/line coordinates, omissions, before-state availability and history. Ordinary prose stays attributed | Captured Current describes a historical readback, not current source freshness or renewed retention. No raw source acquisition, generation, canonical write or background transmission on navigation. |
| Offline review | Viewer pages and their sources/scripts form a closed `viewer/` subtree; design-review tasks, measurements, manifests and baseline comparisons are outside its navigation. Package-relative files, closed fragments, no remote assets, script-free documents with ordinary citation/return links and visible original records. Python-only loopback preview must survive connection timeouts | Review ZIP can be moved and extracted. Source inputs and metadata remain private. Developer browser tooling and build-time Pygments are not preview dependencies. No public hosting is authorized or required. |

The future adapter must preserve Overview → Work → scoped Code/Decision → evidence
without inheriting this trial's Python builder, duplicated citation markup, CSS
layering, original HTML hierarchy or a Rust/Vue framework choice. UI state consists
of selected Project/Work, claim, exact Source and return origin; it confers no write
authority. Operations/Projections remain the read boundary specified by architecture.

## Other maintained destinations

| Destination | Required integration boundary | Current trial coverage |
| --- | --- | --- |
| Decisions | Exact selected Work association, Question/choice/reason/applicability/review state, code linkage and evidence; unrelated Decisions must not be inferred from matching paths | Baseline destination remains in the separate review material. B/Click have no related Decision; no rationale is invented. A populated Decision journey is still required before adoption. |
| Repository Map / Code | Keep repository-wide inventory distinct from a claim's Work-scoped Source. Real relations, endpoints, capability, freshness and exact omissions remain inspectable | Baseline Map remains in the separate review material. The Goal-only case cannot borrow a repository function as Work evidence. New map composition is unimplemented. |
| Analysis | Show coverage, failed/unsupported regions, latest attempt versus retained results and freshness independently | Baseline Analysis/diagnostics remain in the separate review material; no new analysis composition or execution. |
| Documents | Preserve draft/preview/grounding/adoption boundaries and source-independent output metadata | Baseline Documents remain in the separate review material; no new document-generation/adoption interaction. |
| Privacy / diagnostics / export | Deeper inspection stays reachable without taking over ordinary reading; preserve existing authority and local read-only boundaries | Original evidence/audit remains in Viewer; baseline diagnostics are in the separate review material. These operations are not implemented by the trial. |

## Review and promotion

Use the answer-free tasks in `review/journeys.html`, alternating baseline/target
order and repeating desktop/mobile. Record exact manifest, width, case, actions,
actual misunderstanding, return failure and preference. Human timing, comprehension
and errors remain null until observed. The source-first controls are bounded
assertions, not a blind independent semantic review. No model score replaces H1/H2/H3.

The user selected simultaneous document/evidence reading on desktop, smaller mobile
code, and separation of review material. Review still needs to assess the exact
revised artifact, opening-limit balance, condition-table usefulness and multi-Work
navigation beyond the small fixture. Approval applies only to the reviewed bytes and stated scope. Silence,
historical prototype direction, agent visual judgment and test success confer no
approval. Production promotion still follows validation-plan §17 and applicable
Product acceptance; no frontend technology is selected here.
