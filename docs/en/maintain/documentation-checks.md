# Documentation Checks

Use this checklist before final documentation acceptance or a major review handoff. It is a practical docs-maintenance checklist for Markdown documentation only: a read-only documentation quality profile.

This checklist is not a runtime conformance suite. It does not run fixtures, seed runtime state, compare runtime state/events/artifacts/projections/errors, append `task_events`, create artifacts, refresh projections, create generated operational artifacts, create conformance reports, create QA or acceptance state, record evidence, record QA, record Acceptance, record Residual Risk, affect close readiness, close work, or prove implementation readiness.

Docs-maintenance `PASS`, `WARN`, and `FAIL` labels may help manual review decide what to inspect or edit next. They are not manual acceptance, final acceptance, close readiness, implementation readiness, or runtime fixture results.

Runtime conformance is separate. It applies only to implemented Core/API/storage/surface behavior and is judged by executable fixtures and state assertions, not documentation prose. No runtime conformance result should be implied before runtime implementation and materialized fixture suites exist.

## Review Types

Use these labels when reporting a check result.

| Review type | Meaning |
|---|---|
| `manual` | A reviewer must make the judgment. Search tools may collect candidates, but a script-only pass is not enough. |
| `scriptable` | A local documentation script or parser can check the stated condition directly. A reviewer still handles documented exceptions. |
| `future-runtime-only` | Only a future runtime implementation and its future proof path can validate the behavior. Current documentation review can only check that the docs do not overclaim it. |

## Checklist

### Link Check

- Review type: `scriptable`.
- What to inspect: Relative Markdown links, README routes, paired-language links, owner-section links, and heading anchors in active docs.
- Common failure examples: A link points to a moved file. An anchor still uses an old heading. An English page links to a Korean-only anchor by accident. A README route points to a deleted or inactive page.
- Pass means: Every relative link and anchor resolves to an active document or to a clearly documented exception. Owner links point to the current owner document or owner section.

### Term Check

- Review type: `manual`.
- What to inspect: Learn and Use pages, examples, headings, summaries, and status text for internal labels used as default user-facing language.
- Common failure examples: A user-facing page opens with `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events` before explaining the ordinary user situation. A user example implies the user must say an internal label to get help.
- Pass means: User-facing prose starts from normal user language. Internal labels appear only when they help explain a visible boundary, blocker, record, API, template, or Reference link.

### Stage Check

- Review type: `manual`.
- What to inspect: MVP-1, Engineering Checkpoint, Kernel Smoke, Assurance Profile, Operations Profile, Later, and Roadmap wording in Build, Reference, Use, and Roadmap docs.
- Common failure examples: A Roadmap candidate is written as an MVP-1 requirement. `Kernel Smoke` is treated as a stage. Later-profile export, reporting, operations, or conformance-runner material is required for the smallest runnable slice.
- Pass means: Engineering Checkpoint stays an internal authority-loop smoke. MVP-1 User Work Loop stays the first user-value milestone. Later-profile and Roadmap material remains future scope unless an owner document has promoted it with scope, fallback behavior, and proof expectations.

### Status Check

- Review type: `manual`.
- What to inspect: Entrypoints, handoff sections, Build docs, Maintain docs, and any prose that could imply current implementation status.
- Common failure examples: A page says the Harness Server already exists in this repo. Documentation acceptance is treated as server-coding authorization. Reference design prose is framed as implemented runtime behavior without a future or design boundary.
- Pass means: Docs describe the current repo as documentation-only, in post-redesign review, and not implementation-ready unless the maintainer handoff owner explicitly says so. Intended future behavior is distinguishable from implemented behavior.

### Security Wording Check

- Review type: `manual` for documentation wording. Actual preventive or isolated enforcement proof is `future-runtime-only`.
- What to inspect: Claims using cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, or isolation language.
- Common failure examples: Write Authorization is described as OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation. A connector is said to block arbitrary tool calls without a proven blocking path. A security boundary implies broader OS isolation than the owner document supports.
- Pass means: Each claim matches the documented guarantee level. Cooperative and detective surfaces do not claim preventive control. Preventive or isolated claims name the exact covered operation, mechanism, owner document, and proof status, or remain clearly future-oriented.

### User-Language Check

