# Phase 5 Repository Intelligence conclusion

- Phase 6 implementation gate: `ready`
- Scope: the local Rust Repository Intelligence subsystem boundary,
  Repository/Analysis Snapshot identity, inventory, structural and selected
  semantic analysis, incremental structural refresh, grounded local search,
  explanation basis, capability/coverage/freshness reporting, and
  Project-scoped, basis-grounded read-only canonical linkage
- Excluded claim: this conclusion does not certify installation, Codex or other
  host integration, Inquiry, Recall, generated documents, background-provider
  shipping, the V11 multi-repository journey, or cutover acceptance

## Implemented subsystem responsibility

Repository Intelligence observes one Project-linked repository source as a
path-independent Repository Snapshot and produces rebuildable Analysis
Snapshots. Analyzer-independent inventory records files, languages, manifests,
configuration, documents, exclusions, unavailable areas, and ecosystem basis.
The common analysis envelope preserves snapshot identity, source/range,
capability, coverage, diagnostic, freshness, uncertainty, adapter/analyzer, and
provenance for every published result.

The subsystem owns only Derived State and provenance-bearing evidence
candidates. It may carry references to canonical Source, Decision, Context
Item, and Checkpoint targets only after validating target existence and
same-Project ownership against an immutable Canonical Context read basis.
Source references preserve the applicable canonical snapshot basis or an
explicit `not_applicable` state; Decision, Context Item, and Checkpoint
references preserve an exact revision. Current and historical revisions remain
distinguishable, and correction or Decision supersession does not silently
retarget an existing Analysis Snapshot.

Repository Intelligence has no canonical store handle or write operation and
cannot create, answer, revise, supersede, or forget a Question or Decision.
Consumers can reconstruct the single current Analysis Snapshot format and
revalidate every persisted canonical link before search or explanation output
is produced. Invalid linkage fails closed without canonical mutation. Analysis
refresh, deleting serialized analysis, and rebuilding from source preserve
canonical records and user correction; rebuildable Derived State owns no user
judgment.

## Structural and semantic approach

Structural analysis uses seven narrow in-process Tree-sitter adapters behind a
parser-independent normalized envelope:

- Java
- Python
- JavaScript
- TypeScript
- C
- C++
- Rust

Adapters publish parser-owned declarations, half-open zero-based line/UTF-8-byte
ranges, hierarchy and supported syntax relations. Parser errors, unsupported
constructs, and adapter failure remain explicit diagnostics and bounded
capability states. Inventory does not depend on parser success.

Semantic analysis is a distinct in-process source-semantic pass for
Java/Maven, TypeScript/Node, and Rust/Cargo. It publishes locally supported
`defines`, `references`, `resolves_to`, `type_of`, `implements`, and `overrides`
relations with semantic provenance. Scope and arity select a target only when
the local source basis is unambiguous; otherwise the target and reason remain
unresolved. This is not a compiler, LSP, SCIP, macro-expansion, or runtime
correctness claim.

## Maintained fixture and capability evidence

The shared manifest contains nine V01 inputs and three V02 ecosystem entries:

- seven official single-language structural fixtures;
- one Java/Python/TypeScript polyglot fixture;
- one out-of-set Go text fixture for inventory fallback; and
- Java/Maven, TypeScript/Node, and Rust/Cargo semantic entries.

The maintained Phase 5 acceptance harness maps 34 subsystem requirements to 38
Production Rust tests. It executes the library and inventory, structural, and
semantic integration targets rather than implementing analysis meaning in the
orchestrator. The evidence covers deterministic repeat, expected entities and
relations, source ranges, language/ecosystem context, analyzer-independent
inventory, polyglot failure isolation, semantic absence/failure, broken-build
remainder in all three selected ecosystems, same-name and overload distinction,
source-grounded search/explanation basis, canonical no-mutation, and Derived
State rebuild separation. Narrow canonical-grounding assertions additionally
cover Repository Source Project/snapshot basis, dangling and cross-Project
targets, impossible revisions, preserved historical revisions, correction and
supersession stability, persisted-snapshot read-side revalidation, every
automatic/manual reference ingress, and deterministic grounded-reference
serialization.

V01 is `passed` at this subsystem boundary. All seven structural languages pass
the maintained Production fixtures; same-snapshot identity and serialization
are stable; changed-file refresh reparses the changed TypeScript file and its
declared dependent while reusing an unaffected file; build-context change
invalidates the affected language scope; and one failed polyglot adapter does
not erase healthy languages or inventory.

