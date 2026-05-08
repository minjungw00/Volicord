# Codex Working Rules

This repo is in pre-MVP Harness documentation redesign mode. Keep this file as a short always-on compass, not a procedure manual, schema reference, or project history.

- Do not implement the harness server, runtime code, generated operational files, or product implementation yet.
- Work in `docs/en` first; mirror semantic documentation changes in `docs/ko` in the same batch.
- Keep source-of-truth boundaries strict: operational state in `state.sqlite` current records plus `state.sqlite.task_events`, raw evidence in the artifact store, Markdown projections as derived views, MCP schemas in `docs/*/05-mcp-api-and-schemas.md`, SQLite DDL in `docs/*/06-reference-mvp.md`, kernel transitions in `docs/*/03-kernel-spec.md`, projection rules in `docs/*/07-document-projection.md`, and full template bodies in `docs/*/appendix/A-template-library.md`.
- Before significant work resumes, read current Harness status and show the current Journey Card. If MCP is unavailable under an explicit docs-authoring override, show “Preflight status” instead.
- Use Decision Packets for blocking product judgment; do not ask for broad approval. Ask one blocking question at a time, with recommendation and uncertainty when available.
- Before product writes, call `prepare_write` and show the Write Authority summary. The Autonomy Boundary is judgment latitude, not write authority.
- AFK work is allowed only inside the active scoped Change Unit, Autonomy Boundary, and any granted sensitive approval that applies.
- If MCP is unavailable, hold product/runtime/code writes.
- Use small batches and report changed files.

## Docs-Authoring Override

A one-time `DOCS_AUTHORING_OVERRIDE` may permit pre-MVP documentation edits when a working Harness MCP surface is unavailable. It must list exact allowed paths, permits only those documentation edits, and is not `prepare_write`, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, task close, or a canonical state transition.

Do not use this exception for runtime code, product implementation code, generated operational files, secrets, destructive writes, or broad refactors. If no explicit override is present, hold writes.
