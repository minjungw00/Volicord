# Brand guidelines

This maintenance owner defines how the Volicord brand is named and presented in
repository documentation, README files, CLI/MCP instructions, generated
repository guidance, and future authoring work.

It does not define runtime behavior, public API behavior, storage behavior,
security guarantees, schemas, or Core authority semantics. When exact product
behavior matters, route to the Reference owners listed in [Owner routes](#owner-routes).

## Official Copy

| Item | Official form |
|---|---|
| Brand name | `Volicord` |
| Korean pronunciation | `볼리코드` |
| First mention in an independent Korean entry document | `Volicord(볼리코드)` |
| Brand mnemonic | `Volition, recorded.` |
| English tagline | `AI moves. Judgment stays yours.` |
| Korean tagline | `AI가 움직여도, 판단은 사용자에게.` |
| English product descriptor | `A local work authority record for AI-assisted product work.` |
| Korean product descriptor | `AI 지원 제품 작업을 위한 로컬 작업 권한 기록` |

Core messages:

| English | Korean |
|---|---|
| Scope stays explicit. | 범위는 분명하게. |
| Judgment stays with the user. | 판단은 사용자에게. |
| Evidence stays visible. | 근거는 보이게. |
| Closure stays honest. | 닫기는 정직하게. |

Use the tagline and core messages for brand, onboarding, and presentation
contexts where they help readers orient to Volicord. Do not insert the tagline
into operational Reference contracts, error messages, or routine CLI output.

## Spelling Rules

- Use `Volicord` for the brand.
- Use `volicord` only for exact lowercase technical identifiers.
- Do not use `VoliCord`, `Voli Cord`, `VOLI-CORD`, `Voli`, or `VC` as product
  names or abbreviations.
- Preserve exact identifiers, file paths, API methods, schema names, field
  names, status values, product labels, and code literals required by the
  [Terminology Map](../../terminology-map.yaml) and the applicable owner
  document.

## Product And Component Presentation

- Volicord is the product/system brand.
- Core remains a product concept and authority-record role. Do not rename Core
  or collapse Core's exact state-authority role into the public Volicord product
  identity.
- `volicord` is the administrative CLI identifier. Exact CLI behavior belongs to
  [Administrative CLI](../reference/admin-cli.md).
- `volicord mcp --stdio` is the local MCP adapter process identifier. Exact MCP process,
  transport, and response-wrapping behavior belongs to [MCP Transport](../reference/mcp-transport.md).
- `Volicord Runtime Home` is a product label. Exact runtime location and
  repository-boundary behavior belongs to [Runtime Boundaries](../reference/runtime-boundaries.md).
- Domain concepts such as `Task`, Write Ticket, User Judgment, Evidence,
  Close Status, final acceptance, and residual-risk acceptance must not be given
  decorative Volicord-derived names.

When a Reference owner uses an exact identifier or product label, preserve that
owner-defined string at the point of use. Brand presentation does not silently
change API methods, binaries, storage identifiers, environment variables, file
paths, or schema values.

## Voice And Claim Boundaries

Prefer precise verbs such as record, distinguish, preserve, show, verify, and
identify.

Restrict broad claims using control, guarantee, secure, protect, monitor,
approve, or decide unless the applicable contract owner supports the exact
claim. Volicord presentation must not imply stronger scope, security, runtime,
or Core authority guarantees than the linked Reference owner defines.

Keep administrative acceptance, managed configuration, behavioral connection
verification, and operational session authorization separate. A command can accept
`host_kind=codex`, a setup path can meet its prerequisites, and a current
managed MCP runtime/project session can authorize an allowed project call.
Each fact remains within that named scope. Configuration presence, a passing
fixture, or a terminal-side MCP check cannot substitute for the current
managed-host observations or session validation. Those observations do not
certify executable identity, provenance, or future host behavior. Route environment and setup applicability to
[System Requirements](../reference/system-requirements.md), operational session
and behavioral verification meaning to
[Agent Connection](../reference/agent-connection.md#validated-agent-session).

Do not describe Volicord as making user-owned judgments. Volicord can help
record, route, preserve, or show the boundary where the user's judgment is
needed, but the judgment remains user-owned.

Do not describe Volicord authority records as OS-level enforcement, a sandbox,
or proof that an agent followed instructions. Use the
[Product and Maintenance Charter](product-maintenance-charter.md) for the
durable product-identity boundary, and route exact guarantee wording to the
focused Reference owners.

Do not merge test success, write approval, final acceptance, and
residual-risk acceptance into one generic approval. Keep those concepts
separate and route exact meaning to [Core Model](../reference/core-model.md) and
the relevant API owners.

## Visual Principles

These are project-local visual identity principles. They do not create a logo,
icon, asset library, trademark plan, website, release plan, UI contract, or
runtime behavior.

Prefer visual systems that show explicit separation, records, decision
branches, and visible state boundaries.

Avoid shield, lock, surveillance-eye, military-control, robot-versus-human,
cable, and single-checkmark motifs.

Do not use color as the sole status carrier.

Initial project tokens:

| Token | Value |
|---|---|
| Base ink | `#171A21` |
| Base background | `#F6F4EE` |
| Volicord indigo | `#3F4FD8` |
| Secondary gray | `#68707D` |

## Test Harness Term Boundary

`Volicord` is the product brand. The ordinary technical term `test harness`
refers to testing infrastructure and must not be used as a product name or
abbreviation for Volicord.

Use `test harness` only when discussing a general testing fixture, runner, or
support system. In Korean, use `테스트 하네스` only for that ordinary technical
term. Do not translate `Volicord` as `test harness` or `테스트 하네스`.

## Owner Routes

Use these owners for exact behavior and guarantee questions instead of copying
their contracts into brand material:

| Question | Owner |
|---|---|
| Product scope and supported baseline boundaries | [Scope](../reference/scope.md) |
| Work authority, User Judgment, Evidence, Write Ticket, acceptance, residual risk, and Close Status | [Core Model](../reference/core-model.md) |
| Runtime locations, product repository boundaries, Runtime Home boundaries, and component/location separation | [Runtime Boundaries](../reference/runtime-boundaries.md) |
| Security wording, guarantee levels, invocation-context assumptions, and explicit non-guarantees | [Security](../reference/security.md) |
| Administrative CLI commands, arguments, output, host setup, and command/API boundary | [Administrative CLI](../reference/admin-cli.md) |
| Local MCP adapter process startup, stdio transport, protocol handling, and response wrapping | [MCP Transport](../reference/mcp-transport.md) |
| Environment applicability and prerequisites for an accepted `HOST` value, configuration, or setup path | [System Requirements](../reference/system-requirements.md) |
| Current Agent Connection behavioral verification and operational-session authorization | [Agent Connection](../reference/agent-connection.md#validated-agent-session) |
| Documentation owner routing and metadata | [Documentation Policy](documentation-policy.md), [doc-index.yaml](../../doc-index.yaml) |
| Bilingual terminology and identifier preservation | [Translation Policy](translation-policy.md), [Terminology Map](../../terminology-map.yaml) |
