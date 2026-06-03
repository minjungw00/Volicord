# Rewrite Plan

## What this document helps you do

Use this plan when you classify future Harness documentation rewrite work.

It gives maintainers and agents a shared way to decide whether old prose should be preserved, shrunk, moved, deleted, or routed for a real decision.

This is Maintain documentation. It governs documentation rewrite planning only. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, product-state changes, conformance results, evidence records, QA records, work acceptance, close readiness, or residual-risk records.

## Read this when

- You are planning a redesign pass over old Harness documentation.
- You need to decide what to do with existing wording, sections, tables, or document structure.
- You found prose that may conflict with the clarified product thesis, owner boundaries, Korean quality rules, stage boundaries, or honest security wording.

## Before you read

Read `AGENTS.md` first. Before documentation edits, read [Authoring Guide](authoring-guide.md). Before bilingual or terminology-affecting edits, read [Translation Guide](translation-guide.md). Before touching Korean docs, read the Korean [문서 작성 가이드](../../ko/maintain/authoring-guide.md) and [번역 가이드](../../ko/maintain/translation-guide.md).

## Main idea

Preserve the core principles, not the old document shape.

Old structure and old prose do not need to survive when the principles, owner boundaries, reader purpose, and English/Korean semantic parity are preserved. A good rewrite may move, merge, compress, or delete existing material.

## Rewrite categories

Use these categories for rewrite findings and batch notes.

| Category | Use when | Rewrite action |
|---|---|---|
| `preserve` | The text already supports the product thesis, has the right owner, and helps the intended reader. | Keep the meaning. Wording may still be polished for clarity, parity, and Korean quality. |
| `shrink` | The text is directionally right but too long, repetitive, too internal, or too contract-heavy for its document family. | Compress to the reader-visible consequence and link to the owner document for exact rules. |
| `move` | The text belongs in another document family or owner Reference document. | Move the meaning to the owner path, replace the old location with a short route or remove it, and update English/Korean links together. |
| `delete` | The text is obsolete, misleading, duplicative, or conflicts with the product thesis, current stage model, owner boundary, Korean quality rules, or security guarantee level. | Remove it. Do not keep prose only for continuity. |
| `decision-needed` | The rewrite exposes a real unresolved choice about schema, state, API, stage boundary, security guarantee, fixture semantics, terminology, or implementation readiness. | Route the decision to the owning document. Major server-coding decisions belong in the MVP Plan decision-log sections, not scattered TODOs. |

## Principles to preserve

All rewrite categories must preserve these principles:

- Harness is a local authority record for scope, user-owned judgment, evidence, verification expectations, work acceptance, close readiness, and residual risk.
- Users should not need to know Harness-internal terms to use it.
- Agents must not silently replace user-owned product, technical, security/privacy, work-acceptance, or residual-risk judgments.
- Rendered Markdown, projections, summaries, and conversation history are not the operational source of truth.
- Early Harness documentation must not imply sandboxing, OS permission isolation, tamper-proof files, or pre-execution security blocking unless a proven mechanism is documented.

## Rewrite rules

During redesign, optimize for clarity, implementability, and the Harness thesis.

Do not treat documentation pages as Harness runtime objects. Do not create runtime state, Write Authorizations, Evidence Manifests, Manual QA records, Acceptance records, Residual Risk records, generated projections, operational outputs, fixture files, conformance runners, or product-repository examples for documentation edits.

Keep strict contracts in their owner Reference docs. Non-owner docs should summarize the reader-visible consequence and link to the owner.

When a semantic change starts in English, mirror it in Korean in the same batch. When a Korean change reveals a meaning issue, reflect the meaning back into English.

## Korean rewrite rules

Korean documentation should preserve the same meaning without becoming a line-by-line translation.

- Write natural Korean technical prose.
- Prefer short sentences.
- Avoid Korean sentences that are mostly English nouns with Korean particles attached.
- Keep API names, schema names, field names, enum values, file paths, and literal identifiers unchanged.
- Put natural Korean first in user-facing prose. Add exact Harness labels only when they help precision, search, or owner-link alignment.
