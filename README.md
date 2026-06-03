# Harness Project / 하네스 프로젝트

Harness is a local work-authority server for AI-assisted product work. Its job is to keep fragile conversation context from becoming the source of truth. It preserves the local basis for scope, user-owned judgment, evidence, verification expectations, work acceptance, close readiness, and residual risk. When an agent should not decide, Harness routes that decision back to the user.

Harness는 AI 지원 제품 작업을 위한 로컬 작업 권한 서버입니다. 대화의 깨지기 쉬운 맥락이 기준 기록처럼 굳어지지 않게 하는 것이 하네스의 역할입니다. 하네스는 범위, 사용자 소유 판단, 근거, 확인과 검증 기대, 작업 수락, 닫기 가능 여부, 잔여 위험의 로컬 근거를 보존합니다. 에이전트가 판단하면 안 되는 일은 사용자에게 다시 돌려보냅니다.

## Product Thesis / 제품 명제

| Harness is not / 하네스가 아닌 것 | Harness does / 하네스가 하는 일 |
|---|---|
| A prompt pack or chat script. / 프롬프트 묶음이나 대화 스크립트. | Keeps work authority outside prompts and conversation. / 작업 권한을 프롬프트와 대화 밖에 둡니다. |
| MCP itself or an API wrapper. / MCP 자체나 API 래퍼. | May use MCP/API surfaces, but the product thesis is the local work-authority record. / MCP/API 접점을 사용할 수는 있지만, 제품 명제는 로컬 작업 권한 기록입니다. |
| A workflow engine, report generator, or dashboard. / 워크플로 엔진, 보고서 생성기, 대시보드. | Records the basis for work and can derive readable views from that record. / 작업의 근거를 기록하고 그 기록에서 읽기용 보기를 만들 수 있습니다. |
| A hosted agent platform. / 호스팅 에이전트 플랫폼. | Is designed around a local Harness Server / Installation. / 로컬 하네스 서버/설치를 중심으로 설계됩니다. |
| A sandbox or OS permission system. / 샌드박스나 OS 권한 시스템. | Preserves authority boundaries; it does not claim OS-level isolation or arbitrary-tool permission control. / 권한 경계를 보존하지만 OS 수준 격리나 임의 도구 권한 제어를 주장하지 않습니다. |

The short version: chat stays conversation, MCP/API is only a possible mechanism, reports are readable views, and Core-owned local state plus artifact references are the operating basis.

짧게 말하면 대화는 대화로 남고, MCP/API는 가능한 구현 메커니즘일 뿐이며, 보고서는 사람이 읽는 보기입니다. 운영 기준은 Core가 소유한 로컬 상태와 아티팩트 참조입니다.

## Repository Identity / 저장소 정체성

Current state: this repository is documentation-only and in post-redesign review. It contains source documentation for the future local Harness Server, not the server itself.

Future state: after documentation acceptance and a separate implementation-planning readiness decision, this repository is intended to become the Harness Server source repository.

It is not the user's Product Repository. It is not a Harness Runtime Home. No Harness Server, runtime, generated projection system, conformance runner, runtime data, product implementation code, or generated operational artifact exists here yet.

현재 상태: 이 저장소는 문서 전용이며 재설계 이후 검토 상태입니다. 향후 로컬 하네스 서버를 위한 원천 문서를 담고 있을 뿐, 서버 자체를 담고 있지 않습니다.

향후 상태: 문서 수락과 별도의 구현 계획 준비 결정이 모두 이루어진 뒤, 이 저장소는 하네스 서버 소스 저장소가 될 예정입니다.

이 저장소는 사용자의 제품 저장소가 아닙니다. 하네스 런타임 홈도 아닙니다. 아직 이곳에는 하네스 서버, 런타임, 생성된 읽기용 요약 시스템, conformance runner, 런타임 데이터, 제품 구현 코드, 생성된 운영 아티팩트가 없습니다.

Documentation files are source material for understanding and eventually implementing Harness. They are not Harness runtime objects, state records, generated projections, evidence, QA records, acceptance records, residual-risk records, or close records. Documentation edits do not start server/runtime implementation and do not authorize implementation planning by themselves.

문서 파일은 하네스를 이해하고 나중에 구현하기 위한 원천 자료입니다. 하네스 런타임 객체, 상태 기록, 생성된 읽기용 요약, 근거, QA 기록, 작업 수락 기록, 잔여 위험 기록, 닫기 기록이 아닙니다. 문서 편집은 서버/런타임 구현을 시작하지 않으며 그 자체로 구현 계획을 허가하지 않습니다.

