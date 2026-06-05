# Later: Future Fixtures

## What this document helps you do

Use this page as a compact inventory of future fixture scenario families. It preserves design knowledge that may be useful after MVP-1 without turning that knowledge into an MVP requirement, a runnable conformance suite, a required fixture file set, or a server implementation plan.

This is future design documentation only. It is not an MVP-1 requirement, not implemented runtime behavior, not active API or DDL, and not current conformance. The current repository is documentation-only and contains no runnable Harness Server conformance tests, generated conformance artifacts, executable fixture catalog files, server implementation, or runtime state. Current phase and handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

The old long pseudo-fixture payloads have been intentionally removed. Do not reconstruct scenario scripts here merely for completeness. If a future owner promotes a scenario, the exact behavior and exact fixture body belong in the owner Reference path named below.

## Catalog Boundary

[Conformance Fixtures Reference](../reference/conformance-fixtures.md) owns the core conformance model, active MVP structured fixture drafts, exact structured fixture draft shape, future runner behavior, assertion semantics, fixture profiles, current-phase status, and the narrow Engineering Checkpoint Kernel Smoke authoring queue.

This catalog owns only future scenario inventory. A row here is a candidate family, not a fixture body, public request schema, storage schema, DDL row, stage exit criterion, generated artifact, runtime result, or implementation task. Catalog rows must not be cited as proof that a behavior exists, that an API is active, that a table must be implemented now, or that MVP-1 scope has expanded.

Projection output may be mentioned as a later display to inspect, but projection output is never conformance truth. Promotion must prove behavior through Core-owned state, owner records, events, artifacts, errors, and the relevant owner contracts.

## Promotion Criteria

Future scenario families may become active only through an owner promotion path. Promotion has two possible targets:

| Promotion target | Minimum criteria before promotion |
|---|---|
| Active behavior example | The owner document names the behavior, delivery stage or profile, user-visible outcome, affected owner records, fallback behavior, security or guarantee wording, and non-claims. The example stays explanatory unless the owner also materializes an executable fixture. |
| Runtime conformance case | The behavior has an accepted implementation-planning scope, exact API and storage owners, exact structured fixture body fields, seed-state expansion rules where needed, runner behavior, assertion semantics, request/response observations, storage/event/artifact/blocker/error observations, and forbidden-side-effect assertions. It must prove Core state and owner records, not rendered prose alone. |

Every promotion must also:

- state whether the target is Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, or Roadmap;
- identify the owner Reference section for exact schemas, DDL, security wording, or fixture shape;
- remove or narrow the catalog row if it would otherwise duplicate the promoted owner;
- mirror the same meaning in the Korean document;
- keep unsupported or weaker surfaces honest through fallback or reduced-guarantee wording.

## Bucket Map

Use this map before reading scenario families:

| Bucket | Scenario material parked here | Promotion route |
|---|---|---|
| Assurance Profile | Verification strengthening, Manual QA, detailed evidence, risk review, design-quality, stewardship, TDD trace, feedback-loop, and context-hygiene families when they support assurance behavior. | [Assurance Profile](assurance-profile.md), then the relevant Reference owner. |
| Operations Profile | Export, recovery, handoff, artifact integrity, projection refresh, reconcile, operator readiness, `doctor`/readiness, and future conformance-run families. | [Operations Profile](operations-profile.md), then [Operations And Conformance Reference](../reference/operations-and-conformance.md). |
| Roadmap | Dashboard, hosted workflows, team workflows, broad connectors, Browser QA Capture automation, Cross-Surface Verification automation, remote/shared MCP, preventive guard expansion, hooks, orchestration, metrics, and other expansion candidates. | [Roadmap](../roadmap.md) promotion criteria. |

Catalog entries can appear under the closest technical concern, but the bucket above controls stage interpretation. Listing a family here does not make it stage-required.

## Catalog-Only Future Families

These families are deliberately parked outside Engineering Checkpoint and MVP-1 User Work Loop.

