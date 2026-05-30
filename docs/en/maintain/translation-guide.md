# Translation Guide

## What this document helps you do

Use this guide when editing English and Korean Harness documentation together.

This is maintenance documentation for bilingual documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, or product state changes before the documentation set is accepted for implementation planning, and it does not define conformance pass/fail, evidence, QA, acceptance, close readiness, or implementation readiness. The first product MVP target is v0.1 Kernel MVP, and Kernel Smoke is the narrow conformance profile for exercising it. v0.2 Evidence & Projection Pack, v0.3 Agency Pack, and v0.4 Operations Pack move toward the Agency-Hardened MVP reference conformance target. v1+ Expansion remains roadmap scope unless owner docs promote and prove an item.

## Read this when

- You are changing meaning in an English or Korean doc.
- You are reviewing English/Korean semantic parity.
- You need to decide whether Korean wording should preserve an English identifier or use natural Korean prose.

## Before you read

Read [Authoring Guide](authoring-guide.md) for owner boundaries, docs-maintenance checks, and the rule that strict contracts stay in Reference docs.

## Main idea

The goal is semantic parity, not sentence-by-sentence translation. Korean should read like natural technical Korean while preserving official identifiers, exact contracts, code-like names, and stable product terms.

In user-facing Korean, prefer the natural phrase first and add the exact Harness label only when the label helps precision. For example, use `범위(Change Unit)`, `결정 패킷(Decision Packet)`, `쓰기 허가 기록(Write Authorization)`, `잔여 위험(Residual Risk)`, `수동 QA(Manual QA)`, `분리 검증(detached verification)`, or `결과 수락(final acceptance)` when both the reader-friendly phrase and the Harness label matter.

## Keep exact

Keep these unchanged across English and Korean docs:

- API names
- schema names
- enum values
- DDL names
- code identifiers
- file names and path names
- error codes and validator IDs