V02 is `passed` for Java/Maven, TypeScript/Node, and Rust/Cargo at this subsystem
boundary. Each selected ecosystem retains source-ranged semantic results,
same-name distinction, deterministic rebuild, diagnostics, and usable remainder
for a broken local dependency. LSP, SCIP, and compiler-native alternatives were
not selected because they add ecosystem-specific distribution, build, and
child-process requirements without evidence needed by the current bounded
source-local contract.

## Degradation and freshness guarantees

Capability is reported by snapshot, language, area, and capability rather than
as one repository boolean. `available`, `partial`, `unavailable`, `unsupported`,
`failed`, and `stale` retain different reasons, affected scope, usable remainder,
coverage, diagnostics, and user-visible consequence. Excluded, ignored, vendor,
generated, binary, and unavailable inventory areas remain inspectable instead
of being counted as covered source.

A parser failure affects only its language/area; healthy inventory and other
languages remain usable. A missing or failed semantic adapter publishes no
semantic fact for its affected language and does not erase structural results.
Broken dependencies are partial rather than empty success. Search and grounded
explanation input expose their Repository/Analysis Snapshot, range, capability,
coverage, diagnostic, provenance, freshness, gaps, and uncertainty. A range
bound to an older Repository Snapshot is historical evidence and is never
marked as current navigation.

Structural refresh records file-content, declared-dependency, build-context,
adapter-contract, prior-failure, added, removed, and reuse outcomes. Unaffected
files are not unconditionally reparsed. The selected semantic pass rebuilds its
source-local index deterministically; this evidence does not claim a persistent
incremental semantic cache.

## Authority and privacy boundary

The nested workspace dependency is one-way: Repository Intelligence depends on
the Canonical Context Kernel only for typed identities and immutable read
bases, while the Kernel has no Repository Intelligence dependency. Neither
reconstruction crate depends on a legacy Volicord crate.

Production analysis is local and in-process. It contains no external provider,
network transport, child-process analyzer, CLI, MCP, viewer, or canonical write
path. Source-grounded explanation basis is available to a later host/projection
consumer, but this phase does not claim that host integration or generated
explanations ship. Because the selected Production adapters start no child
process, V10 is not triggered or claimed complete by Phase 5.

## Known limits

- Fixtures are small, deterministic, and self-authored. They do not establish
  population accuracy, large-repository scaling, resource ceilings, or
  unmeasured performance.
- Structural syntax does not fully model macro expansion, generated source,
  conditional build selection, annotation processing, template instantiation,
  reflection, dynamic dispatch, external services, or runtime-only state.
- The three semantic adapters resolve bounded local source evidence. They do not
  provide compiler-exact overload/generic/type/trait inference or external
  dependency bodies.
- Direct declared-dependency and conservative language build-context
  invalidation are demonstrated; a complete transitive build graph is not.
- An in-process native parser fault can terminate its process. Child-process
  containment and broader process recovery remain future V10/V11 evidence if a
  process-based adapter is introduced.
- Real repository usefulness, agent explanation quality, and combined product
  recovery remain V11 responsibilities.

## Decision and Phase 6 status

No maintained Phase 5 evidence activates a Q1-Q13 revisit trigger. In
particular, Q2 remains satisfied without reducing the seven-language structural
set, and the three selected semantic ecosystems satisfy its minimum semantic
gate within the documented limits. Q3 is not broadened: no background provider
or source transmission path exists in this subsystem.

Phase 6 entry is `ready`. Repository Intelligence now provides the bounded,
source-grounded fact, capability, coverage, freshness, and exact canonical
target basis that Inquiry, Checkpoint, and Recall may consume and revalidate
without granting analysis authority over user judgment. Phase 6 and later
validation must preserve that direction and complete their own acceptance
rather than treating this subsystem conclusion as V11 or product cutover
acceptance.

## Maintained references

- Active owners: `rebuild/docs/design/architecture.md`,
  `rebuild/docs/design/domain-model.md`,
  `rebuild/docs/design/repository-intelligence.md`,
  `rebuild/docs/design/privacy-and-provider-boundary.md`,
  `rebuild/docs/design/versioning-policy.md`, and
  `rebuild/docs/design/failure-and-recovery.md`.
- V01: `rebuild/validation/repository-intelligence/polyglot-structural/report.md`.
- Structural qualification:
  `rebuild/validation/repository-intelligence/production-structural-qualification/report.md`.
- V02: `rebuild/validation/repository-intelligence/semantic-qualification/report.md`.
- Phase 5 acceptance:
  `rebuild/validation/repository-intelligence/phase-5-acceptance/assertions.py`.
- Fixture authority: `rebuild/validation/shared/fixture-manifest.json`.
- Production boundary:
  `rebuild/crates/volicord-repository-intelligence/`.