| Future family | Catalog boundary |
|---|---|
| Full Manual QA | Full policy matrices, browser/manual capture expansion, QA waiver detail, and QA dashboards stay future or Assurance Profile scope unless a narrower owner path is promoted. |
| Eval systems and detached verification automation | Cross-surface evaluator orchestration, detailed Eval reports, independence hardening, and assurance upgrades stay future or Assurance Profile scope. MVP-1 must only avoid claiming detached verification unless a compatible verification record exists. |
| TDD trace and feedback-loop policy | RED/GREEN trace, feedback-loop execution policy, and policy-specific test-path scenarios stay future or Assurance Profile scope. |
| Module map and interface contract | Domain, module, and interface stewardship scenarios remain candidates until owner docs promote exact records and validators. |
| Journey, Spine, and detailed report projections | Journey Card, Journey Spine, Run Summary, detailed Evidence Manifest, detailed Eval, and polished report projections are derived-output candidates. They do not become state or MVP-required projection kinds. |
| Export, recover, release handoff, and artifact-integrity operations | Export/recover, release handoff, retention, redaction export, and artifact check scenarios stay Operations Profile or later scope unless promoted. |
| Dashboard, team workflow, and orchestration | Hosted UI, dashboards, shared/team workflow, permission, parallel-lane, and orchestration scenarios stay Roadmap candidates. |
| Advanced connector and security behavior | Broad connector ecosystems, remote/shared MCP, browser capture automation, preventive guards, isolated profiles, hooks, sidecars, and higher security claims require owner-defined mechanisms and fixture proof for the covered operation before promotion. |

<a id="staged-fixture-coverage"></a>
<a id="fixture-example-map"></a>

## Scenario Family Inventory

The sections below replace the old staged coverage map and long fixture example payloads. They preserve family names and intent only. They are not a checklist of files to create.

<a id="intake-and-decision-catalog-entries"></a>

### Intake And Decision Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Natural-language intake and plain routing | Harness-shaped work can begin or resume without a startup phrase, and ordinary user language maps to the compatible Task, scope, user judgment, or next safe action. | Promote only through the Core/API intake owner, with explicit non-authorization wording for product writes. |
| Tiny direct without authority bypass | Very small obvious work may stay in Direct mode without introducing a `tiny` mode or bypassing scope, user judgment, sensitive-action permission, or Write Authorization. | Promote only after the active Direct profile defines the narrow behavior and escalation path. |
| Codebase-answerable before user question | Current refs and provided facts are used before asking the user to repeat information, while unresolved product or material technical decision still routes to the user. | Promote with context-source freshness rules owned by Agent Integration or Core owners. |
| User judgment quality and separation | Product decision, technical decision, sensitive-action permission, Manual QA, final acceptance, and residual-risk acceptance remain distinct. | Promote through user judgment and gate owners; do not reintroduce broad approval. |
| Residual risk visibility before acceptance or close | Known close-relevant residual risk must be visible before acceptance or close. `ResidualRiskSummary.status=none` is valid only when no known close-relevant risk exists. | Promote through Core Model and residual-risk owners before any executable fixture is written. |

<a id="core-fixture-examples"></a>

### Core, Evidence, Verification, And Close Families

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Write scope and Write Authorization lifecycle | No active Change Unit blocks writes; compatible `prepare_write` creates a durable Write Authorization; missing, consumed, stale, or violated authorizations block or stale dependent claims. | Promote through [Core Model Reference](../reference/core-model.md) and [MVP API](../reference/api/mvp-api.md). |
| Evidence and close readiness | Direct docs-only work can close only when evidence is sufficient; missing acceptance-criteria support, pending verification, or pending QA blocks close. | Promote through Core and evidence/close owners, not through report text. |
| Detached verification boundary | Manual bundle review, subagent review, same-session review, verification-risk acceptance, and visible accepted risk stay distinct assurance paths. | Promote through Eval/verification owners; same-session self-review must not create detached assurance. |
| Projection failure with current state | Current Core state remains authoritative when a projection is stale, skipped, or failed. | Promote through Projection and Core owners; never let rendered Markdown satisfy gates. |

<a id="artifact-redaction-and-export-non-leakage-catalog-entries"></a>

