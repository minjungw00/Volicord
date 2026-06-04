# Authoring Guide

## What this document helps you do

Use this guide when you add, rewrite, split, rename, or review Harness documentation.

It helps you keep the current documentation readable for the intended reader, clear about where each kind of detail belongs, and aligned between English and Korean.

This is maintenance documentation. It governs documentation maintenance only. The first runnable target is Engineering Checkpoint, an internal authority-loop smoke that is not the product MVP. Kernel Smoke is only a narrow future smoke-check authoring label under that checkpoint. The first user-value target is MVP-1 User Work Loop. Assurance Profile and Operations Profile harden agency assurance, operations, and handoff behavior later, and Roadmap remains future scope unless owner docs promote and prove it.

## Read this when

- You are adding, splitting, renaming, or reviewing documentation.
- You need to decide which document owns a strict contract.
- You are checking English/Korean parity, links, TODO hygiene, or duplicate owner text.

## Before you read

For exact runtime contracts, use the Reference owner documents linked below. For Korean wording rules, use [Translation Guide](translation-guide.md).

For redesign triage categories and the rule that old structure and old prose do not need to be preserved, use [Rewrite Plan](rewrite-plan.md).

## Main idea

Keep each document useful for its reader and keep exact contracts in their owner Reference docs. The docs are source material for understanding and implementing Harness; they are not runtime objects governed by Harness.

## One owner rule

Every normative contract has one owner document. The owner is the only place to define exact fields, enum values, DDL, schemas, algorithms, state transitions, gate rules, fixture body shapes, template bodies, storage rules, security guarantees, error precedence, and official definitions.

Other document families may name the reader-facing consequence of a contract and link to the owner, but they must not create a second definition. If a local explanation needs a full table, schema block, DDL block, transition matrix, fixture mini-language, gate matrix, validator table, or algorithm, it belongs in the owning Reference document.

Family boundaries:

| Family | Role | Boundary |
|---|---|---|
| Learn | Why Harness exists and what concepts mean. | Do not define strict schemas, gates, or implementation sequences. |
| Use | How users and agents interact. | Include low-level contract detail only when it helps user trust or explains a visible blocker. |
| Build | Implementation sequence and staged delivery plan. | Keep stage goals, sequencing, and exit criteria; link to Reference for exact schemas, gates, DDL, APIs, storage, and fixture details. |
| Reference | Exact contracts, schemas, algorithms, security model, and storage model. | Include enough context to understand the contract, but do not turn Reference docs into tutorials or reader journeys. |
| Maintain | Documentation authoring and review rules. | Govern docs work only; do not define runtime behavior or conformance pass/fail. |

## Reference contract owner map

Use this map before adding a strict rule. If another document needs the same rule, summarize the reader-facing consequence and link to the owner instead of copying the contract.

