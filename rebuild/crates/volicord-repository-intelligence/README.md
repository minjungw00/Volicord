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
affected gate-language scope. Local search returns inventory, entity, and
relation hits with source/range, capability, coverage, diagnostics, provenance,
and freshness; a historical range is never marked as current navigation.

The crate still has no semantic analyzer, external provider, child-process
analyzer, CLI, MCP surface, or canonical write path. Parser failure is a bounded
failed capability result rather than empty success, and languages outside the
first structural gate retain inventory-only fallback. No repository source is
transmitted externally.

All results are Derived State. The crate can reference `ProjectId` and `SourceId`
from `volicord-context`, but it cannot open or mutate the Canonical Context store.
Deleting its serialized output cannot delete or change Projects, Sources,
Questions, Decisions, Context Items, or Checkpoints.
