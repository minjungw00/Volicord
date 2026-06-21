# Agent Integration Profile과 호스트 라우팅

## 맥락

Harness는 Codex, Claude Code 같은 호스트와 직접 코딩 에이전트 통합을
제공하면서도 등록된 Product Repository가 둘 이상인 사용 방식을 지원해야
합니다. 예전의 고정 프로젝트 MCP setup 모델은 환경 변수로 서버 프로세스를
프로젝트 하나에 묶었습니다. 이 모델은 사용자 범위 호스트 설정, 명시적
다중 프로젝트 허용 목록, 호스트별 신뢰와 승인 흐름에 맞지 않습니다.

MCP 클라이언트는 roots나 시작 디렉터리 맥락을 제공할 수 있습니다. 하지만
그 값은 호스트 힌트일 뿐이며 Harness 권한이 아니므로 그 자체로 프로젝트를
선택할 수 없습니다.

## 결정

Harness는 Agent Integration Profile을 코딩 에이전트 통합 하나의 지속되는
레지스트리 식별 정보로 사용합니다. MCP 서버 프로세스는 `integration_id`로
시작되며, 프로젝트 접근은 프로세스 시작 때 고정하지 않고 도구 호출마다
선택하고 검증합니다.

이 설계는 아래 책임을 분리합니다.

- 레지스트리는 통합 식별 정보, 묶인 코딩 에이전트 접점 식별 정보, 명시적
  프로젝트 멤버십, 선택적 기본 프로젝트, 관리되는 Host Installation
  인벤토리를 저장합니다.
- `harness-mcp`는 시작 때 통합을 검증하고, 프로필에서 묶인 접점 맥락을
  파생하며, 공개 Harness 도구와 `harness.list_projects` 도우미를 노출하고,
  모호한 프로젝트 선택을 거절합니다.
- 관리 CLI는 지원 호스트의 통합 setup을 생성, 검증, 갱신, 제거합니다.
  명시적 승인이 있으면 프로젝트 범위 설정과 저장소 안내 파일도 관리합니다.
- 호스트 신뢰, 프로젝트 승인, OAuth, 다시 로드, 재시작, 모델 동작은 외부
  호스트와 사용자에게 남습니다.

## 결과

- 사용자 범위 호스트 설정은 등록된 모든 프로젝트를 허용하지 않고도
  명시적으로 추가된 여러 프로젝트를 다룰 수 있습니다.
- 호스트 MCP 명령이 같은 `integration_id`를 이미 가리키고 있으면 프로젝트
  멤버십 추가나 철회에 호스트 설정 재작성이 필요하지 않습니다.
- 프로젝트 선택 실패가 결정적이 됩니다. 어댑터는 프로젝트 선택 누락이나
  모호함을 보고하고, 허용된 프로젝트 목록을 보도록 에이전트를 안내할 수
  있습니다.
- 호스트 setup 상태는 설정은 되었지만 호스트 동작을 기다리는 상태와 완전한
  검증 완료를 구분할 수 있습니다.
- 기존 고정 프로젝트 MCP setup은 호환 동작이며, 새 문서나 생성되는 호스트
  설정의 기준 setup 모델이 아닙니다.

## 비목표

- 이 결정은 공개 Harness API 메서드를 추가하지 않습니다.
- CLI 명령을 공개 API 메서드로 만들지 않습니다.
- MCP roots, 현재 작업 디렉터리, 호스트 라벨을 Harness 권한으로 만들지
  않습니다.
- 사용자 범위 호스트 설치에 등록된 모든 프로젝트를 부여하지 않습니다.
- 저장소 안내, MCP 서버 instructions, 호스트 규칙 파일이 모델 동작을
  강제한다고 정의하지 않습니다.
- Harness 런타임 상태, SQLite 데이터베이스, 생성 로그, QA 결과, 수락 기록,
  닫기 준비 상태, 잔여 위험 기록을 Product Repository에 둘 수 있게 하지
  않습니다.

## 관련 구현 영역

이 결정에 맞는 구현 작업은 아래 영역에 속합니다.

- [`crates/harness-mcp`](../../../../crates/harness-mcp): 통합에 묶인 시작,
  MCP 초기화, 도구 발견, 프로젝트 선택, Core 호출 전 어댑터 검증.
- [`crates/harness-cli`](../../../../crates/harness-cli): 관리 install/status/
  verify/uninstall 흐름과 저장소 안내 관리.
- [`crates/harness-store`](../../../../crates/harness-store): 레지스트리
  스키마, 마이그레이션, 통합 멤버십, Host Installation 인벤토리, Runtime
  Home 접근.
- 저장 값 집합과 기계 판독 가능한 관리 출력에 쓰이는 공유 타입.

## 관련 테스트와 참조 담당 문서

이 설계의 테스트는 시작 검증, 프로젝트 선택, 멤버십 철회, 호스트 setup
상태, 저장소 쓰기 승인, 관리 marker 교체, 레거시 고정 프로젝트 setup
호환 동작을 다뤄야 합니다.

참조 담당 문서:

- [에이전트 통합](../../reference/agent-integration.md)
- [MCP 전송](../../reference/mcp-transport.md)
- [관리 CLI](../../reference/admin-cli.md)
- [런타임 경계](../../reference/runtime-boundaries.md)
- [저장소 기록](../../reference/storage-records.md)
- [저장소 DDL](../../reference/storage-ddl.md)
- [저장소 버전 관리](../../reference/storage-versioning.md)
- [보안](../../reference/security.md)