- Review type: `manual`.
- What to inspect: Openings, examples, commands users might say, status explanations, judgment prompts, close explanations, and recovery text in user-facing docs.
- Common failure examples: A Use page starts with a record taxonomy instead of what the user can ask. A judgment prompt leads with `Decision Packet` rather than the choice and consequence. A status view explanation says `ProjectionKind` before explaining the visible summary.
- Pass means: User docs begin with ordinary tasks, questions, visible blockers, needed judgments, available evidence, or close outcomes. Internal labels are introduced after the reader already knows what problem the label helps solve.

### Mermaid Check

- Review type: `scriptable` for syntax where a Mermaid parser is available; `manual` for usefulness.
- What to inspect: Mermaid fenced code blocks, nearby explanatory prose, diagram labels, and consistency with owner prose.
- Common failure examples: A diagram has Mermaid syntax that cannot render. A diagram is decorative but does not clarify a relationship, sequence, boundary, or lifecycle. A diagram contradicts the surrounding prose or owner contract.
- Pass means: Diagrams are syntactically reasonable, renderable in the expected docs toolchain, and useful enough to reduce reader effort. Nearby prose explains what to notice.

### Bilingual Check

- Review type: `manual`.
- What to inspect: English/Korean active file map, paired file purpose, section coverage, owner links, stable identifiers, exact code-like strings, and Korean prose quality.
- Common failure examples: A Korean page omits a section added in English. A path, enum value, error code, validator ID, or API name is translated or changed. Korean prose becomes English technical nouns joined by Korean particles. A paired link points to a different owner.
- Pass means: English and Korean docs preserve the same meaning, coverage, owner routing, and exact identifiers. Korean headings and prose may differ when they remain natural and semantically aligned.

### Owner Check

- Review type: `manual`.
- What to inspect: Strict contracts, schemas, DDL, enum values, state transitions, gate rules, algorithms, fixture body shapes, template bodies, storage rules, security guarantees, and official definitions.
- Common failure examples: A Use page repeats a full gate matrix. A Build page defines an enum table owned by Reference. A Maintain page gives a second normative definition of projection freshness. A glossary definition is copied and changed outside the Glossary owner.
- Pass means: Each strict contract is defined in one owner document. Non-owner docs use a short reader-facing summary, the local consequence, and an owner link.

### Repair-Target Owner Map Check

- Review type: `manual`.
- What to inspect: The known pre-implementation repair axes in [Authoring Guide: Pre-implementation repair target owner map](authoring-guide.md#pre-implementation-repair-target-owner-map), including owner contract, API/schema, Storage/DDL, Core transition, stage/profile, evidence/close, security/local-access, conformance proof, user-output/context, and design-quality drift.
- Common failure examples: A later-profile API branch is written as an MVP requirement. A status card is treated as gate authority. A design-quality validator becomes a blocker outside its owner activation rule. Documentation checks are described as runtime conformance. Security wording claims pre-tool blocking without a proven owner path.
- Pass means: Each observed repair axis routes to the canonical owner family, and non-owner docs keep only a short local summary plus owner link. Listed `FAIL` symptoms are reported as docs-maintenance failures only; this check does not decide documentation acceptance, manual acceptance, runtime conformance, or implementation readiness.

### Projection/State Check

- Review type: `manual`.
- What to inspect: Language about projections, rendered templates, Markdown status views, generated documents, state, artifacts, evidence, QA, Acceptance, close readiness, and operational truth.
- Common failure examples: A rendered Markdown view is called canonical state. A documentation file is treated as a runtime object, generated projection, evidence record, QA record, Acceptance record, Residual Risk record, close record, or operational artifact. A projection is described as gate authority.
- Pass means: Rendered views and generated documents are described as derived display. Future operational authority remains with Core-owned local state and artifact references, as owned by the relevant Reference docs.

### Template-Scope Check

- Review type: `manual`.
- What to inspect: Template references, projection/template pages, Use docs, Build docs, Later docs, and Roadmap items that mention future templates or rendered outputs.
- Common failure examples: A later-profile export template is required for MVP-1 close. Future template bodies are treated as current MVP requirements. A user-facing page duplicates full template bodies instead of linking to the Template Reference owner.
- Pass means: Future templates remain future or later-profile material unless an owner promotes them. Active MVP requirements name only the templates and rendered views required by the active stage. Full template bodies stay with the Template Reference owner.
