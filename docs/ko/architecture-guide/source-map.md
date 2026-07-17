# 소스 맵

이 맵은 유지관리자를 현재 구현 소유자로 안내합니다. 제품 계약이 아니므로 정확한 동작은
집중된 Reference 문서를 사용합니다.

## 공유 타입

| 경로 | 책임 |
|---|---|
| `crates/volicord-types/src/schema.rs` | 공유 요청, 응답, 저장 레코드 형태. |
| `crates/volicord-types/src/values.rs` | 폐쇄 제품 값 집합. |
| `crates/volicord-types/src/ids.rs` | 불투명 식별자. |
| `crates/volicord-types/src/canonical.rs` | 정규 직렬화와 해시. |
| `crates/volicord-types/src/tool_names.rs` | 공개 MCP 도구 이름 레지스트리. |

## Store

| 경로 | 책임 |
|---|---|
| `crates/volicord-store/src/schema/registry.sql` | Runtime Home registry DDL 정본 소스. |
| `crates/volicord-store/src/schema/project.sql` | project Store DDL 정본 소스. |
| `crates/volicord-store/src/bootstrap.rs` | Runtime Home과 Store bootstrap. |
| `crates/volicord-store/src/agent_connections.rs` | Agent Connection 레코드와 project allowlist. |
| `crates/volicord-store/src/workflow_records.rs` | workflow 레코드 읽기와 쓰기. |
| `crates/volicord-store/src/core_pipeline/` | Core open, 검증, replay, commit, mutation 적용. |
| `crates/volicord-store/src/guards.rs` | Guard 관찰, 예상 쓰기, suppression 입력. |
| `crates/volicord-store/src/evidence_capture.rs` | Evidence-capture intent와 producer 레코드. |
| `crates/volicord-store/src/artifacts.rs` | 아티팩트 staging과 영속 본문 검증. |
| `crates/volicord-store/src/error.rs` | Store 실패 분류. |

## Core

| 경로 | 책임 |
|---|---|
| `crates/volicord-core/src/pipeline.rs` | 공통 사전 점검, replay, plan 선택, 응답, commit 조율. |
| `crates/volicord-core/src/methods/` | 메서드별 구조 검증과 계획. |
| `crates/volicord-core/src/policy/` | 재사용 접근, workflow, evidence, continuity, write-ticket, close-readiness 정책. |
| `crates/volicord-core/src/authority_status.rs` | typed status와 authority receipt 대응. |

## CLI와 Codex 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-cli/src/main.rs` | 프로세스 진입과 관리 명령 디스패치. |
| `crates/volicord-cli/src/connection_command/` | connection add, list, status, verify, mode, remove 조율. |
| `crates/volicord-cli/src/host_integration/codex/` | Codex 구성, 실행 파일 식별, trust fact, 검증. |
| `crates/volicord-cli/src/guard_command/` | Guard 이벤트 디코딩과 bounded 관찰. |
| `crates/volicord-cli/src/user_command.rs` | CLI 받은 편지함과 local-user resolution. |
| `crates/volicord-cli/src/doctor_command.rs` | 진단 사실 수집과 표시. |

## MCP 어댑터

| 경로 | 책임 |
|---|---|
| `crates/volicord-mcp/src/stdio.rs` | stdio 생명주기, 프레이밍, 초기화, 프로세스 사전 점검. |
| `crates/volicord-mcp/src/adapter.rs` | 공개 인수 디코딩, 서버 소유 맥락, Core 디스패치, wrapping. |
| `crates/volicord-mcp/src/tool_registry.rs` | 압축된 공개 도구 descriptor. |
| `crates/volicord-mcp/src/schema_validation.rs` | 공개 schema 검증. |
| `crates/volicord-mcp/src/repository_discovery.rs` | 결속된 Product Repository 탐색. |

## 테스트

| 경로 | 책임 |
|---|---|
| `crates/*/tests/`와 module-local `tests` | crate 경계와 unit test. |
| `tests/conformance/` | 교차 메서드 conformance scenario. |
| `tests/release-validation/` | 정확한 최종 Codex 아티팩트 검증과 체크인된 네 플랫폼 manifest. |
| `crates/volicord-test-support/` | 일회용 Runtime Home, repository, Store, 요청 도우미. |

지속되는 책임이 이동하면 이 맵을 갱신합니다. 삭제된 경로, 생성 경로, 개인 scratch 경로를
나열하지 않습니다.
