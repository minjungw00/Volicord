# Translation Guide

## What this guide helps you do

Use this guide when editing English and Korean Harness documentation together.

This is maintenance guidance for bilingual documentation. It does not define runtime behavior, conformance pass/fail, generated outputs, evidence, QA, acceptance, close readiness, or implementation readiness.

## Read this when

- You are changing meaning in an English or Korean doc.
- You are reviewing English/Korean semantic parity.
- You need to decide whether Korean wording should preserve an English identifier or use natural Korean prose.

## Before you read

Read [Authoring Guide](authoring-guide.md) for owner boundaries, docs-maintenance checks, and the rule that strict contracts stay in Reference docs.

## Main idea

The goal is semantic parity, not sentence-by-sentence translation. Korean should read like natural technical Korean while preserving official identifiers, exact contracts, and product terms.

In user-facing Korean, prefer the natural phrase first and add the exact Harness term only when the label helps precision. For example, use `범위(Change Unit)`, `판단 자료(Decision Packet)`, `쓰기 권한(Write Authorization)`, `남은 위험(residual risk)`, or `결과 수락(final acceptance)` when both the reader-friendly phrase and the Harness label matter.

## Keep exact

Keep these unchanged across English and Korean docs:

- API names
- schema names
- enum values
- DDL names
- code identifiers
- file names and path names
- error codes and validator IDs

Keep these stable product terms exact when they refer to Harness concepts:

- Task
- Change Unit
- Decision Packet
- Write Authorization
- Evidence Manifest
- ProjectionKind
- MCP
- Core
- state.sqlite
- task_events
- prepare_write
- record_run
- close_task

## Translate naturally

Use the term that fits the sentence and reader context.

| English term | Korean guidance |
|---|---|
| context | Use `context` in identifier-like or AI-session phrasing. Use `맥락` in ordinary prose. |
| boundary | Keep `boundary` in code or identifier context. Use `경계` in prose. |
| authority | Use `권한` for operational authority. Use `기준 권한` when the sentence needs to stress the source of authority. |
| canonical | Keep `canonical` in identifier context. Use `기준` or `기준 기록` in Korean prose. |
| mutate | Prefer `change` or `modify` in English prose. Use `변경하다` in Korean. |
| surface | Choose the concrete meaning: `interface`, `view`, `entrypoint`, `display area`, or the Korean equivalent by context. User-facing Korean often wants `접점`, `화면`, or `표시 영역`. |
| evidence | Use `evidence` only when it is a product term. Use `근거` or `증거` in Korean prose. |
| acceptance / final acceptance | When this means the user's judgment that the result is acceptable, use `수락`, `결과 수락`, or `최종 수락` by context. |
| acceptance criteria | Use `수용 기준` for formal acceptance criteria. Use `완료 기준` when the sentence is about task completion rather than formal criteria. Do not use `수락 기준`. |
| residual-risk acceptance / accepted risk | In user-facing prose, prefer `남은 위험을 받아들이는 판단` or `남은 위험을 받아들이다`. In stricter reference contexts, `위험 수락` or exact Harness identifiers may be acceptable when the surrounding page uses that register. |
| Acceptance Gate / acceptance_gate | Keep exact identifiers such as `Acceptance Gate` or `acceptance_gate` where needed. Explain the meaning in Korean prose instead of inventing a new unstable term. |
| residual risk | Use `남은 위험` in user-facing prose. `잔여 리스크` is acceptable in more formal technical prose when the page already uses that register. |
| approval | Use `승인` in ordinary prose. Keep `Approval` when naming the Harness concept, status, or record. |
| write authority | Use `쓰기 권한` in prose. Keep `Write Authorization` exact when naming the Harness record. |
| gate | Use `게이트` for strict contracts. In user-facing flow, prefer concrete phrases such as `확인`, `닫기 확인`, or `막힘`. |

## Avoid examples

Avoid mixed-language phrases that preserve English technical words without helping the reader:

- `state를 mutate한다`
- `authority boundary를 preserve한다`
- `surface에 expose한다`
- `projection freshness를 report한다`
- `acceptance를 complete한다`
- `risk를 accept한다`
- `acceptance criteria를 수락 기준으로 쓴다`

## Prefer examples

Prefer natural phrases that preserve the technical meaning:

- `상태를 변경한다`
- `권한 경계를 유지한다`
- `화면에 보여준다`
- `projection이 최신인지 표시한다`
- `결과를 수락한다`
- `남은 위험을 받아들인다`
- `수용 기준을 확인한다`

## Before / After examples

Use patterns like these when polishing Korean prose:

| Before | After |
|---|---|
| `Core가 state 변경을 수행한다.` | `Core가 상태를 변경한다.` |
| `Agent는 authority boundary 유지가 필요하다.` | `Agent는 권한 경계를 유지해야 한다.` |
| `이 surface는 blocker 표시를 담당한다.` | `이 화면은 blocker를 보여준다.` |
| `Operations는 projection freshness를 report한다.` | `Operations는 projection이 최신인지 표시한다.` |
| `canonical source를 update한다.` | `기준 기록을 업데이트한다.` |
| `context를 잃지 않도록 한다.` | `맥락을 잃지 않도록 한다.` |
| `acceptance가 필요하다.` | `결과 수락이 필요하다.` |
| `risk를 accept한다.` | `남은 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 결과 수락처럼 쓴다.` | `남은 위험을 받아들이는 판단`처럼 결과 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |

## Korean heading policy

Korean headings should be natural Korean headings, not mechanical English-heading copies.

Keep official identifiers exact inside headings when the heading is about that identifier. Otherwise, choose the heading a Korean technical reader would expect.

Heading order and document meaning should remain aligned with the English document. Heading text does not need to match word for word.

## Bilingual review checklist

```text
[ ] Does the Korean page preserve the same meaning as the English page?
[ ] Does the paired file preserve the same active file path, reader purpose, semantic section coverage, owner links, and contractual detail?
[ ] Does the Korean prose read naturally to a Korean technical reader?
[ ] Are API names, schema names, enum values, DDL names, identifiers, paths, error codes, and validator IDs exact?
[ ] Are stable product terms preserved when they refer to Harness concepts?
[ ] Are source-of-truth phrases and owner links aligned with the owner Reference docs?
[ ] Are non-owner duplicate contracts summarized with owner links instead of translated as full contract copies?
[ ] Are mixed-language phrases replaced with natural Korean where possible?
[ ] In user-facing docs, do natural Korean phrases appear before Harness labels when both are needed?
[ ] Are headings idiomatic while preserving the same document structure and scope?
[ ] Were English and Korean link changes made in the same batch?
[ ] Does the review avoid treating translation drift as runtime state, evidence, QA, acceptance, close readiness, or implementation readiness?
```