### Artifact Redaction And Export Non-Leakage Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Secret or PII omitted, visible evidence only | Evidence, QA, Eval, projection, or report views can cite visible nonsecret material while omitted values remain unavailable. | Promote through artifact, evidence, redaction, and export owners. |
| Blocked input as metadata-only notice | Blocked payloads may leave safe metadata and unresolved downstream effects without exposing forbidden bytes. | Promote only after artifact storage and redaction behavior are exact. |
| Untrusted staged URI and task-scoped refs | Arbitrary paths, traversal, symlink escape, or cross-Task artifact relations cannot become trusted artifact evidence. | Promote through storage and ArtifactRef owners. |
| Artifact integrity affects dependent claims | Missing files, missing `sha256` or `size_bytes`, `hash_mismatch`, or owner-link mismatch blocks dependent evidence, QA, Eval, projection, export, or close readiness. | Promote through artifact integrity and operations owners. |
| Export and Release Handoff non-leakage | Exported snapshots and handoff reports show omission/block notes without leaking raw omitted values or blocked payloads. | Promote through Operations Profile, export, and security owners. |

<a id="agency-fixture-examples"></a>

### Agency Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| User judgment before product or technical trade-off writes | Writes that depend on user-owned product or material technical decision are held until a compatible judgment exists. | Promote through user judgment, Change Unit, and `prepare_write` owners. |
| Sensitive-action permission is not judgment or close | Approval-shaped permission does not satisfy product decision, evidence, verification, QA, final acceptance, residual-risk acceptance, or close. | Promote with exact separation between MVP sensitive-action judgment and later Approval profile records. |
| AFK Autonomy Boundary stop conditions | AFK or high-autonomy work stops or routes to judgment when product, public API, security, privacy, or other stop conditions are triggered. | Promote through agency and connector capability owners with honest guarantee wording. |
| Acceptance and residual-risk sequencing | Final acceptance and residual-risk acceptance are separate user judgments, and known close-relevant risks must be visible first. | Promote through close, acceptance, and residual-risk owners. |

<a id="connector-fixture-examples"></a>
<a id="connector-agency-catalog-entries"></a>

### Connector Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Guarantee display and capability honesty | A connector reports cooperative, detective, preventive, or isolated guarantees only at the level it can actually support. | Promote through Agent Integration and Security owners; preventive claims require fixture-proven pre-action blocking. |
| MCP unavailable or capability mismatch holds unsafe writes | Missing MCP, stale capability profile, missing artifact capture, missing QA capture, weak redaction, or weaker guard capability holds affected write or close-relevant paths. | Promote through API error precedence and connector capability owners. |
| Generated file or managed instruction drift routes to reconcile | Generated connector files and managed blocks are detected and routed to reconcile without silently rewriting owner records. | Promote through Projection/Reconcile and connector manifest owners. |
| Current-position context before significant resume | Resume reads current Task state, refs, pending judgments, residual risk, and projection freshness before producing an instruction bundle. | Promote through context-push/pull profile owners. |
| Guard, freeze, and careful mode do not create authority | These labels may narrow behavior or hold writes, but they do not create a stronger guarantee, Write Authorization, Approval, verification, QA, final acceptance, residual-risk acceptance, close, or assurance upgrade by themselves. | Promote only with exact surface capability proof and fallback behavior. |
| Local-only MCP and local security posture | Non-loopback, forwarded, tunneled, unauthenticated, or weak local exposure is reported honestly and does not create authority. | Promote through Security and Operations owners. |

<a id="design-quality-fixture-examples"></a>
<a id="stewardship-fixture-examples"></a>
<a id="stewardship-catalog-entries"></a>

### Design-Quality And Stewardship Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Shared Design required and continued while unknowns remain | Ambiguous work keeps design shaping open until goals, non-goals, acceptance criteria, affected flow, module/interface impact, verification, QA, and risks are inspectable or routed to user judgment. | Promote through Design Quality and Shared Design owners. |
| Codebase-answerable stewardship facts first | Module ownership, domain language, public interface impact, affected paths, and test/QA affordances already present in current refs are used before asking the user. | Promote with source freshness and owner-record rules. |
| Horizontal exceptions, feedback loops, and TDD trace | Horizontal exception reasons, behavior feedback loops, RED/GREEN traces, and non-test write guards become assurance checks only after policy owners define exact behavior. | Promote through Design Quality Policies, not this catalog. |
| Manual QA required or waived through owner paths | Manual QA requirement, waiver reason, product-risk waiver judgment, and QA gate effects remain separate from final acceptance and verification. | Promote through Manual QA and user judgment owners. |
| Public interface, module, and domain language stewardship | Public boundary changes, interface-contract review, future-change risk, and domain-language conflicts route through owner records and close blockers. | Promote through stewardship, module map, and interface contract owners. |
| Findings route to existing owner paths | Run, Eval, Manual QA, and design-quality findings affect evidence, user judgment, feedback loop, Manual QA, Eval, residual risk, validator results, gates, or close blockers without creating a new finding schema here. | Promote only in the relevant owner contract. |
| Review Stage display is not authority | Spec Compliance Review and Code Quality / Stewardship Review can be displayed separately, but display text cannot close work, accept risk, create evidence, satisfy QA or verification, create Approval, or create Write Authorization. | Promote through projection/display owners with Core non-substitution rules. |

