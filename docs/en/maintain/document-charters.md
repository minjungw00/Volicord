# Document charters

Use these charters when deciding what Volicord's major documents should own,
exclude, diagram, and link to over time. They complement
[Documentation policy](documentation-policy.md) and
[`docs/doc-index.yaml`](../../doc-index.yaml); they do not define product
behavior, public API behavior, storage effects, security guarantees, schemas,
Core authority semantics, conformance results, QA results, acceptance
decisions, close-readiness state, or residual-risk decisions.

The charters describe durable information ownership. If exact behavior matters,
link to the focused [Reference Index](../reference/README.md) owner instead of
copying a second contract into an onboarding, guide, development, or Maintain
page.

## General Use

- Use each charter as a scope check before adding detail to a high-traffic
  document.
- Keep examples and diagrams at the document's reader level. Explain what a
  diagram answers in natural prose or a caption, and link to Reference owners
  for exact behavior. Do not expose review labels such as `Diagram role:` in
  ordinary reader-facing documents.
- Prefer links at the moment a reader needs deeper precision. Avoid turning a
  landing page into a complete reference index.
- Keep maintenance wording durable. Do not add task history, migration notes,
  scratch plans, generated records, or implementation logs.

## Root README

Document: [Root README](../../../README.md)

- Primary goal: Present Volicord as a product landing overview that orients a
  first reader and supports a real first setup path. It is not just a
  documentation router and not a reference manual.
- Intended reader: New users, operators, agent integrators, and source-code
  learners who need the product story before choosing a deeper path.
- Should own: Problem framing, user value in ordinary language, a concrete
  scenario, quick start, beginner concept introductions, user workflow diagram,
  local component map, Core boundaries at overview level, what Volicord manages
  and does not manage, and navigation to deeper documents.
- Should not own: Full CLI option references, full host matrices, storage DDL,
  MCP preflight line lists, complete troubleshooting catalogs, full API method
  references, or exact security guarantee wording.
- Beginner concept rule: Introduce first-read meanings before workflow prose or
  diagrams depend on Volicord-specific terms such as `Volicord Runtime Home`,
  `Agent Connection`, `volicord mcp --stdio`, and `User Channel`.
- Acceptable diagrams: Guide-level user workflow and local component maps.
  The root README should include both, with surrounding prose that naturally
  explains what each diagram shows and what it omits. Avoid presenting a
  complete API call sequence, storage layout, or contract boundary map, and do
  not expose diagram-role metadata as reader-facing labels.
- Link deeper by: Sending setup detail to [Installation](../getting-started/installation.md),
  first host use to [Quickstart](../getting-started/quickstart.md), workflow
  practice to [User Workflow](../guides/user-workflow.md) and
  [Agent Workflow](../guides/agent-workflow.md), host operations to
  [Agent Host Setup](../guides/agent-host-setup.md), and exact behavior to
  [Administrative CLI](../reference/admin-cli.md),
  [MCP Transport](../reference/mcp-transport.md),
  [Core Model](../reference/core-model.md),
  [Agent Connection](../reference/agent-connection.md), and
  [Runtime Boundaries](../reference/runtime-boundaries.md).

## Getting Started Overview

Document: [Getting Started Overview](../getting-started/overview.md)

- Primary goal: Give the first-read product identity and local authority model
  before the reader installs or operates Volicord.
- Intended reader: New users, agents, and implementers who need a conceptual
  orientation.
- Should own: Volicord's public product identity, the Volicord/Core distinction
  at first-read depth, the reason local authority records matter, and
  overview-level routes to scope and Reference owners.
- Should not own: Installation commands, host setup procedures, CLI option
  semantics, API method behavior, storage tables, or security guarantee detail.
- Acceptable diagrams: Optional concept maps that show the product-local
  authority relationship or reader routes. Do not use diagrams as contract
  definitions.
- Link deeper by: Routing supported and unsupported scope to
  [Scope](../reference/scope.md), authority concepts to
  [Core Model](../reference/core-model.md), runtime placement to
  [Runtime Boundaries](../reference/runtime-boundaries.md), security claims to
  [Security](../reference/security.md), terminology to
  [Glossary](../reference/glossary.md), and setup to
  [Installation](../getting-started/installation.md).

## Installation

Document: [Installation](../getting-started/installation.md)

- Primary goal: Lead readers through installing, finding, and verifying the
  local executable.
- Intended reader: New users, operators, and implementers preparing a local
  Volicord installation.