Keep these exact when they refer to literal identifiers, schema/API values, file/template names, heading anchors, or code-like references. In ordinary Korean prose, prefer the stable Korean terms in [Korean Canonical Terms](#korean-canonical-terms).

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

Do not translate markers such as `HARNESS:BEGIN`, schema names such as `ArtifactRef`, `ProjectionKind`, `decision_kind=approval`, `approval_gate`, `ResidualRiskSummary.status=none`, validator IDs, error codes, file paths, API/tool/schema names, or other exact strings.

Keep exact stage labels exact: `v0.1 Kernel MVP`, `Kernel Smoke`, `v0.2 Evidence & Projection Pack`, `v0.3 Agency Pack`, `v0.4 Operations Pack`, `Agency-Hardened MVP`, `Agency-Hardened MVP reference conformance target`, and `v1+ Expansion`. For the three-space model in Korean prose, use `제품 저장소`, `하네스 서버 소스 저장소` when referring to this repository's future source role, and `하네스 런타임 홈`; add the English labels in parentheses only when they help disambiguate the architecture term.

Reference headings that serve as lookup anchors should remain stable unless a dedicated link/anchor migration updates all links. User-facing prose should prefer natural Korean. A Korean alias line may provide the natural term under a stable reference heading.

## Korean Canonical Terms

Use these as the preferred terms in Korean prose. Keep exact English strings where the sentence refers to an identifier or contract value, and add the English label in parentheses when it helps the reader.

| English term | Korean canonical term | Usage note |
|---|---|---|
| Harness | 하네스 | Use for the product name in ordinary Korean prose. Keep literal strings such as `HARNESS:BEGIN`. |
| Product Repository | 제품 저장소 | Use for the user's product workspace. Add the English label only when disambiguating the three-space model. |
| Harness Server source repository | 하네스 서버 소스 저장소 | Use for this repository's intended future source-code role after documentation acceptance. |
| Harness Runtime Home | 하네스 런타임 홈 | Use for the per-user/per-installation operational data home. Add the English label only when helpful. |
| durable local state | 지속 로컬 상태 | First use may include `지속 로컬 상태(durable local state)`. |
| artifact ref | 아티팩트 참조 | In evidence contexts, `증거 아티팩트 참조` is also acceptable. Keep the `ArtifactRef` schema name exact. |
| projection | 읽기용 투영 문서 | Keep `projection` for `projection freshness`, `ProjectionKind`, API fields, and template kinds when useful. `읽기용 보기` is natural in general explanation. |
| kernel | 커널 | Use `커널` outside exact headings and owner links. |
| gate | 관문 | Prefer `관문` in Learn/Use docs. Reference docs may retain `gate` when referring to kernel fields or values. |
| Decision Packet | 결정 패킷 | Keep `Decision Packet` in literal record/API/schema/anchor contexts when needed. |
| Write Authorization | 쓰기 허가 기록 | Use in prose for the record or result of `prepare_write`. Keep exact API/tool names and fields. |
| Residual Risk | 잔여 위험 | Prefer this over `남은 위험` in user-facing prose too. Plain explanatory wording such as `남은 불확실성` is acceptable when not naming the product concept. |
| Manual QA | 수동 QA | Keep `Manual QA` in exact template/schema/API contexts. |
| detached verification | 분리 검증 | May retain `detached verification` in assurance explanations. |
| cooperative | 협력형 | Retain the English label in guarantee-level tables. |
| detective | 탐지형 | Retain the English label in guarantee-level tables. |
| preventive | 예방형 | Retain the English label in guarantee-level tables. |
| isolated | 격리형 | Retain the English label in guarantee-level tables. |

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
| residual-risk acceptance / accepted risk | In user-facing prose, prefer `잔여 위험을 받아들이는 판단` or `잔여 위험을 받아들이다`. In stricter reference contexts, `위험 수락` or exact Harness identifiers may be acceptable when the surrounding page uses that register. |
| Acceptance Gate / acceptance_gate | Keep exact identifiers such as `Acceptance Gate` or `acceptance_gate` where needed. Explain the meaning in Korean prose instead of inventing a new unstable term. |
| residual risk | Use `잔여 위험` as the canonical term. Plain explanatory wording may describe the uncertainty, but keep terminology consistent. |
| approval / Approval | Use `Approval` when naming the canonical Harness sensitive-action permission concept, status, gate, or record. Use `승인` only for ordinary non-concept prose in Korean. |
| write authority | Use `쓰기 권한` in ordinary prose. Use `쓰기 허가 기록(Write Authorization)` when naming the Harness record produced by `prepare_write`. |
| gate | In user-facing flow, prefer `관문`, `확인`, `닫기 확인`, or `막힘` by context. Reference docs may retain `gate` for kernel fields or strict contracts. |

Capitalization rule: `Approval` is the canonical Harness permission concept for sensitive actions. Lowercase `approval` may remain only in stable identifiers, enum values, schema names, intentionally fixed phrases, or quoted legacy/user wording, such as `approval_gate`, `decision_kind=approval`, `approval_request_candidate`, `approval_scope`, `approval-shaped`, and approval drift.

## Avoid examples

Avoid mixed-language phrases that preserve English technical words without helping the reader:

- `state를 mutate한다`
- `authority boundary를 preserve한다`
- `surface에 expose한다`
- `projection freshness를 report한다`
- `acceptance를 complete한다`
- `risk를 accept한다`
- `acceptance criteria를 수락 기준으로 쓴다`
- `Harness 상태를 local state와 artifact ref에 둔다`
- `detached verification을 독립 검증이라고만 쓴다`

## Prefer examples

Prefer natural phrases that preserve the technical meaning:

- `상태를 변경한다`
- `권한 경계를 유지한다`
- `화면에 보여준다`
- `projection이 최신인지 표시한다`
- `결과를 수락한다`
- `잔여 위험을 받아들인다`
- `수용 기준을 확인한다`
- `하네스 상태를 지속 로컬 상태와 아티팩트 참조에 둔다`
- `분리 검증(detached verification)을 기록한다`

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
| `risk를 accept한다.` | `잔여 위험을 받아들인다.` |
| `acceptance criteria를 수락 기준으로 쓴다.` | `수용 기준` 또는 문맥에 따라 `완료 기준`을 쓴다. |
| `residual-risk acceptance를 결과 수락처럼 쓴다.` | `잔여 위험을 받아들이는 판단`처럼 결과 수락과 구분한다. |
| `acceptance_gate를 수락 게이트로 새로 번역한다.` | `acceptance_gate`를 유지하고 한국어 문장으로 의미를 설명한다. |
| `surface capability를 확인한다.` | `접점이 실제로 할 수 있는 일을 확인한다.` |
| `Harness 상태는 local state와 artifact ref에 있다.` | `하네스 상태는 지속 로컬 상태와 아티팩트 참조에 있다.` |
| `detached verification을 독립 검증으로 표시한다.` | `분리 검증(detached verification)으로 표시한다.` |

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
[ ] Are Korean canonical terms consistent in ordinary prose, while exact identifiers remain exact?
[ ] Are source-of-truth phrases and owner links aligned with the owner Reference docs?
[ ] Are non-owner duplicate contracts summarized with owner links instead of translated as full contract copies?
[ ] Are mixed-language phrases replaced with natural Korean where possible?
[ ] In user-facing docs, do natural Korean phrases appear before Harness labels when both are needed?
[ ] Are headings idiomatic while preserving the same document structure and scope?
[ ] Were English and Korean link changes made in the same batch?
[ ] Does the review avoid treating translation drift as runtime state, evidence, QA, acceptance, close readiness, or implementation readiness?
```
