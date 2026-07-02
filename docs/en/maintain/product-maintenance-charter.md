# Product and maintenance charter

Use this charter when planning durable Volicord service direction,
documentation roles, implementation guidance, tests, brand language, or
translation policy. It complements [Documentation Policy](documentation-policy.md),
[Document Charters](document-charters.md), [Brand Guidelines](brand-guidelines.md),
[Translation Policy](translation-policy.md), [Validation](validation.md), and
the focused [Reference Index](../reference/README.md).

This is a maintenance charter. It does not define product behavior, public API
behavior, storage effects, security guarantees, runtime behavior, schemas, Core
authority semantics, conformance results, QA results, acceptance decisions,
close-readiness state, or residual-risk decisions.

## Product Identity

- Volicord is a local work authority record for AI-assisted product work.
- Core is the local authority record for Volicord state. Do not collapse Core's
  exact state-authority role into the public Volicord product identity.
- User judgment remains distinct from agent action. Volicord may record,
  route, preserve, or show where judgment is needed, but it must not be
  presented as making user-owned decisions.
- Authority records are not OS-level enforcement. They are local records and
  workflow controls, not a sandbox, command monitor, file-system isolation
  layer, network isolation layer, security proof, or guarantee that an agent
  followed instructions.
- Scope, evidence, write ticket, write approval, final acceptance,
  residual-risk acceptance, and Close Status stay separate in planning,
  documentation, code, tests, and reports.

## Service Planning Principles

Plan service work around explicit scope, visible evidence, user-owned judgment,
recorded state transitions, and honest Close Status checks. Features should
make those boundaries easier for users and agents to see, not hide them behind
a polished summary.

Host integrations and observation surfaces are cooperative and detective unless a
focused Reference owner defines a stronger guarantee. Product language,
implementation comments, tests, CLI help, generated guidance, and examples must
not imply stronger protection than the relevant Reference owner supports.

Before Volicord is ready for a first major release, the repository does not
need to preserve legacy CLI, API, storage, fixture, or documentation
compatibility when goal-directed design work needs a cleaner product model.
When older shapes conflict with the intended model, update the focused
Reference owner, implementation, examples, and maintained documentation
together instead of preserving older behavior only because it existed before.

Non-goals for service planning include replacing the editor, shell, tests,
code review, or user judgment; acting as an OS security boundary; proving that
all writes are safe; and making agent summaries into Core authority records.

## Documentation Roles

| Category | Intended role |
|---|---|
| [Root README](../../../README.md) | First-user foundation and basic product explanation. It should orient readers, show the normal first path, and introduce beginner concepts. It must not become a pure link router or a full Reference manual. |
| Maintain documentation | Product principles, branding, service planning goals, code guidance, documentation style, testing philosophy, translation policy, validation boundaries, and owner-routing practice. It guides maintainers without redefining exact product contracts. |
| User documentation | Practical usage help for installation, setup, ordinary workflow, troubleshooting, user-agent collaboration, and examples. It may summarize behavior for users and should link to Reference when exact behavior matters. |
| Contract and Reference documentation | Public behavior, supported scope, API, storage, transport, security boundaries, schemas, value meanings, error behavior, and other exact product contracts. |
| Architecture Guide documentation | Architecture and implementation understanding beyond contract detail: source structure, request flow, design patterns, testing strategy, change workflow, and durable implementation rationale. |

Use [Document Charters](document-charters.md) for detailed ownership rules for
major document families.

## Code Guidance

Implementation work is contract-first. If a code change needs behavior that no
focused owner defines, update the applicable Reference owner first or report
the owner gap. Do not make code, tests, fixtures, examples, generated output,
CLI help, or implementation comments the only place where product behavior is
defined.

Core-facing code stays independent of CLI and MCP adapter layers. CLI and MCP
adapters may call Core-facing interfaces. Code should make effect paths,
state transitions, no-effect branches, user-judgment routing, and close blockers
easy to reason about.

Architecture Guide should track durable source structure. Update the
applicable [Architecture Guide](../architecture-guide/README.md) when an
implementation change durably changes crate roles, module responsibilities,
request flow, or testing strategy.

## Quality Gates And Length

File length, document length, and LOC counts must not be hard quality gates for
Volicord code or documentation. A number cannot prove that ownership is clear,
contracts are correct, examples are accurate, state transitions are tested, or a
first reader can complete the workflow.

Hard length limits create the wrong incentive: splitting focused material only
to satisfy a count can scatter owner-defined contracts, duplicate explanations,
and make reader paths harder to follow. Refactor long files or documents when
the split improves ownership, readability, reviewability, or source structure,
not because a line count was exceeded.

Use quality controls that match the risk: owner routing, contract accuracy,
reader usability, bilingual semantic parity, link integrity, source
consistency, durable tests, and validation commands.

## Testing Philosophy

Repository tests should validate durable behavior, contracts, state
transitions, user value, stable abstraction boundaries, and maintained
documentation rules. Tests should be named after the current product behavior
or maintenance rule they protect.

One-time cleanup tests must not be committed. A search that only proves an old
term, flag, field, example, or implementation detail disappeared is an audit for
the change process, not a durable repository test. If the absence matters as a
lasting contract, test the positive current shape instead, such as the current
public CLI option set, current storage schema, current MCP-visible schema, or
current terminology role metadata.

Test results are implementation or maintenance checks. They do not define
product contracts, prove security, complete QA, establish Close Status,
record final acceptance, or accept residual risk. Use [Validation](validation.md)
and [Testing Strategy](../architecture-guide/testing-strategy.md) for placement and
reporting boundaries.

## Brand And Translation

Use [Brand Guidelines](brand-guidelines.md) for Volicord spelling, official
bilingual copy, component presentation, visual principles, and claim
boundaries. Brand language must keep judgment with the user and must not make
authority records sound like OS-level enforcement.

Use [Translation Policy](translation-policy.md) and
[`docs/terminology-map.yaml`](../../terminology-map.yaml) for bilingual
semantic parity, natural Korean technical prose, identifier preservation, and
terminology routing. English and Korean Maintain documents must carry the same
principles by meaning unit, including pre-major compatibility guidance, length
gate rejection, durable test philosophy, and user-judgment boundaries.

## Maintainer Completion Standard

When a maintained path, role, owner, or route changes, update
[`docs/doc-index.yaml`](../../doc-index.yaml), paired-language routes, and
reader navigation in the same change. Run the applicable documentation
validation and report results in the conversation, not repository files.

Do not leave task-specific plans, work logs, generated records, runtime homes,
SQLite files, generated logs, archive copies, or local runtime output in
maintained documentation.