<a id="context-hygiene-fixture-examples"></a>
<a id="context-hygiene-catalog-entries"></a>

### Context Hygiene Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Stale PRD, stale projection, or old design doc is pull-only | Stale context may point to refs worth inspecting, but it cannot replace current Task state, acceptance criteria, Change Unit scope, product decision, or gate state. | Promote through context hygiene, projection freshness, and Core owners. |
| Resume uses current state, not chat memory | Significant resume uses Core state, current-position refs, evidence refs, active user judgments, residual-risk summary, and projection freshness instead of stale chat memory. | Promote through Agent Integration and context profile owners. |
| Compact context by phase | Always-on context stays refs-first, current, one screen or less, and profile-relevant; full docs, schemas, logs, artifact contents, and future catalog material remain pull-on-demand. | Promote through Agent Integration Reference. |
| Retrieved or indexed context is non-authority | Search, memory, or indexed context can supply refs or excerpts but cannot authorize writes, satisfy gates, accept work, accept risk, update projection freshness, or close tasks. | Promote only after Context Index or equivalent Roadmap owner is promoted. |
| Evaluator bundle freshness | Verification bundles must be current enough for the asserted evaluation and cannot set detached verification passed when material context is stale or missing. | Promote through Eval/verification owners. |

<a id="core-projection-reconcile-and-verification-boundary-catalog-entries"></a>

### Core, Projection, Reconcile, And Verification Boundary Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Current state versus stale projection | Core state stays readable and authoritative while projections are stale or failed; close/readiness cannot infer readiness from stale Markdown. | Promote through Projection and Core owners. |
| Managed-block edits route to reconcile | Human edits inside managed blocks or generated output produce reconcile candidates and leave owner records unchanged until accepted through Core. | Promote through Projection/Reconcile owners. |
| Same-session self-review is not detached verification | Same-session review may be useful context but cannot satisfy detached verification or upgrade assurance. | Promote through Eval/verification owners. |

<a id="operations-profile-catalog-entries"></a>

### Operations Profile Catalog Entries

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Release Handoff does not close or deploy | Handoff reports may summarize close readiness, blockers, evidence refs, verification refs, Manual QA refs, residual-risk refs, changed files, projection freshness, artifacts, and advisory checklists without mutating Task state or deploying. | Promote through [Operations Profile](operations-profile.md) and export/handoff owners. |
| Export, recover, and artifact integrity | Export/recover operations report retention, redaction, integrity, and availability without silently repairing state or widening secret access. | Promote through Operations, Storage, ArtifactRef, and Security owners. |
| Doctor and readiness diagnostics | `doctor`, `connect`, `serve mcp`, readiness, and future conformance-run entrypoints report operator posture without implying early-stage requirements. | Promote through Operations And Conformance and Security owners. |

<a id="roadmap-browser-qa-capture-candidate-entries"></a>

### Roadmap Browser QA Capture Candidate Entries

Browser QA Capture is a Roadmap candidate, not an Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, or Kernel Smoke requirement. It becomes executable only after the capability profile, redaction and secret/PII policy, test environment, artifact retention, conformance target, fallback semantics, and no projection-as-canonical dependency are defined.

| Scenario family | Later capability it would test or illustrate | Promotion notes |
|---|---|---|
| Browser capture artifacts attach to Manual QA | Screenshots, QA capture, logs, console logs, network traces, accessibility snapshots, or workflow recordings support a Manual QA record when the surface can capture them. | Promote through Browser QA Capture and Manual QA owners. |
| Capture is not final acceptance or detached verification | Browser artifacts can support evidence but do not replace human Manual QA judgment, final acceptance, residual-risk acceptance, or detached verification. | Promote through QA, acceptance, residual-risk, and Eval owners. |
| Unsupported surface falls back to human notes | A surface without capture support reports missing capability and recommends human Manual QA notes or manually supplied artifacts without failing staged delivery solely for lacking automation. | Promote through connector capability and fallback owners. |

