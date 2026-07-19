# Change Guide

Use this guide to route implementation work. Product behavior is defined by the
focused Reference owner, not by current code or this guide.

## Before Editing

1. Read the repository and nearest scoped `AGENTS.md`.
2. Use [`docs/doc-index.yaml`](../../doc-index.yaml) to find the focused
   English/Korean owner pair.
3. Read [Architecture](architecture.md) and this guide before changing durable
   Rust structure.
4. Inspect the worktree and preserve unrelated user changes.
5. If an owner does not define required behavior, update that owner first or
   report the gap.

## Route By Change Type

| Change | Start in implementation | Required owner review |
|---|---|---|
| Public method, request, response, error, or value | `volicord-types`, then Core | API method/schema and Failure Model |
| Planning, policy, replay, or authority | `volicord-core` | focused API, Core Model, Storage Effects |
| DDL, strict stored record, or transaction effect | `volicord-store` | Storage DDL, Records, Effects, Versioning |
| MCP lifecycle, decoding, tool list, or projection | `volicord-mcp` | MCP Transport and API owners |
| Administrative command or CLI inbox | `volicord-cli` | Administrative CLI and User Action owners |
| Codex setup or verification | Codex adapter and connection command | Agent Connection, Security, System Requirements |
| Release build or package integrity | `tests/release-integrity`, release workflow | Validation |
| Documentation route or terminology | `docs/doc-index.yaml`, paired docs | documentation and translation policies |

The first-release adapter surface is Codex Record profile with `personal` and
`shared` managed stdio connections. Do not add another adapter, profile,
transport, or user-action resolution channel without an explicit owner change.

## Keep Boundaries Intact

- CLI and MCP adapters may call Core-facing interfaces; Core must not depend on
  adapter internals.
- Store validates strict persisted owner records before use and applies
  owner-defined effects atomically.
- Public adapters reject hidden invocation context; server-owned context is
  derived locally.
- MCP may create or resume a UserAction request. Only the CLI inbox resolves it.
- Guard prompt capture is observation only.
- Generated documentation changes through its source and generator.

## Validation

For Rust changes, run from the workspace unless the task clearly supports a
narrower crate command:

```sh
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-targets --all-features
```

For documentation changes, run the checks in
[Validation](../maintain/validation.md), including:

```sh
cargo run -p xtask -- docs-check
git diff --check
```

Release changes also require the generic release-integrity package and the
ordinary build, package, checksum, platform, and workflow checks applicable to
the change. A real-Codex smoke run is an optional operational observation for
the current configuration and environment; version changes renew that
observation, while managed-call authorization remains session-bound.

## Handoff

Report changed files, validation and results, skipped checks with reasons, and
remaining risks or out-of-scope findings. Do not write work logs or validation
output into maintained documentation.
