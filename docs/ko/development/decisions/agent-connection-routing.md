# Agent Connection과 호스트 라우팅

## 맥락

Volicord는 Codex, Claude Code, generic MCP 설정을 위한 직접 coding-agent host 지원이 필요하며, 동시에 등록된 `Product Repository`가 둘 이상인 사용 방식을 지원해야 합니다. MCP roots와 시작 디렉터리 맥락은 호스트 힌트입니다. Volicord 권한이 아니며 그 자체로 Project를 안전하게 선택할 수 없습니다.

## 결정

Volicord는 Agent Connection을 로컬 MCP 호스트 connection 하나의 지속 registry identity로 사용합니다. `volicord mcp --stdio` 프로세스는 `--connection <connection_id>`로 시작하며, 생성된 호스트 항목이 연결된 Project 하나에 안전하게 묶이면 `--project <project_id>`도 담을 수 있습니다. 여러 프로젝트 connection에서는 Project 접근을 프로세스 시작 때 고정하지 않고 도구 호출마다 선택하고 검증합니다.

이 설계는 아래 책임을 분리합니다.

- Registry는 Agent Connection identity, host kind, host scope, target metadata, connection mode, enabled state, verification state, 명시적 Connection Project membership을 저장합니다.
- `volicord mcp --stdio`는 시작 때 Agent Connection을 검증하고, 그 connection에서 current connection context를 파생하며, connection mode에 따라 MCP-visible tool을 노출하고, `volicord.list_projects`를 제공하며, 모호한 Project 선택을 거절합니다.
- 관리 CLI는 지원되는 호스트 connection setup을 생성, 검증, 갱신, 제거합니다.
- Host trust, project approval, OAuth, reload, restart, model behavior는 외부 호스트와 사용자에게 남습니다.

## 결과

- 사용자 범위 호스트 설정은 등록된 모든 Project를 허용하지 않고도 명시적으로 연결된 여러 Project를 다룰 수 있습니다.
- 여러 프로젝트를 다루는 호스트 MCP 명령이 같은 `connection_id`를 이미 가리키면 연결된 Project 추가나 제거에 명령 재작성이 필요하지 않습니다. 프로젝트에 묶인 생성 항목은 선택된 Project 바인딩이 바뀔 때 다시 생성될 수 있습니다.
- Project 선택 실패가 결정적입니다. 어댑터는 Project 선택 누락이나 모호함을 보고하고 연결된 Project 목록을 보도록 에이전트를 안내할 수 있습니다.
- 프로젝트에 묶인 시작은 도구 처리 전에 session-watch baseline을 만들 수 있습니다. 여러 프로젝트 시작은 명시적 프로젝트 선택 전까지 watcher coverage를 pending으로 보고합니다.
- Host setup 상태는 설정은 되었지만 호스트 동작을 기다리는 상태와 완전한 검증 완료를 구분할 수 있습니다.
- 생성되는 호스트 설정은 프로젝트 범위 항목에 `volicord mcp --stdio --connection <connection_id> --project <project_id>`를 선호하며 connection context나 actor provenance 환경 변수를 요구하지 않습니다. 여러 연결 Project를 의도적으로 다루는 흐름에는 connection-only 생성 항목이 남습니다.

## 비목표

- 이 결정은 공개 Volicord API 메서드를 추가하지 않습니다.
- CLI 명령을 공개 API 메서드로 만들지 않습니다.
- MCP roots, 현재 작업 디렉터리, host label, 복사된 `connection_id` 값을 Volicord 권한으로 만들지 않습니다.
- 사용자 범위 connection에 등록된 모든 Project를 부여하지 않습니다.
- 저장소 안내, MCP server instructions, host rule file이 모델 동작을 강제한다고 정의하지 않습니다.
- Volicord 런타임 상태, SQLite 데이터베이스, 생성 로그, QA 결과, 수락 기록, 닫기 준비 상태, 잔여 위험 기록을 `Product Repository`에 둘 수 있게 하지 않습니다.

## 관련 구현 영역

- [`crates/volicord-mcp`](../../../../crates/volicord-mcp): connection-bound startup, MCP initialization, tool discovery, Project selection, Core 호출 전 adapter validation.
- [`crates/volicord-cli`](../../../../crates/volicord-cli): 공개 `volicord mcp` 프로세스 진입점, 호스트 설정 명령 생성, 관리 connect/status/verify/uninstall 흐름.
- [`crates/volicord-store`](../../../../crates/volicord-store): registry schema, migration, Agent Connection records, Connection Project membership, Runtime Home access.
- 저장 값 집합과 기계 판독 가능한 관리 출력에 쓰이는 공유 타입.

## 관련 테스트와 참조 담당 문서

이 설계의 테스트는 startup validation, Project selection, membership revocation, host setup status, project scope의 repository-write approval, managed marker replacement, unsupported startup form rejection을 다뤄야 합니다.

참조 담당 문서:

- [Agent Connection Reference](../../reference/agent-connection.md)
- [MCP 전송](../../reference/mcp-transport.md)
- [관리 CLI](../../reference/admin-cli.md)
- [런타임 경계](../../reference/runtime-boundaries.md)
- [저장소 기록](../../reference/storage-records.md)
- [저장소 DDL](../../reference/storage-ddl.md)
- [저장소 버전 관리](../../reference/storage-versioning.md)
- [보안](../../reference/security.md)