## Current Status / 현재 상태

| Check / 확인 | Current status / 현재 상태 |
|---|---|
| Documentation review / 문서 검토 | Post-redesign review; documentation acceptance candidate only. Maintainers have not accepted the docs yet. / 재설계 이후 검토 상태이며 문서 수락 후보입니다. 유지보수자가 아직 문서를 수락하지 않았습니다. |
| Implementation planning / 구현 계획 | Not accepted. Maintainers must confirm implementation-readiness criteria before first runtime-batch planning. / 아직 수락되지 않았습니다. 첫 런타임 배치 계획 전에 유지보수자가 구현 준비 조건을 확인해야 합니다. |
| Runtime implementation / 런타임 구현 | Not started. No runtime artifacts or conformance results exist here. / 시작하지 않았습니다. 런타임 아티팩트나 conformance 결과가 없습니다. |
| Server-coding decisions / 서버 코딩 결정 | Open decision-ledger items are recorded in the MVP Plan. No server/runtime implementation decision has been formally accepted for coding. / 서버 코딩 전 열린 결정은 MVP 계획에 기록되어 있습니다. 서버/런타임 구현 결정은 코드 작성용으로 공식 수락되지 않았습니다. |

Detailed handoff status lives in [Implementation Overview](docs/en/build/implementation-overview.md#maintainer-handoff-summary) / [구현 개요](docs/ko/build/implementation-overview.md#문서-인계-요약). Server-coding decisions live in [MVP Plan](docs/en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) / [MVP 계획](docs/ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정).

상세 인계 상태는 [Implementation Overview](docs/en/build/implementation-overview.md#maintainer-handoff-summary) / [구현 개요](docs/ko/build/implementation-overview.md#문서-인계-요약)가 담당합니다. 서버 코딩 전 결정은 [MVP Plan](docs/en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding) / [MVP 계획](docs/ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정)에 둡니다.

## Start Here / 시작하기

Start at [docs/README.md](docs/README.md) for compact bilingual routing, the minimal first-read path, and role-based reader paths.

이중 언어 경로, 최소 첫 읽기 경로, 독자별 경로는 [docs/README.md](docs/README.md)에서 시작하세요.

| Need / 필요 | Start / 시작 |
|---|---|
| First-time reader / 처음 읽는 독자 | [Overview](docs/en/learn/overview.md) / [개요](docs/ko/learn/overview.md) |
| User working with an agent / 에이전트와 작업하는 사용자 | [User Guide](docs/en/use/user-guide.md) / [사용자 가이드](docs/ko/use/user-guide.md) |
| Implementer reviewing future server work / 향후 서버 구현을 검토하는 구현자 | [Implementation Overview](docs/en/build/implementation-overview.md) / [구현 개요](docs/ko/build/implementation-overview.md) |
| Documentation maintainer / 문서 유지보수자 | [Authoring Guide](docs/en/maintain/authoring-guide.md) / [문서 작성 가이드](docs/ko/maintain/authoring-guide.md) |
| Exact contracts / 정확한 계약 | [English Reference Index](docs/en/reference/README.md) / [한국어 Reference 색인](docs/ko/reference/README.md) |

## Documentation Redesign / 문서 재설계

The redesign may change terminology, MVP staging, schema structure, projection structure, security wording, and document organization. Preserve the core value, not old wording: Harness is a local work-authority server, not a prompt pack, MCP itself, a workflow engine, a report generator, a dashboard, a hosted agent platform, or a sandbox/OS permission system.

이번 재설계에서는 용어, MVP 단계, 스키마 구조, 읽기용 요약 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 오래된 문구가 아니라 핵심 가치를 보존합니다. 하네스는 로컬 작업 권한 서버이지 프롬프트 묶음, MCP 자체, 워크플로 엔진, 보고서 생성기, 대시보드, 호스팅 에이전트 플랫폼, 샌드박스나 OS 권한 시스템이 아닙니다.

Strict contracts live in the Reference owner docs linked from the Reference indexes. Learn, Use, and Build pages should explain and route rather than duplicate those contracts.

엄격한 계약은 Reference 색인에서 연결하는 owner 문서가 담당합니다. Learn, Use, Build 문서는 그 계약을 중복하기보다 필요한 설명과 경로를 제공합니다.
