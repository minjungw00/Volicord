# Diagram policy

Use this policy when creating, reviewing, captioning, or maintaining diagrams in
Volicord documentation. It complements [Documentation policy](documentation-policy.md),
[Document Charters](document-charters.md), [Brand Guidelines](brand-guidelines.md),
and [Validation](validation.md).

This is a maintenance policy. It does not define product behavior, API
behavior, storage effects, security guarantees, runtime behavior, schemas, Core
authority semantics, conformance results, QA results, acceptance decisions,
close-readiness state, or residual-risk decisions.

## Purpose

Every diagram must answer one reader question. Choose the diagram category
before drawing arrows, and make that question clear in the surrounding prose
or caption. Diagram categories and any role metadata are authoring and review
aids; they help maintainers decide placement, caption scope, and validation
expectations. They are not required reader-facing labels. If one picture needs
to show workflow order, component responsibility, authority, runtime calls, and
storage movement at the same time, split it into smaller diagrams or move the
detail to the focused owner document.

Diagrams are explanatory aids, not replacement contracts. When exact behavior,
storage effects, schema meaning, security wording, API behavior, or Core
authority semantics matter, the diagram must link to the applicable Reference
owner or document owner. A diagram that looks reference-like must have a clear
accuracy owner or should not be added.

## Required Caption Context

Simple diagrams may satisfy these requirements in one concise caption. Dense
Mermaid diagrams, diagrams in Reference documents, and diagrams with multiple
arrow styles should use a short caption plus a nearby note or legend.

Captions should explain what the diagram shows and, when useful, what it omits.
When relevant, the caption or adjacent prose must clarify:

- the question answered by the diagram
- the target reader
- what each arrow style means
- the owner or source of truth used to keep the diagram accurate
- what the diagram intentionally omits

Do not rely on the diagram title alone. If readers need to inspect the source
to understand the arrow semantics, the caption is not complete.

## Reader-Facing Captions And Role Metadata

Role metadata belongs in authoring notes, review comments, or maintain-policy
discussion when it helps reviewers understand why a diagram belongs in a
document. It should not be pasted into ordinary reader-facing prose.

Avoid literal policy labels such as `Diagram role:` or `그림 역할:` in README,
User Guide, Architecture Guide, Reference, and Maintain pages. Reader-facing
documents should state the same purpose naturally, for example:

- "This workflow shows how the user, agent host, and Volicord hand off a
  decision; it omits storage layout and API call order."
- "This map shows the local components a host starts or reads from; it is not a
  complete runtime sequence."

It is acceptable to say "workflow" or "component map" when that is the clearest
reader-facing description. Do not make ordinary readers parse maintenance
metadata before they can understand the diagram.

## Diagram Categories

| Category | Question it answers | Typical reader | Arrows usually mean | Do not use it to show | Appropriate locations |
|---|---|---|---|---|---|
| User workflow diagram | What does the user or agent do next, and where does judgment or handoff occur? | New users, product users, agents, operators | Work order, handoff, user decision, or visible collaboration loop | Architecture components, API call order, storage ownership, or authority source of truth | Root README, User Guide Overview, User Workflow Guide, Agent Workflow Guide, guide-level examples |
| Component map | What parts exist, and what responsibility or boundary does each part have? | Operators, source-code learners, implementers | Relationship, containment, responsibility boundary, or allowed communication as defined by the legend | A user task flow, exact runtime sequence, ownership of user judgment, or storage lifecycle | Root README at overview depth, Agent Host Setup, Architecture Guide documents, focused Reference owners when contract-backed |
| Runtime sequence | In what time order do calls, messages, or process steps happen? | Implementers, agent integrators, reviewers | Time-ordered invocation, message, response, callback, or process transition | Authority dependency, user approval meaning, durable storage ownership, or broad component responsibility | Architecture Guide documents, request lifecycle explanations, API method owners, focused Reference pages |
| Authority model | Which record, role, or channel owns a decision, source of truth, or authority boundary? | Maintainers, agents, reviewers, implementers | Authority relation, ownership, source-of-truth path, or decision-routing relation | Execution order, implementation call stack, storage location, or component deployment | Core Model, User Workflow Guide, Agent Workflow Guide, Reference owners, overview pages only at conceptual depth |
| Storage lifecycle | How does a stored record, artifact, or file state move through creation, update, retention, or removal? | Implementers, storage reviewers, maintainers | State transition, persistence boundary, retention transition, or deletion/removal phase | Process ownership, authority ownership, user workflow order, or runtime call order | Storage Reference, Runtime Boundaries, Architecture Guide architecture, storage-focused implementation docs |
| Connection setup flow | How does an operator prepare, verify, guide, use, or remove an Agent Connection setup? | Operators, agent integrators, agents | Setup step, verification checkpoint, configuration handoff, or recovery branch | Full MCP protocol sequence, storage lifecycle, broad architecture map, or exact CLI output contract | Quickstart for compact happy paths, Agent Host Setup, Agent Host Troubleshooting, Administrative CLI at command-contract depth |
| Dependency graph | What depends on what for build, source organization, documentation ownership, or concept routing? | Implementers, maintainers, reviewers, source-code learners | Dependency direction declared by the caption, such as build dependency, module dependency, document dependency, or concept prerequisite | Runtime execution order, authority ownership, storage movement, or user task order | Architecture Guide documents, architecture pages, testing strategy, maintain documents, route metadata explanations |

## Review Rules

- Match the diagram category to the document charter. A first-time onboarding
  page should not carry a dense implementation sequence, and a Reference page
  should not use a broad orientation diagram as the source of exact behavior.
- Keep review metadata separate from reader prose. Reader-facing captions should
  explain the diagram's purpose, arrow meaning, and omissions without exposing
  maintenance labels.
- Use one arrow meaning per arrow style. If execution flow and authority
  dependency both appear, use distinct styles with a legend or separate the
  diagrams.
- Keep storage location, process ownership, and authority ownership separate.
  A file path, process, or component can appear in more than one relationship,
  but the diagram must not make those relationships look identical.
- Use component maps to orient readers to parts and boundaries. Do not use
  architecture component diagrams as substitutes for user workflows.
- Place dense Mermaid diagrams where the intended reader expects detail. If a
  first-time reader needs product orientation, prefer a compact workflow or
  concept map and link to the deeper owner.
- Do not add dense Mermaid diagrams without explanatory captions, legends, or
  nearby prose that names the omitted detail.
- For diagrams with product-claim or visual-presentation weight, follow
  [Brand Guidelines](brand-guidelines.md). Do not use color as the only status
  carrier.

## Accuracy Maintenance

Review a diagram when its document owner changes meaning, when a linked
Reference owner changes the facts the diagram summarizes, or when durable source
structure changes in a way that affects an Architecture Guide diagram.

Before keeping or adding a diagram, confirm:

- the category and reader question are clear
- the caption names arrow semantics and omissions when needed
- the diagram is located in a document allowed to own that level of detail
- exact behavior routes to the focused Reference owner instead of being defined
  only by the diagram
- the paired English and Korean documents preserve the same meaning when the
  changed document is bilingual

If a diagram cannot be kept accurate with a clear owner and review path, replace
it with prose or remove it from maintained documentation.
