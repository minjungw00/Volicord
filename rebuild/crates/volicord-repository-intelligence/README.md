# Repository Intelligence

This crate owns Volicord's Production Repository Intelligence common boundary:
path-independent Repository Snapshots, analyzer-independent inventory,
Analysis Snapshots, capability/coverage/freshness reporting, normalized
structural and semantic envelopes, and source-grounded local search.

Production structural analysis uses a parser-independent common core with local
Tree-sitter adapters for Java, Python, JavaScript, TypeScript, C, C++, and Rust.
Adapters emit only parser-owned declarations and syntax relations, with explicit
partial diagnostics for syntax errors and constructs such as macros,
preprocessor conditionals, generated code, and dynamic behavior. Source ranges
are snapshot-bound, half-open, zero-based line/UTF-8-byte-column coordinates.
Entity and relation identities and canonical JSON are deterministic for the
same snapshot and adapter contract.

Each file records its content, dependency, build-context, adapter, and analyzer
basis. A refresh reparses added or changed files and declared dependents, reuses
unaffected current facts, and records explicit invalidation categories. A
manifest or recognized build-context change conservatively refreshes the
affected gate-language scope. Local search returns inventory, entity,
structural-relation, and semantic-relation hits with source/range, capability,
coverage, diagnostics, provenance, and freshness; a historical range is never
marked as current navigation.

Production semantic analysis is a distinct in-process source-semantic pass for
Java/Maven, TypeScript/Node, and Rust/Cargo. It normalizes local definitions,
references, declared types, explicit implementations, and override-style
relations, resolves overloads only when scope and arity identify one target,
and preserves unresolved targets, incomplete-build diagnostics, and usable
remainder. It has a separate adapter/analyzer identity and `semantic_result`
provenance from Tree-sitter structural facts. The current Analysis Snapshot
writer is format version 4 and remains deterministically rebuildable.

`CanonicalGrounding` derives an immutable identity-and-basis index from
Canonical Context's `CanonicalReadBasis`. Repository Sources must exist in the
same Project before a Repository Snapshot is constructed. Persisted Source
links carry their canonical snapshot basis or explicit `not_applicable` state;
Decision, Context Item, and Checkpoint links carry their exact revision.
Grounded explanation and local search validate the complete Analysis Snapshot
before copying any link. Analysis and refresh cannot open or mutate the
Canonical Context store, and a later correction or supersession does not
retarget an older Analysis Snapshot.

The crate still has no external provider, child-process analyzer, CLI, MCP
surface, or canonical write path. Parser/analyzer failure is a bounded failed
capability result rather than empty success, and unsupported languages retain
inventory/structural fallback. No repository source is transmitted externally,
and the in-process semantic selection does not trigger or claim V10.

All results are Derived State. The crate can reference typed canonical IDs from
`volicord-context`, but it cannot open or mutate the Canonical Context store.
Deleting its serialized output cannot delete or change Projects, Sources,
Questions, Decisions, Context Items, or Checkpoints.