- Should own: Tutorial-level prerequisites, release binary install flow,
  development source build path, guided setup prompts and action-required
  guidance at onboarding depth, executable discovery, installation profile
  preparation, deterministic setup options for automation, and verification
  checkpoints.
- Should not own: Environment applicability classifications, full
  administrative CLI contracts, host configuration procedures, exhaustive
  recovery guidance, MCP protocol behavior, or storage effects.
- Acceptable diagrams: Usually none. A compact install-to-verify sequence is
  acceptable if it makes the tutorial clearer.
- Link deeper by: Sending environment classification to
  [System Requirements](../reference/system-requirements.md), command
  semantics to [Administrative CLI](../reference/admin-cli.md), MCP process
  requirements to [MCP Transport](../reference/mcp-transport.md), and first host
  use to [Quickstart](../getting-started/quickstart.md).

## Quickstart

Document: [Quickstart](../getting-started/quickstart.md)

- Primary goal: Provide the shortest real path from verified executables to one
  successful supported agent-host setup.
- Intended reader: New users, operators, and agent integrators who want a first
  working connection.
- Should own: A focused happy path, minimal commands, setup prompt or
  action-required handoff at first-run depth, expected success checks, and the
  next document when the happy path stops.
- Should not own: Full host matrices, all setup and removal variants, complete
  CLI flag behavior, complete troubleshooting catalogs, MCP transport
  contracts, or API method references.
- Acceptable diagrams: Optional linear step diagrams. Avoid full component maps
  or troubleshooting decision trees.
- Link deeper by: Sending executable preparation to
  [Installation](../getting-started/installation.md), complete host operations
  to [Agent Host Setup](../guides/agent-host-setup.md), stalled setup to
  [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md), and
  exact command or process behavior to [Administrative CLI](../reference/admin-cli.md)
  and [MCP Transport](../reference/mcp-transport.md).

## Agent Host Setup Guide

Document: [Agent Host Setup](../guides/agent-host-setup.md)

- Primary goal: Explain how to install, verify, guide, operate, and remove
  supported agent-host integrations.
- Intended reader: Operators, agent integrators, and agents working with Codex,
  Claude Code, or generic MCP configuration.
- Should own: Operator choices, supported setup paths, guide-level preflight
  checks, verification and status flows, managed guidance boundaries, and safe
  removal flow.
- Should not own: Exact CLI output contracts, full troubleshooting catalogs,
  MCP protocol line lists, host-internal trust behavior, full environment
  applicability rules, storage DDL, or API schema meaning.
- Acceptable diagrams: Host setup flow diagrams, configuration boundary maps,
  and verification path diagrams that stay at guide level.
- Link deeper by: Sending command detail to [Administrative CLI](../reference/admin-cli.md),
  local MCP process detail to [MCP Transport](../reference/mcp-transport.md),
  location and repository boundaries to
  [Runtime Boundaries](../reference/runtime-boundaries.md), connection concepts
  to [Agent Connection](../reference/agent-connection.md), API surfaces to
  [API Methods](../reference/api/methods.md), and recovery detail to
  [Agent Host Troubleshooting](../guides/agent-host-troubleshooting.md).

## Agent Workflow Guide

Document: [Agent Workflow](../guides/agent-workflow.md)

- Primary goal: Explain how agents work against Volicord's authority
  boundaries during product work.
- Intended reader: Agents, agent operators, and maintainers reviewing
  agent-facing workflow guidance.
- Should own: Authority-aware work sequencing, when to use Volicord APIs at
  guide level, evidence practices, user-judgment request flow, write-readiness
  boundaries, and close-readiness expectations for agents.
- Should not own: Public API method contracts, schema definitions, storage
  effects, user-owned decision content, security guarantees, or host setup
  instructions.
- Acceptable diagrams: Agent workflow loops, handoff diagrams, and guide-level
  state-progress diagrams. Do not replace method reference sequences.
- Link deeper by: Sending exact API behavior to
  [API Methods](../reference/api/methods.md), authority concepts to
  [Core Model](../reference/core-model.md), connection context to
  [Agent Connection](../reference/agent-connection.md), user-facing practice to
  [User Workflow](../guides/user-workflow.md), and method-specific behavior to
  the focused method owners under `docs/en/reference/api/`.

## User Workflow Guide

Document: [User Workflow](../guides/user-workflow.md)

- Primary goal: Help users collaborate with agents while preserving
  user-owned judgment and visible work records.
- Intended reader: Product users, agents supporting those users, and operators
  explaining the user-facing workflow.
