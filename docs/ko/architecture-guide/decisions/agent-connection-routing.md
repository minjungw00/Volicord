# Agent Connection routing

## 맥락

첫 release는 관리 stdio에서 `host_kind=codex`와
`integration_profile=record`만 받습니다. 연결은 한 사용자
또는 선택한 한 project에 설치할 수 있지만, Core 요청은 여전히 명시적으로 허용된 Product
Repository를 해석해야 합니다.

## 결정

정확한 `host_kind=codex`, `integration_profile=record`,
`connection_scope=personal|shared`를 가진 Agent Connection 하나를 저장합니다. 그
연결의 명시적 project membership을 유지합니다. 각 관리 stdio 프로세스는 생성된 관리
launch 맥락에서 시작하고 권위 있는 runtime session 하나와 그 project session을
기록합니다.

어댑터는 각 tool 호출에서 현재 Connection, project membership, mode, 관리
runtime/project session, revision, Runtime Home, StorageManifest, project 선택을
검증합니다. Connection과 project context는 로컬에서 파생하며 공개 도구 인수가
선택하거나 덮어쓸 수 없습니다.

personal 연결은 사용자 소유 Codex 구성을 변경합니다. shared 연결은 지원되는 project
소유 Codex 구성을 변경합니다. 둘 다 같은 Core와 stdio 경계를 사용합니다.

## 결과

- 한 프로세스가 connection이나 project 경계를 묵시적으로 넘을 수 없습니다.
- project 이동 또는 교체에는 owner-defined 검증이나 repair가 필요합니다.
- 연결 레코드는 운영체제 권한을 부여하거나 사용자 신원을 증명하지 않습니다.
- Launch lease, client/host version, 경로, process 관찰은 actor나 binary identity를
  증명하지 않습니다.
- CLI 받은 편지함만 UserAction을 해결합니다.

정확한 필드와 명령은 [Agent Connection](../../reference/agent-connection.md),
[Administrative CLI](../../reference/admin-cli.md),
[MCP Transport](../../reference/mcp-transport.md)가 소유합니다.