| Contract area | Owner document | Owner boundary |
|---|---|---|
| Kernel | [Kernel Reference](../reference/kernel.md) | Invariants, entity relationship semantics where they affect state, lifecycle and state transitions, gates, `prepare_write`, Write Authorization, `record_run`, close semantics, waivers, and non-substitution rules. |
| MCP API | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), and [API Schema Later](../reference/api/schema-later.md) | Active MVP-1 tools, public MCP resources, common envelopes, request/response schemas, shared refs, public errors, idempotency/replay, state conflict behavior, `ValidatorResult`, API `ArtifactRef`, and later-profile API material. |
| Storage | [Storage And DDL](../reference/storage-and-ddl.md) | Runtime home layout, persisted state, SQLite DDL profiles, storage-owned JSON `TEXT`, enum hardening, migrations, locks, artifact storage, baseline capture, projection job table, and validator-run storage. |
| Projection | [Document Projection Reference](../reference/document-projection.md) and [Template Reference](../reference/templates/README.md) | Derived view rules, output tiers, managed blocks, human-editable sections, artifact-ref rendering, projection freshness/failure behavior, and full rendered template bodies. |
| Security | [Security Threat Model Reference](../reference/security-threat-model.md) | Threat model, assets, trust boundaries, threat/control categories, high-risk control expectations, local access security posture, and guarantee-level meanings and honest-display rules. |
| Conformance | [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and [Future Fixture Catalog](../reference/future-fixture-catalog.md) | Conformance Fixtures owns the core model, fixture body shape, runner behavior, assertion semantics, fixture profiles, suite metadata boundaries, current-phase status, and reduced Kernel Smoke queue. Future Fixture Catalog owns detailed future scenario candidates, future fixture examples, staged coverage maps, suite family summaries, and catalog-only future candidates. |
| Operations | [Operations And Conformance Reference](../reference/operations-and-conformance.md) | Operator behavior, staged operator surface, diagnostics, `connect`, `doctor`, `serve mcp`, projection refresh, reconcile, recover, export, artifact checks, conformance run entrypoint, and docs-maintenance profile reporting boundary. |
| Agent Integration | [Agent Integration Reference](../reference/agent-integration.md) and [Surface Cookbook](../reference/surface-cookbook.md) | Connector capability profiles, generated manifests, context push/pull profiles, fallback semantics, Role Lens, reference-surface behavior, connector conformance overview, and surface-specific recipes. |
| Glossary | [Glossary Reference](../reference/glossary.md) | Public and internal terminology definitions, capitalization, official term wording, record-name orientation, and owner routing. |
| Runtime Architecture | [Runtime Architecture Reference](../reference/runtime-architecture.md) | The three spaces, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, and architecture-level failure and recovery overview. |
| Design Quality | [Design Quality Policies](../reference/design-quality-policies.md) | Policy contracts, policy-to-validator mapping, stable validator IDs, severity composition, policy waiver semantics, evidence expectations, and design-quality gate/close impact. |

## Current Redesign Scope

### Current Review Baseline

This repository is in documentation review/redesign only. Keep three statuses separate:

- Documentation review status: the current documentation set is in post-redesign review and is a documentation acceptance candidate for maintainer review.
- Implementation planning readiness: not accepted yet. Maintainers must deliberately confirm the implementation-readiness criteria before first runtime-batch planning.
- Runtime implementation status: not started. The repository remains documentation-only. Its intended future role is the Harness Server source repository, but server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision.

No server/runtime implementation decisions have been formally accepted for coding in this repository phase. Open server-coding decision-ledger items are recorded in the MVP Plan and must be accepted or explicitly deferred, with stage impact, before coding the affected behavior.

Do not describe the current docs as fully accepted, implementation-complete, implementation-ready, or ready for server coding unless the maintainer handoff status explicitly defines that acceptance.

Documentation edits may change source docs, but they do not start Harness server/runtime implementation or authorize implementation planning.

Detailed phase/status warnings belong in the root README, language READMEs, Build handoff docs, and this Maintain guidance. Learn and Use pages may link to those owners, but their openings should lead with what users can ask, what the agent should clarify, what Harness preserves, and what users can expect to see.

The redesign may change terminology, staging, schema structure, projection structure, security wording, and document organization. Do not preserve existing prose merely for continuity when it conflicts with the clarified product thesis or implementation feasibility.

Documentation editing in this repository does not require Harness runtime procedures. Do not create runtime state, `task_events`, Write Authorizations, Evidence Manifests, Manual QA records, Acceptance records, Residual Risk records, generated projections, generated operational files, executable fixtures, fixture files, runtime records, or product-repository examples for documentation edits. These terms may be documented as future Harness behavior only.

Documentation files are source material for understanding and implementing Harness. They are not Harness projections unless a future Harness Server explicitly generates them as projections. Do not make documentation pages obey the runtime lifecycle they describe; explain the lifecycle, link to owner contracts, and keep editorial checks editorial.

Path allowlists, language-pair batches, and review batch boundaries for documentation edits are normal maintainer editing controls. They are not Harness runtime override capabilities, Core authorization, Write Authorization, evidence, QA, Acceptance, residual-risk acceptance, close, projections, `task_events`, or runtime state transitions.

### Redesign Editing Contract

During redesign, optimize for clarity, implementability, and the product thesis, not for preserving existing wording.

- Do not keep prose only because it already exists. Rewrite, move, compress, or delete text that makes Harness look like a broad workflow engine, ALM system, evaluation harness, QA automation platform, report generator, or generic MCP wrapper.
- Use [Rewrite Plan](rewrite-plan.md) to classify future edits as `preserve`, `shrink`, `move`, `delete`, or `decision-needed`.
- Preserve the core Harness principles and value proposition: local Core-owned authority for scope, user-owned judgment, evidence references, close readiness, work acceptance, and residual risk outside chat.
- Future, profile-specific, diagnostic, or roadmap material must read as staged or candidate material. It must not look like a current MVP requirement or proof that implementation exists.
- User-facing docs must not require readers to know internal Harness vocabulary before they can understand what to ask, what the agent will clarify, what is blocked, what needs user judgment, or what close means.
- Exact contracts belong in owner Reference docs. Other docs should summarize reader-visible outcomes and link to the owner instead of duplicating schemas, DDL, gates, fixture bodies, projection templates, or state-transition rules.
- Documentation files are not Harness runtime objects. They are not governed by future runtime Write Authorization, Evidence Manifest, `task_events`, Acceptance, residual-risk, projection, conformance, or generated operational-output rules.

### Redesign Backlog Frame

Use this short backlog frame to keep redesign findings small and routable:

- Product definition drift: keep Harness defined as a local authority record and judgment-routing layer, not a prompt pack, workflow engine, report generator, dashboard, or broad hosted agent platform.
- Stage/profile boundary drift: keep Engineering Checkpoint as the internal authority-loop smoke, keep MVP-1 User Work Loop as the first user-value milestone, and keep assurance, operations, diagnostic, and roadmap material outside earlier requirements unless an owner promotes it.
- Judgment model complexity: keep user-owned judgment visible, proportional, and separated from agent judgment, sensitive-action Approval, work acceptance, and residual-risk acceptance.
- Close/verification ambiguity: keep evidence, verification, Manual QA, work acceptance, close readiness, and residual risk distinct. None of them substitutes for another.
- Security guarantee overstatement risk: match cooperative, detective, preventive, and isolated wording to the exact documented mechanism and proof level.
- Context/token overload risk: keep always-on agent context short and current; route detailed contracts to owner docs or retrieval paths.
- User-facing terminology burden: write the user-visible situation first. Do not require users to say internal labels such as Discovery, Change Unit, Decision Packet, Write Authorization, Evidence Manifest, Projection, Gate, or `task_events`. Introduce internal terms only when they help the reader act, interpret a visible blocker, or drill into a reference contract.

## Preserved Principles

Implementation details may change if these principles remain intact:

- Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification, QA expectations, work acceptance, residual-risk status, and close readiness.
- User-owned judgment must be preserved. Product decisions, important technical decisions, QA expectations, work acceptance, waivers, and residual-risk acceptance stay with the user unless an owner contract explicitly says otherwise.
- Evidence, verification, manual QA, work acceptance, and residual risk are separate records and judgments. They must not substitute for each other.
- Chat, Markdown-rendered projections, connector output, and generated documents are not operational truth. Core-owned local state and artifact references are the authority.

When a rewrite changes a term, stage, schema shape, projection shape, security claim, or document boundary, check that these principles still hold before polishing prose.

## Maintainer Handoff Rule

[Implementation Overview: Maintainer handoff summary](../build/implementation-overview.md#maintainer-handoff-summary) owns the short handoff: what the documentation set defines, documentation review status, implementation planning readiness, runtime implementation status, future repository role, preserved principles, current delivery model, implementation-readiness criteria, remaining open-question status, remaining documentation drift status, and maintainer acceptance conditions.

[MVP Plan: Implementation decisions needed before server coding](../build/mvp-plan.md#implementation-decisions-needed-before-server-coding) is the single place for major implementation decisions found during maintainer review or first runtime-batch planning. Do not leave major decisions as scattered `TODO_DECISION` markers in active docs. The current baseline has open server-coding decision-ledger items; entrypoint and handoff status must say that, without treating documentation acceptance as implementation-planning readiness or server-coding authorization. If a future baseline's decision log is empty, say exactly that; do not turn it into a "no open decisions" claim while implementation-readiness criteria still require maintainer judgment. If only editorial cleanup remains, say which docs-maintenance category owns it and why it does not block the current stage.

## Known Redesign Issues Tracker

Use this tracker as the maintainer handoff review checklist for areas that commonly drift. These are maintainer-facing review risks, not open implementation decisions, runtime implementation tasks, runtime conformance, or acceptance records. They do not prove implementation readiness, and they do not authorize server/runtime implementation.

Tracker status meanings:

- Observed in the current docs: this pass or a maintainer review found the drift in active docs. The row should name enough location/context to route the fix.
- Candidate to verify in the current docs: the risk is plausible, but this pass has not proved it is present. Verify before treating it as observed drift or a server-coding blocker.
- Regression-prevention check: current wording is believed acceptable for this baseline, but future edits must not reintroduce the drift.
- Baseline status check: entrypoints and handoff sections must preserve the current repository status.

Do not label an issue "non-blocking" unless the docs name what stage it does not block and what later stage it may block. Do not hide implementation-readiness concerns under vague "follow-up" wording; name the owner, affected stage, and decision or edit needed.

Rows marked "Candidate to verify in the current docs" are not assertions that the drift is currently present. If verification shows a major implementation decision is needed, record it in [MVP Plan: Implementation decisions needed before server coding](../build/mvp-plan.md#implementation-decisions-needed-before-server-coding) with owner doc, affected behavior or field, affected stage, options, and decision needed.

The routing table below is not proof that an item is already blocking. A review risk becomes a documentation drift item, schema/design decision, stage boundary decision, implementation-readiness blocker, or future roadmap item only after verification shows that kind of issue.

Use these item categories when routing confirmed tracker findings or docs-maintenance findings:

| Item category | Use when |
|---|---|
| Documentation drift | The fix is wording, owner-boundary cleanup, link repair, TODO hygiene, terminology, or English/Korean parity. |
| Schema/design decision | The fix requires a real choice in schema, state, API, DDL, security guarantee, fixture semantics, or another owner contract. |
| Stage boundary decision | The fix requires deciding whether a capability belongs in Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, or Roadmap. |
| Implementation-readiness criterion | The item must be true before maintainers accept first runtime-batch planning. |
| Future roadmap item | The item is useful later and remains outside Engineering Checkpoint through Operations Profile unless promoted. |

Potential item category after verification:

| Review risk | Default routing if confirmed |
|---|---|
| Repository identity as the future Harness Server source repository can drift. | Implementation-readiness criterion |
| Stage names can still imply Engineering Checkpoint, Kernel Smoke, or a legacy kernel-stage label is a product MVP or the first user-value slice. | Stage boundary decision |
| User-facing docs may open with heavy implementation disclaimers. | Documentation drift |
| User-facing docs overuse internal terms. | Documentation drift |
| Discovery / requirements clarification may converge too early on a Change Unit or the first safe implementation unit. | Stage boundary decision |
| Legacy judgment alias mapping can drift. | Schema/design decision |
| Decision Packet schema and examples may feel too heavy for small decisions. | Schema/design decision |
| Approval, work acceptance, and residual-risk acceptance are too easy to confuse. | Schema/design decision |
| Storage/DDL can present future-profile tables, fields, or gates as required too early. | Stage boundary decision |
| Conformance fixture docs may be too detailed for the current implementation stage. | Implementation-readiness criterion |
| Operations entrypoints may appear required too early. | Stage boundary decision |
| Korean user-facing docs may still contain excessive English technical nouns. | Documentation drift |
| Decision-ledger status wording can drift. | Implementation-readiness criterion |
| Current staging may be too large while deferring core user-visible value. | Stage boundary decision |
| Projection/template scope may be too broad for early implementation. | Stage boundary decision |
| Security guarantee wording must match actual enforcement level. | Schema/design decision |
| Agent context strategy must prevent excessive prompt/context load. | Implementation-readiness criterion |
| Documentation can drift into runtime-object language. | Documentation drift |
| Roadmap candidates can drift into staged delivery without promotion. | Future roadmap item |

| Review risk | Tracker status | Editing rule |
|---|---|---|
| Repository identity as the future Harness Server source repository can drift. | Baseline status check. | Keep entrypoints clear that the repo is currently documentation-only, is in post-redesign review, its intended future role is the Harness Server source repository, and server/runtime implementation has not started and may start only after documentation acceptance and a separate implementation-planning readiness decision. |
| Stage names can still imply Engineering Checkpoint, Kernel Smoke, or a legacy kernel-stage label is a product MVP or the first user-value slice. | Candidate to verify in the current docs. | Say Engineering Checkpoint is an internal authority-loop milestone, Kernel Smoke is its narrow future smoke-check authoring label, and MVP-1 User Work Loop is the first narrow user-value slice. |
| User-facing docs may open with heavy implementation disclaimers. | Candidate to verify in the current docs. | For user-facing Learn and Use docs, prefer a workflow-first opening that starts with what users can ask, what the agent should clarify, what Harness preserves, and what users should expect to see. Route detailed phase/status warnings to the root README, language READMEs, Build handoff docs, and Maintain guidance. Keep any local status note brief. |
| User-facing docs overuse internal terms. | Candidate to verify in the current docs. | Explain the user-visible situation first; introduce internal terms only when they help the reader act. |
| Discovery / requirements clarification may converge too early on a Change Unit or the first safe implementation unit. | Candidate to verify in the current docs. | Leave room for early discovery, shared understanding, and user-owned judgment before requiring a scoped implementation unit. |
| Legacy judgment alias mapping can drift. | Resolved design; regression-prevention check. | Active owner docs use canonical `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `judgment_type`, `presentation`, and `display_label`. `request_user_decision`, `judgment_domain`, `decision_kind`, `decision_profile`, `judgment_category`, `judgment_route`, and `display_depth` are compatibility or legacy terms, not independent user-facing axes. New examples should prefer the canonical judgment names, and affected gates or blocked actions stay in separate owner fields. |
| Decision Packet schema and examples may feel too heavy for small decisions. | Resolved design; regression-prevention check. | Small judgments should use `presentation=short` and fit on one screen. Full-format Decision Packet presentation is optional/later-profile or for complex judgments; it must not become the default prompt structure for every user-owned judgment. |
| Approval, work acceptance, and residual-risk acceptance are too easy to confuse. | Regression-prevention check. | Keep sensitive-action permission, work acceptance, and residual-risk acceptance separate in examples and routing text. |
| Storage/DDL can present future-profile tables, fields, or gates as required too early. | Candidate to verify in the current docs. | Distinguish reference-schema presence from staged implementation requirement. Required fields apply when the owning tool, record, or profile is active or used; they do not by themselves expand the smallest runnable slice. |
| Conformance fixture docs may be too detailed for the current implementation stage. | Candidate to verify in the current docs. | Keep fixture documentation future-oriented and staged. Do not imply executable fixture files or runnable Harness Server conformance tests exist now. |
| Operations entrypoints may appear required too early. | Candidate to verify in the current docs. | Keep operator entrypoints staged and future-oriented unless the relevant Build stage explicitly includes them. They must not become a prerequisite for Engineering Checkpoint by wording drift. |
| Korean user-facing docs may still contain excessive English technical nouns. | Candidate to verify in the current docs. | Use natural Korean first. Preserve exact English identifiers only for stable labels, schema names, file names, enum values, API fields, and places where precision needs the identifier. |
| Decision-ledger status wording can drift. | Regression-prevention check. | Keep entrypoint, handoff, and authoring-guide status aligned with the MVP Plan ledger. If open items exist, say they exist and that no server/runtime implementation decision has been accepted for coding. If a future ledger is empty, say only that content status and do not imply full acceptance, implementation completeness, implementation readiness, or server-coding readiness. |
| Current staging may be too large while deferring core user-visible value. | Candidate to verify in the current docs. | Name the tension between MVP size and early user-visible value; leave staging decisions to the owning Build and Reference docs. |
| Projection/template scope may be too broad for early implementation. | Candidate to verify in the current docs. | Flag broad early scope and route staging decisions to the projection/template owners. |
| Security guarantee wording must match actual enforcement level. | Regression-prevention check. | Use cooperative, detective, preventive, or isolated wording only when the documented surface can provide that level. |
| Agent context strategy must prevent excessive prompt/context load. | Regression-prevention check. | Keep always-on agent context short and route details to owner docs or retrieval paths. |
| Documentation can drift into runtime-object language. | Regression-prevention check. | Use the separation rule in Current Redesign Scope: maintain docs as source material, not runtime state or generated projections. |
| Roadmap candidates can drift into staged delivery without promotion. | Regression-prevention check. | Keep Roadmap items in the Roadmap unless an owner promotes them with scope, fixtures, fallback behavior, and no projection-as-canonical dependency. Do not treat future roadmap items as prerequisites for documentation review, Engineering Checkpoint, or MVP-1. |

## Documentation principles

Write from the reader's next useful step. A document should make it easier for the reader to understand, decide, use, build, verify, or maintain something specific.

Prefer a small number of strong ideas over a complete inventory of internal machinery. Move strict contracts to Reference docs, and link to them when another document needs precision.

Introduce unfamiliar concepts with a concrete situation first. Readers should understand why a concept exists before they are asked to memorize its exact definition.

Every document should make the reader's next useful step clear near the top. The opening may use predictable headings, ordinary requests, practical examples, or a workflow-first setup, as long as the reader can quickly tell why the page matters, what to do next, and where exact owner details live.

Write current documentation as current truth. Migration history, removed structures, and old names must stay out of the main explanation. If a dedicated migration note exists during a migration, keep that history there; otherwise rely on Git history or a clearly labeled non-active migration record.

## Document types

Use the document tree as an ownership split. Learn docs explain why. Use docs explain how users and agents interact. Build docs explain implementation sequence. Reference docs define exact contracts. Maintain docs define documentation maintenance rules. Avoid duplicating normative contracts across these paths; summarize locally and link to the owner.

### Learn

Learn docs build the reader's mental model.

They explain why Harness exists, why a concept matters, and what trade-offs shape the system before implementation details. Use them when the reader needs orientation more than a command, schema, or checklist.

### Use

Use docs help a person operate Harness during an AI-assisted work session.

They explain how users and agents interact with Harness: user-facing flow, status interpretation, decisions, handoffs, and recovery paths. Mention internal gates only when they explain why the user sees a block or next action.

### Build

Build docs help an implementer construct the reference system after documentation acceptance and a separate implementation-planning readiness decision.

They explain implementation sequence after documentation acceptance and a separate implementation-planning readiness decision: implementation order, module boundaries, runnable slices, staging, and verification strategy. Link to Reference docs for exact schemas, DDL, and invariants.

### Reference

Reference docs own exact contracts.

They define exact contracts: strict schemas, gates, DDL, enum values, state transitions, invariants, API shapes, storage rules, projection rules, fixture semantics, and official definitions.

### Maintain

Maintain docs govern the documentation system itself.

They define documentation maintenance rules: authoring rules, translation policy, review checklists, link hygiene, ownership maps, and documentation-maintenance expectations. They must not become runtime conformance specs or product implementation plans.

## Entrypoint rule

README pages are routing pages before they are explanations. They should briefly say what Harness is and is not, then route first-time readers, users, implementers, reference readers, and maintainers to the right owner docs.

Keep entrypoints current and compact. Do not use them to preserve migration history, removed names, inactive paths, or old structures unless a section is clearly labeled as a non-active migration record.

README pages may summarize path ownership, but they should not copy strict contracts. Link to Reference owners for exact schemas, DDL, gates, state transitions, fixture semantics, template bodies, and official definitions.

First-time reader routes should include the fast practical tour before deeper Learn and Reference paths. Use routes should include practical user-owned judgment examples near the User Guide so readers can understand judgment requests before reading strict Reference contracts.

## Opening patterns

Every active document should begin with a compact opening that makes the reader's path clear. The required information may appear through exact headings, natural headings, prose, examples, or a workflow-first setup. Do not treat the four headings below as mandatory for every page.

Reference, Build, and Maintain docs may use this predictable structured opening when it helps the reader:

- `What this document helps you do`: state the useful outcome in plain language. Avoid saying only what the document "covers."
- `Read this when`: name the situation that makes the document relevant. This can be a short list.
- `Before you read`: name the assumed context, prior document, or prerequisite. If there is no prerequisite, say so briefly.
- `Main idea`: give the reader the central model or claim that will make the rest of the page easier to follow.

These names are a pattern, not a global heading contract. A page may satisfy the opening rule without using these exact headings when the reader situation is clearer another way.

### Workflow-first opening for Learn and Use

User-facing Learn and Use pages may start with ordinary requests, practical examples, or user workflow when that serves the reader better than the structured opening. This is especially appropriate for primary user-facing pages.

A workflow-first opening should:

- start with what users can ask
- show what the agent should clarify
- state what Harness preserves
- say what users should expect to see
- keep phase/status notes brief and route detailed status to the root README, language READMEs, Build handoff docs, and Maintain guidance
- introduce internal Harness terms only after the user-visible situation is clear

Heading text differences are not drift when the document serves the reader situation, the necessary context is present, owner links remain valid, exact contract details stay in Reference owners, and English/Korean versions are semantically aligned. Do not reintroduce internal labels such as `direct`, `work`, `Decision Packet`, or `judgment_domain` into an opening only to satisfy the structured pattern.

### Reference scope, only for reference docs

Reference docs should state the exact contract they own and what they deliberately do not own. This prevents strict details from spreading across Learn, Use, and Build docs.

### Template reference opening, only for `reference/templates`

Template reference files use a specialized opening pattern. Docs-maintenance should identify them by path: `docs/*/reference/templates/README.md` for the directory index and non-README Markdown files under `docs/*/reference/templates/` for individual templates.

The directory README should begin with `Used when`, then output tiers and template implementation classes. It should explain that the directory owns rendered template bodies and display card shapes while projection rules, freshness behavior, and authority boundaries stay with their reference owners.

Each individual template file should begin with these sections, in this order:

- `Used when`: the reader purpose and projection or display situation.
- `Source records`: the owner records, refs, gates, artifacts, or summaries the renderer may read.
- `Rendered sections`: the display shape readers should expect.
- `Full template`: the complete rendered body or card body.

Template files must also make the non-authority boundary visible, either in the opening explanation or near `Notes`: a template is rendered display, not canonical state, gate authority, approval, acceptance, evidence, schema, DDL, or runtime behavior.

## Concept introduction rule

Introduce concepts through examples before strict definitions.

Start with a concrete situation, show what problem the concept solves, and then name the concept. Put the strict definition after the reader has seen why it matters.

Preferred shape:

```text
When an agent wants to change product state, Harness first needs the work scope: what may change, what stays out of scope, and where the agent must stop. The internal scoped-write record is the Change Unit. The larger user-value item the user wants finished or answered is the Task.
```

Avoid opening a Learn doc with a dense definition list unless the page is explicitly a glossary or reference table.

## Reference contract rule

Strict schemas, gates, DDL, enum values, state transitions, invariants, API shapes, storage rules, projection contract details, and fixture semantics belong in Reference docs.

Learn, Use, Build, and Maintain docs may summarize a contract in one or two sentences when needed, then link to the owning Reference document. They should not duplicate full tables, schema bodies, transition matrices, DDL blocks, gate matrices, algorithm steps, or fixture mini-languages.

Build docs should describe what to implement first, what to defer, and what proves a stage complete. They should not copy public request/response schemas, DDL, storage validation rules, close-blocker taxonomies, gate compatibility matrices, or fixture assertion fields. When a Build checklist needs precision, link to the owner Reference section and state only the local sequencing consequence.

Use docs should stay at the user trust boundary. They may mention the contract that explains a user-visible hold, blocker, decision prompt, evidence gap, or close result, but should not expose low-level field lists, storage rows, or validator internals unless the user needs that detail to make a judgment.

Reference docs should stay contract-shaped. A short plain-language introduction is useful, but long tutorials, staged delivery plans, and reader walkthroughs should move to Learn, Use, or Build and link back to the Reference owner for exact rules.

Runtime conformance fixture body shape, assertion modes, isolated execution behavior, JSON `TEXT` validation, and owner-bound enum/status validation are owned by [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format). Other docs should summarize that conformance is executable-state-based and link to the owner instead of restating the full contract.

Detailed future scenario candidates, future fixture examples by concern, staged fixture coverage maps, fixture suite family summaries, and catalog-only future candidates are owned by [Future Fixture Catalog](../reference/future-fixture-catalog.md). Future catalog rows are design inventory only; they must not redefine fixture body shape, public MCP schemas, DDL, stage exits, runtime readiness, generated artifacts, or proof that fixtures already run.

## Repetition rule

Do not repeat long source-of-truth paragraphs across docs.

When another document needs the same idea, write a short local summary and link to the owner. If the source text changes, update the owner first, then check summaries for drift.

Repeated explanatory examples are allowed when they serve different readers, but repeated normative contract language is a drift risk.

Before adding or accepting a long contract paragraph in Build or Reference, search for the same field, gate, API, storage, fixture, or security wording in the other family. If Build is repeating Reference, shorten Build to sequencing plus an owner link. If Reference is mostly teaching the implementation journey, move or link that explanation to Build and keep only the contract needed to interpret the section.

For the non-authority boundaries that are easy to repeat, use these owners:

| Boundary | Owner for exact wording |
|---|---|
| Context Index and retrieved/indexed context | [Roadmap: Candidate Inventory](../roadmap.md#candidate-inventory) for future feature boundary; [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) for connector context handling |
| Local Derived Metrics | [Roadmap: Candidate Inventory](../roadmap.md#candidate-inventory) |
| Role Lens | [Agent Integration: Role Lens Behavior](../reference/agent-integration.md#role-lens-behavior) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff and export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); rendered shape in [EXPORT Template](../reference/templates/export.md) |
| Docs-maintenance | [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks) for rule bodies; [Operations And Conformance: Docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-profile) for operator reporting |
| Projection and report surfaces | [Document Projection Reference](../reference/document-projection.md); rendered shapes in [Template Reference](../reference/templates/README.md) |
| Security assets, trust boundaries, threat categories, control categories, guarantee-level meanings, and high-risk cooperative/detective/preventive/isolated security expectations | [Security Threat Model Reference](../reference/security-threat-model.md) for threat concepts and honest guarantee display; exact API, storage, kernel, connector, operations, and conformance behavior stays with those owners |

## Owner-link summary pattern

When you find duplicated normative language outside its owner, do not polish the duplicate in place. First decide which document owns the exact contract. Update that owner if the contract needs to change, then replace non-owner copies with:

- one ordinary-language sentence naming what the reader needs to know now
- one link to the owner document or owner section for exact rules
- any local consequence for the current reader path

Example:

```text
Product writes need current Change Unit scope and Write Authorization. Exact write-gate behavior is owned by [Kernel Reference](../reference/kernel.md), and the public request shape is owned by [MVP API](../reference/api/mvp-api.md).
```

Do not paste the gate matrix, request schema, DDL block, fixture body, template body, enum table, or glossary definition into Learn, Use, Build, or Maintain docs.

## Diagram rule

Use diagrams only when they reduce cognitive load.

A diagram is useful when it shows a relationship, sequence, boundary, or lifecycle more clearly than prose. Do not add a diagram as decoration, as a second copy of an already clear list, or as a way to hide unresolved structure.

Every diagram should have nearby prose that explains what to notice. If a diagram and the prose disagree, the owning prose or reference contract is the source to fix first.

## English/Korean semantic parity rule

English and Korean docs must preserve the same active file map, semantic section coverage, and contractual detail.

Paired English/Korean files keep the same active file map and semantic section coverage. Heading text and minor grouping may be idiomatic when owner links, stable identifiers, and reviewability remain clear. Korean headings and prose may be natural Korean; different but semantically equivalent Korean headings are not an automatic docs-maintenance failure. Official identifiers, API names, schema names, enum values, DDL names, file names, error codes, validator IDs, code identifiers, and product terms listed in the translation guide must remain exact.

Any semantic change in `docs/en` must be mirrored in `docs/ko` in the same batch, and the reverse is also true.

## Korean documentation quality rule

Update Korean files semantically, not by literal translation. The Korean version should carry the same meaning, owner links, and stable identifiers, but it may use different sentence order, headings, and paragraph grouping when that makes the page easier to read.

In Korean user-facing text, prefer natural Korean first. Put stable English identifiers in parentheses only when they are needed for recognition, searchability, or exact contract alignment. Avoid awkward bilingual prose in user-facing sections.

Keep Korean sentences shorter than the English source where possible. Split long English sentences into shorter Korean sentences instead of copying the structure.

Preserve exact schema identifiers, API names, enum values, DDL names, file names, error codes, validator IDs, code identifiers, and official product terms in reference or maintainer-facing contexts. In Learn and Use docs, introduce the ordinary Korean idea first unless the exact identifier is required for the reader's next action.

## Link and rename rule

When you rename, move, split, or merge a document, update links in both languages in the same batch.

Prefer links to the owner document or owner section instead of links to secondary summaries. Do not point active owner links to removed migration context.

If old names, old structures, or migration decisions must be retained for review, keep them in a clearly labeled non-active migration record. Active docs should describe the current structure and link to current owners.

After a rename, search for old paths, old anchors, old headings, and old title text. Update the README path, nearby cross-references, template/reference links, and paired-language links together.

## Docs-maintenance checks

Docs-maintenance checks are editorial quality checks. They may report documentation drift, owner mismatch, English/Korean parity issues, duplicate normative text outside the owner, broken links or anchors, and TODO hygiene problems. They are not runtime conformance or implementation readiness, and they do not execute fixture actions, seed runtime state, compare runtime state/events/artifacts/projections/errors, or count toward runtime fixture pass/fail. They do not create or update canonical state, runtime state, `task_events`, evidence artifacts or Evidence Manifests, QA results or Manual QA records, Acceptance records, residual-risk acceptance or Residual Risk records, close readiness, projection refreshes or generated operational reports, or implementation readiness.

Maintain docs may define documentation review rules, category labels, and reviewer expectations. They must not define runtime conformance pass/fail, runtime fixture semantics, Core state effects, gate behavior, or implementation readiness. When a docs-maintenance finding touches a runtime contract, the finding should point to the owner Reference document instead of restating that contract.

### Final pre-acceptance review

Before maintainers accept the documentation set for implementation planning, do one final docs-maintenance pass. Check English/Korean active file map parity, semantic section parity in paired files, broken links and anchors, owner-boundary drift, non-owner duplicate contracts, terminology drift for Approval, Decision Packet, Evidence, Verification, Manual QA, Acceptance, Residual Risk, Projection, and Guarantee Level, and TODO hygiene.

Also check the implementation-readiness criteria in [Implementation Overview](../build/implementation-overview.md#implementation-readiness-criteria): repository identity, user-facing flow without internal-term burden, Discovery as requirements clarification rather than premature Change Unit convergence, canonical `user_judgment` naming with mapped legacy aliases, proportional judgment prompts, Approval/work acceptance/residual-risk acceptance separation, coherent stages, Kernel/API/storage/reference agreement, staged Storage/API scope, staged projection/template scope, honest security guarantee wording, agent context strategy, staged future-oriented conformance fixture plan, staged operations surface, Korean user-facing readability, and clean links/TODOs/terminology.

This final review is still editorial review. It summarizes whether the docs are coherent enough for maintainer handoff; it does not create runtime conformance, canonical state, evidence, QA, Acceptance, residual-risk acceptance, close readiness, or implementation readiness. Use the existing docs-maintenance reporting expectations when recording findings; do not create a new required report format for this final pass.

A docs-maintenance review or future checker should report:

- item category: documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, or future roadmap item
- result: `PASS`, `WARN`, or `FAIL`
- file path
- heading or anchor when available
- owner document and expected source section
- observed drift
- suggested fix
- runtime effect note: none; no canonical state transition or runtime fixture result was recorded
- maintenance note when a finding needs extra context

If a finding is non-blocking for documentation review but blocking before implementation planning or server coding, say both parts explicitly in the maintenance note.

Resolve drift in this order:

1. Identify the owner document or owner section for the exact contract.
2. Update the owner first when the contract itself is wrong or incomplete.
3. Replace non-owner duplicate contracts with a short reader-focused summary plus owner link.
4. Mirror any English/Korean semantic change in the paired file during the same batch.
5. Repair links, anchors, TODO metadata, or glossary phrasing only after the owner boundary is clear.

Use these result meanings:

| Result | Meaning |
|---|---|
| `FAIL` | Drift can make active docs contradictory or non-actionable, such as broken owner links, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` mismatch, missing paired active files, missing semantic section coverage, or non-owner text redefining an owner contract. Idiomatic heading text, workflow-first Learn/Use openings, or minor grouping differences are not failures when the reader situation is served, necessary context is present, owner links and stable identifiers remain valid, exact contracts stay in owner docs, and reviewability remains clear. |
| `WARN` | Drift should be cleaned up but does not yet contradict an owner contract, such as minor glossary phrasing drift, duplicate explanatory prose that is not normative, stale cross-reference wording whose affected stage is explicit, or incomplete TODO metadata that is still understandable. |
| `PASS` | No relevant drift is found for the category. |

Required check categories:

| Category | Required check |
|---|---|
| English/Korean file structure parity | `docs/en` and `docs/ko` keep the same active document paths, README entries, and paired route expectations unless an exception is explicitly documented. |
| English/Korean semantic section parity | Paired files keep the same active file map, reader purpose, semantic section coverage, owner links, and contractual detail. Heading text and minor grouping may be idiomatic when stable identifiers, schema names, enum values, DDL names, validator IDs, code identifiers, and reviewability remain clear. |
| Opening convention compliance | Active docs make the reader's next useful step clear near the top. Reference, Build, and Maintain docs may use the structured opening pattern; Learn and Use docs may use a workflow-first opening. Do not fail a Learn or Use page solely because it omits the old four exact headings. `docs/*/reference/templates/README.md` uses `Used when`, output tiers, and template implementation classes; individual template files under `docs/*/reference/templates/` other than `README.md` use `Used when`, implementation tier, `Source records`, `Rendered sections`, and `Full template`, plus a visible non-authority boundary. |
| Broken cross-reference detection | Markdown links, heading anchors, template/reference links, same-language README routes, paired-language entry links, and owner-section links resolve to active docs and current anchors. |
| Owner-boundary drift | Exact contracts and active owner concepts stay in their active owners, including `reference/kernel.md`, `reference/api/mvp-api.md`, `reference/api/schema-core.md`, `reference/api/errors.md`, `reference/api/schema-later.md`, `reference/storage-and-ddl.md`, `reference/document-projection.md`, `reference/templates/*.md`, `reference/design-quality-policies.md`, `reference/security-threat-model.md`, `reference/operations-and-conformance.md`, `reference/conformance-fixtures.md`, `reference/future-fixture-catalog.md`, and `reference/glossary.md`; non-owner docs summarize and link rather than redefining those contracts. |
| Fixture/action schema drift | Conformance fixture examples, including exact-shape or example-shaped examples in `reference/future-fixture-catalog.md`, keep `action` and executable `input` aligned with public MCP request schemas in `reference/api/mvp-api.md` and shared API schemas in `reference/api/schema-core.md`, plus the `ToolEnvelope` expansion convention in `reference/conformance-fixtures.md`; future catalog entries must not redefine fixture body shape, public MCP schemas, DDL, stage exits, or runtime readiness; docs-maintenance may flag drift but does not execute fixture actions or restate fixture semantics here. |
| Enum, event, validator, and projection drift | State/gate/result values and Kernel Stable Event Catalog names match `reference/kernel.md`; error values match `reference/api/errors.md`; stable `ValidatorResult` IDs, `ProjectionKind` values, and the API-owned ProjectionKind support taxonomy match `reference/api/schema-core.md` and `reference/api/schema-later.md`; storage values match `reference/storage-and-ddl.md`; template implementation classes and projection freshness behavior match `reference/document-projection.md`; rendered template ownership matches `reference/templates/*.md`. |
| Glossary and source-of-truth phrasing drift | Official terms, capitalization, record ID prefixes, source-of-truth wording, and authority-boundary phrases match `reference/glossary.md` and the relevant owner docs without implying extra state authorities. |
| TODO compliance | `TODO_DECISION` and `TODO_IMPLEMENT` use the allowed meanings, name the gap clearly, include enough owner/context to act on, and do not leave `TODO_REWRITE` markers in finished canonical sections. |
| Non-owner duplicate full contracts | Full schemas, DDL, transition tables, fixture mini-languages, template bodies, enum tables, validator tables, projection tables, or glossary definitions outside the owner doc are replaced with a short summary plus owner link. For fixture material, link exact mechanics to `reference/conformance-fixtures.md` and detailed future catalog content to `reference/future-fixture-catalog.md`. |

## Review checklist

```text
[ ] Does the document serve a clear reader situation?
[ ] Do README entrypoints route first-time readers, users, implementers, reference readers, and maintainers quickly?
[ ] Does the opening make the reader's next useful step clear using an allowed structured, workflow-first, or template-specific pattern?
[ ] Are concepts introduced through examples before strict definitions?
[ ] Are strict schemas, gates, DDL, enums, and invariants kept in Reference docs?
[ ] Are long source-of-truth paragraphs and duplicated normative contract blocks summarized and linked instead of repeated?
[ ] Do diagrams reduce cognitive load?
[ ] Are English and Korean files semantically aligned?
[ ] Are official identifiers preserved exactly?
[ ] Are renamed paths, anchors, and README links updated in both languages?
[ ] Is current truth separated from migration history?
[ ] Are Maintain docs limited to documentation governance, not runtime behavior?
```

## Reference ownership map

Use this map for broad document routing. For strict Reference contracts, use [Reference contract owner map](#reference-contract-owner-map) above; this table identifies the active owner in the current documentation structure, so inactive paths do not remain part of the authoring workflow.

| Subject | Active owner |
|---|---|
| Repo and docs entrypoints, reader routes, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoints `docs/en/README.md` and `docs/ko/README.md` |
| Shared reader mental model and three-space overview | `learn/overview.md` |
| Fast first-reader practical tour and short usage scenarios | `learn/harness-in-15-minutes.md` |
| Small core concept introduction | `learn/concepts.md` |
| Project purpose, target users, values, scope, non-goals, automation philosophy | `learn/purpose-and-principles.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | `learn/purpose-and-principles.md` for reader explanation; `reference/design-quality-policies.md` and `reference/kernel.md` for exact contract impact |
| Kernel entities, lifecycle, gates, state transitions, close semantics, `prepare_write`, `close_task` | `reference/kernel.md` |
| Runtime architecture, three spaces in implementation detail, Core process model, artifact architecture, projection/reconcile architecture, guarantee-level display placement | `reference/runtime-architecture.md` |
| Security assets, trust boundaries, threat categories, control categories, guarantee-level meanings, and high-risk cooperative/detective/preventive/isolated security expectations | `reference/security-threat-model.md` for threat concepts and honest guarantee display; exact enforcement, API, storage, kernel, connector, operations, and conformance behavior stays with those owners |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `reference/api/mvp-api.md`, `reference/api/schema-core.md`, `reference/api/errors.md`, `reference/api/schema-later.md` |
| SQLite DDL, migrations, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage-and-ddl.md` |
| MVP implementation order and stage exit criteria | `build/mvp-plan.md` |
| First runnable implementation slice | `build/first-runnable-slice.md` |
| Markdown-rendered projection principles, authority matrix, managed blocks, human-editable sections, artifact reference rendering, output tiers, template implementation classes, projection freshness/failure rules | `reference/document-projection.md` |
| All projection template bodies and display card shapes | `reference/templates/*.md` |
| Design-quality policy contracts, validators, severity composition, waiver semantics, evidence expectations, close impact | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgments, close checklist | `use/user-guide.md` |
| Practical user-owned judgment examples and user-facing judgment request patterns | `use/decision-packet-cookbook.md` for examples; `reference/kernel.md` and `reference/api/mvp-api.md` for exact user judgment behavior |
| User/agent session procedure | `use/agent-session-flow.md` |
| Agent surface capability profiles, common connector contract, fallback semantics, Role Lens, connector conformance overview | `reference/agent-integration.md` |
| Surface-specific recipes | `reference/surface-cookbook.md` |
| Generic capability profile examples | `reference/agent-integration.md` |
| Operator procedures, conformance run overview, doctor/recover/reconcile/export/artifact integrity, docs-maintenance reporting | `reference/operations-and-conformance.md` |
| Core conformance model, exact fixture body shape, runner execution, assertion semantics, current-phase status, fixture profiles by proven behavior, suite metadata boundaries, reduced Kernel Smoke authoring queue | `reference/conformance-fixtures.md` |
| Detailed future scenario candidates, future fixture examples by concern, staged fixture coverage maps, fixture suite family summaries, catalog-only future candidates | `reference/future-fixture-catalog.md` |
| Official term definitions and capitalization | `reference/glossary.md` |
| Roadmap roadmap | `roadmap.md` |
| Documentation authoring rules | `maintain/authoring-guide.md` |
| Translation and bilingual prose rules | `maintain/translation-guide.md` |
| Rewrite planning categories and redesign triage | `maintain/rewrite-plan.md` |
