# Harness Documentation / 하네스 문서

This directory contains the Harness documentation in parallel English and Korean trees.

Harness is an agency-preserving local authority kernel for AI-assisted product work. It keeps a local operating record of the work facts that need to remain followable after chat moves on.

Harness is not a chat script, prompt bundle, test harness, evaluation harness, dashboard, or replacement for the user's product repository or product and technical judgment. This repository is still in documentation review; server and runtime implementation have not started.

For the full identity, authority model, integration-surface boundaries, and non-goals, start with [English Overview](en/learn/overview.md), [English Purpose and Principles](en/learn/purpose-and-principles.md), [Korean Overview](ko/learn/overview.md), or [Korean Purpose and Principles](ko/learn/purpose-and-principles.md).

이 디렉터리는 Harness 문서를 영어와 한국어 트리로 나누어 담고 있습니다.

Harness는 AI 지원 제품 작업을 위한, 사용자 판단권을 보존하는 로컬 권한 커널입니다. 대화가 지나간 뒤에도 따라갈 수 있어야 하는 작업 사실을 로컬 운영 기록으로 유지합니다.

Harness는 대화 스크립트, prompt 묶음, test harness, evaluation harness, dashboard, 사용자의 제품 저장소나 제품 판단과 기술 판단을 대체하는 도구가 아닙니다. 이 저장소는 아직 문서 검토 단계이며, server와 runtime 구현은 시작하지 않았습니다.

정체성, 권한 모델, 통합 접점 경계, 비목표의 전체 설명은 [영어 개요](en/learn/overview.md), [영어 목적과 원칙](en/learn/purpose-and-principles.md), [한국어 개요](ko/learn/overview.md), [한국어 목적과 원칙](ko/learn/purpose-and-principles.md)에서 시작합니다.

## Choose a Language / 언어 선택

| Language | Entry point |
|---|---|
| English | [en/README.md](en/README.md) |
| Korean | [ko/README.md](ko/README.md) |

## Quick Routes / 빠른 경로

