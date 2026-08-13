# Repository Intelligence

This crate owns Volicord's Production Repository Intelligence common boundary:
path-independent Repository Snapshots, analyzer-independent inventory,
Analysis Snapshots, capability/coverage/freshness reporting, and normalized
structural and semantic envelopes.

The current implementation intentionally performs inventory only. It recognizes
the seven accepted structural-gate languages and auxiliary text formats, records
manifest and ecosystem evidence, and reports structural, semantic, and
agent-assisted capabilities honestly when no adapter is installed. It does not
contain a structural parser, semantic analyzer, provider, CLI, MCP surface, or
canonical write path.

All results are Derived State. The crate can reference `ProjectId` and `SourceId`
from `volicord-context`, but it cannot open or mutate the Canonical Context store.
Deleting its serialized output cannot delete or change Projects, Sources,
Questions, Decisions, Context Items, or Checkpoints.