<a id="agency-stewardship-context-and-design-quality-suites"></a>

## Agency, Stewardship, Context, And Design-Quality Suites

Agency, stewardship, context hygiene, and design-quality remain catalog-only Assurance Profile suite candidates until owner docs promote them. If promoted, they must test response facts, Core state, storage rows, events, artifacts, blockers, errors, and forbidden side effects through Core entrypoints or operator actions that call Core. They must not pass by matching Journey Card, user judgment, residual-risk, review-stage, status, or report prose.

Status and `next` recommendations, including Role Lens and Browser QA recommendations, are observable only as read responses unless a later public mutation records an owner record. A recommendation alone must not mutate state, satisfy a gate, enqueue a projection, create evidence, record verification, record QA, accept work, accept residual risk, close a Task, or upgrade assurance.

<a id="catalog-only-fixture-skeleton-guidance"></a>

### Catalog-Only Fixture Skeleton Guidance

This catalog no longer carries fixture skeletons. Future exact-shape fixture materialization belongs in [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and the relevant API, Storage, Core, Security, Projection, Operations, Agent Integration, or policy owner. Delivery-stage mapping belongs in suite metadata or Build owners, not in catalog prose.

<a id="later-profile-fixture-shorthand-notes"></a>

### Later-Profile Fixture Shorthand Notes

Later-profile shorthand is a planning convenience only. It is not active for Engineering Checkpoint or MVP-1 User Work Loop, not an executable runner contract, and not a second API. Before any future fixture becomes executable, shorthand must expand to owner records, validator runs, residual-risk records, or other state explicitly owned by DDL/API docs. It must not create fixture-only storage rows or alternate request payload branches.

<a id="fixture-suites"></a>

## Fixture Suites

Future suite names are planning labels, not a required file set. They group inventory families under fixture profiles in [Conformance Fixtures Reference](../reference/conformance-fixtures.md#fixture-profiles-by-proven-behavior) only after promotion.

| Suite label | Inventory boundary |
|---|---|
| `core` | Write scope, Write Authorization, evidence, close readiness, verification boundary, residual-risk visibility, and projection failure separation beyond the minimal Engineering Checkpoint subset. |
| `connector` | Natural-language routing, capability honesty, MCP availability, generated-file drift, artifact capture fallback, current-position resume context, and local security posture. |
| `artifact-redaction` | Registered artifact boundaries, redaction/blocked metadata, task-scoped refs, integrity checks, and export or handoff non-leakage. |
| `connector-guard-freeze` | Cooperative/detective guard and freeze behavior, careful-mode non-authority, capability mismatch honesty, and preventive claims only when surface-specific proof exists. |
| `agency` | User judgment quality, user-owned trade-off guards, AFK stop conditions, Approval separation, acceptance sequencing, and residual-risk visibility. |
| `stewardship` | Shared design, codebase-answerable facts, feedback loops, TDD trace, public interface review, domain language, findings routing, managed-block reconcile, and review-stage non-authority. |
| `context-hygiene` | Compact current context, stale projection/PRD/chat handling, retrieved context non-authority, evaluator bundle freshness, and resume from Core state. |
| `design-quality` | Policy-pack smoke coverage that composes existing validators and gate behavior without redefining kernel authority, duplicating validator IDs, hiding lower-severity findings, or adding new gates. |
| `operations` | Export, recover, handoff, artifact integrity, readiness, diagnostics, and future conformance-run entrypoints after Operations Profile promotion. |
| `browser-qa-capture` | Roadmap-only capture automation, artifact mapping, Manual QA attachment, detached-verification boundary, final-acceptance boundary, and unsupported-surface fallback. |

## Retired Detail Boundary

Pseudo-fixture YAML, long scenario scripts, detailed assertion payloads, renderer-output expectations, and future runner output requirements were removed from this catalog. Active MVP structured drafts now live in Conformance Fixtures Reference; reintroduce future-profile detail only in an owner Reference document after promotion, and only when it is needed to prove a specific active behavior or runtime conformance case.