| Reader need / 독자 필요 | English start / 영어 시작 | Korean start / 한국어 시작 |
|---|---|---|
| First time with Harness / Harness를 처음 읽을 때 | [Overview](en/learn/overview.md) | [개요](ko/learn/overview.md) |
| A concrete one-task story / 한 작업의 구체적 흐름 | [Harness in One Task](en/learn/harness-in-one-task.md) | [하나의 작업으로 보는 Harness](ko/learn/harness-in-one-task.md) |
| Using Harness during assisted work / AI 지원 작업 중 사용 | [User Guide](en/use/user-guide.md) | [사용자 가이드](ko/use/user-guide.md) |
| Implementing after documentation acceptance / 문서 승인 뒤 구현 준비 | [Implementation Overview](en/build/implementation-overview.md) | [구현 개요](ko/build/implementation-overview.md) |
| Looking up strict contracts / 엄격한 계약 확인 | [English Reference list](en/README.md#reference) | [한국어 Reference 목록](ko/README.md#reference) |
| Maintaining the documentation set / 문서 세트 유지보수 | [Authoring Guide](en/maintain/authoring-guide.md) | [문서 작성 가이드](ko/maintain/authoring-guide.md) |

## Reader Paths and Ownership / 독자 경로와 소유권

The redesigned documentation is organized around five reader paths. Use the path for the reader's immediate need, then link to the owner when exact detail matters.

| Path | Purpose | Ownership note |
|---|---|---|
| Learn | Understand what kind of Harness this is, why it exists, and how its concepts fit together. | Owns mental model and examples, not strict schemas or gates. |
| Use | Follow Harness as a user during assisted development work. | Owns user-facing flow and status interpretation. |
| Build | Prepare to implement Harness after the redesigned docs are accepted. | Owns implementation order and runnable slices, not exact DDL or API bodies. |
| Reference | Look up stable contracts, schemas, operations/conformance rules, glossary definitions, and design-quality policies. | Owns strict contracts. |
| Maintain | Keep the documentation and future Harness system coherent over time. | Owns documentation governance, not runtime behavior. |

재설계된 문서는 다섯 가지 독자 경로를 중심으로 정리합니다. 독자의 지금 필요에 맞는 경로에서 시작하고, 정확한 세부사항이 필요하면 owner 문서로 연결합니다.

| 경로 | 목적 | 소유권 메모 |
|---|---|---|
| Learn | Harness가 어떤 종류의 시스템인지, 왜 필요한지, 핵심 개념이 어떻게 이어지는지 이해합니다. | 이해 모델과 예시를 담당합니다. 엄격한 schema나 gate는 담당하지 않습니다. |
| Use | AI 지원 개발 과정에서 사용자가 Harness를 어떻게 따라가면 되는지 봅니다. | 사용자에게 보이는 흐름과 상태 해석을 담당합니다. |
| Build | 재설계된 문서가 승인된 뒤 구현을 준비합니다. | 구현 순서와 실행 가능한 조각을 담당합니다. 정확한 DDL이나 API 본문은 담당하지 않습니다. |
| Reference | 안정적인 계약, schema, 운영/Conformance 규칙, 용어 정의, 설계 품질 정책을 찾아봅니다. | 엄격한 계약을 담당합니다. |
| Maintain | 문서와 이후 Harness 시스템의 일관성을 유지합니다. | 문서 거버넌스를 담당합니다. 런타임 동작은 담당하지 않습니다. |

## Contract Ownership / 계약 소유권

Reference docs own exact contracts. Learn, Use, and Build docs may summarize a contract in ordinary reader language, but they should link to the owning Reference page instead of copying schemas, DDL, transition tables, fixture bodies, or other normative contract blocks.

엄격한 계약은 Reference 문서가 담당합니다. Learn, Use, Build 문서는 독자에게 필요한 만큼 쉬운 말로 요약할 수 있지만, schema, DDL, transition table, fixture body, 기타 규범적 계약 블록을 복사하지 않고 owner Reference 문서로 연결합니다.

The implementation handoff checkpoint lives in [English Implementation Overview](en/build/implementation-overview.md#implementation-handoff-checkpoint) and [Korean Implementation Overview](ko/build/implementation-overview.md#구현-handoff-checkpoint). It separates documentation-maintenance work from first runtime-batch planning; it does not replace Reference contracts.

Open follow-up entries in that checkpoint are maintainer-updated documentation-maintenance notes only. They do not create runtime authority or Reference contracts.

구현 handoff checkpoint는 [영어 구현 개요](en/build/implementation-overview.md#implementation-handoff-checkpoint)와 [한국어 구현 개요](ko/build/implementation-overview.md#구현-handoff-checkpoint)에 있습니다. 이 checkpoint는 문서 유지보수와 첫 runtime batch 계획을 구분하며, Reference 계약을 대체하지 않습니다.

그 checkpoint의 열린 follow-up 항목은 maintainer가 갱신하는 문서 유지보수 메모일 뿐입니다. Runtime authority나 Reference 계약을 만들지 않습니다.

## Roadmap / 로드맵

Post-MVP items live in each language tree's roadmap: [English](en/roadmap.md), [Korean](ko/roadmap.md). The roadmap is not part of the MVP implementation contract unless a future owner explicitly promotes an item with scope, fixtures, and fallback behavior.

Post-MVP 항목은 각 언어 트리의 Roadmap에 둡니다. [English](en/roadmap.md), [Korean](ko/roadmap.md)를 봅니다. 향후 담당자가 범위, fixture, fallback 동작을 정해 항목을 명시적으로 승격하기 전까지 Roadmap 항목은 MVP 구현 계약에 포함되지 않습니다.

## Parity / 문서 Parity

English and Korean docs keep the same file map and semantic content. Semantic parity must be maintained across `docs/en` and `docs/ko`.

영어 문서와 한국어 문서는 같은 파일 지도를 유지하고, 의미상 같은 내용을 담아야 합니다. 단, 한국어 문서는 자연스러운 한국어 제목과 문장 흐름을 사용할 수 있습니다.