- Should own: User mental model, practical collaboration loop, judgment
  boundaries, evidence visibility, when the user must answer, and guide-level
  close-readiness interpretation.
- Should not own: API method contracts, CLI option detail, host setup
  procedures, exact schema fields, storage behavior, or security guarantees.
- Acceptable diagrams: User decision loops, role-boundary diagrams, and
  handoff diagrams. Avoid component maps that belong in setup or architecture
  documents.
- Link deeper by: Sending exact authority concepts to
  [Core Model](../reference/core-model.md), example decisions to
  [Judgment Examples](../guides/judgment-examples.md), agent-facing procedure to
  [Agent Workflow](../guides/agent-workflow.md), and setup questions to
  [Agent Host Setup](../guides/agent-host-setup.md).

## Reference Documents

Document family: [Reference Index](../reference/README.md) and focused
Reference owners.

- Primary goal: Own exact product contracts and route readers to focused owners.
- Intended reader: Implementers, agent integrators, agents, maintainers, and
  reviewers who need precise behavior.
- Should own: Exact contracts for scope, API methods, schemas, error behavior,
  storage, runtime boundaries, security wording, Agent Connection behavior,
  projection and template behavior, conformance meaning, and design-quality
  meaning. The Reference Index owns human-readable Reference navigation.
- Should not own: Tutorial flow, marketing framing, implementation history,
  migration notes, broad onboarding prose, or duplicate copies of adjacent
  Reference owners.
- Acceptable diagrams: Precise component, state, sequence, data-relation, or
  routing diagrams only when they are supported by the focused contract and
  stay within that owner's scope.
- Link deeper by: Linking to adjacent Reference owners for neighboring
  contracts and to onboarding or guide documents only for reader context, not
  to define exact behavior outside Reference.

## Architecture And Development Documents

Document family: [Developer Documentation](../development/README.md),
[Architecture](../development/architecture.md), and related Development pages.

- Primary goal: Teach the durable implementation shape and how to make changes
  without turning implementation explanation into product contract text.
- Intended reader: Implementers, reviewers, and source-code learners.
- Should own: Durable crate and module roles, request flow, architecture
  decisions, source-learning routes, design patterns, storage and transaction
  architecture, testing strategy, and implementation change workflow.
- Should not own: Product behavior contracts, API schema meaning, storage DDL
  contracts, storage-effect contracts, security guarantees, user-judgment
  authority semantics, or line-number-dependent helper catalogs.
- Acceptable diagrams: Architecture maps, module responsibility diagrams,
  lifecycle diagrams, request sequences, and implementation decision diagrams
  that teach stable code structure.
- Link deeper by: Sending exact product behavior to the applicable
  [Reference Index](../reference/README.md) owner, Rust edit workflow to
  [Change Guide](../development/change-guide.md), validation responsibilities
  to [Testing Strategy](../development/testing-strategy.md), and working rules
  to repository `AGENTS.md` files.

## Maintain Documents

Document family: [Documentation policy](documentation-policy.md),
[Translation policy](translation-policy.md), [Diagram Policy](diagram-policy.md),
[Brand Guidelines](brand-guidelines.md), [Validation](validation.md), this page,
[`docs/doc-index.yaml`](../../doc-index.yaml), and
[`docs/terminology-map.yaml`](../../terminology-map.yaml).

- Primary goal: Help authors, translators, reviewers, and agents keep the
  documentation set accurate, scoped, discoverable, and bilingual by meaning.
- Intended reader: Documentation maintainers, translators, reviewers, agents,
  and implementers doing documentation-adjacent work.
- Should own: Documentation governance, metadata use, terminology controls,
  translation expectations, diagram category and caption rules, validation
  checks, brand presentation, document charters, and repository working-rule
  interaction.
- Should not own: Product behavior, public API behavior, storage effects,
  security guarantees, schemas, Core authority semantics, runtime homes,
  generated records, QA results, acceptance decisions, close-readiness state,
  residual-risk records, or task-specific work notes.
- Acceptable diagrams: Rare route maps or process diagrams for maintenance
  work. Avoid diagrams that imply runtime, API, storage, or security behavior.
- Link deeper by: Sending exact product questions to the focused
  [Reference Index](../reference/README.md) owner, metadata routing to
  [`docs/doc-index.yaml`](../../doc-index.yaml), terminology to
  [`docs/terminology-map.yaml`](../../terminology-map.yaml), and scoped editing
  rules to root and directory `AGENTS.md` files.
