# 용어집

이 용어집은 공개 용어를 간결하게 안내합니다. 일반적인 Volicord 작업 흐름에서
사용자가 무엇을 할지 결정하는 데 필요한 용어만 싣습니다.

완전한 구조화 용어 메타데이터는
[`docs/terminology-map.yaml`](../../terminology-map.yaml)에 있습니다. 용어 지도는
visibility, 허용 문서 계층, 피할 동의어, 식별자 보존, 담당 경로도 기록합니다.
정확한 계약은 아래 집중 담당 문서나 [참조 색인](README.md)을 봅니다.

아키텍처와 참조 문서는 정밀성이 필요할 때 기술 용어를 사용할 수 있습니다. Core,
Change Unit, expected write, host-hook installation, `actor_source`, `operation_category`,
`ArtifactRef`, `StagedArtifactHandle` 같은 용어는 의도적으로 공개 용어집 항목에
넣지 않습니다.

## 공개 용어

| 용어 | 한국어 용어 | 짧은 의미 | 주 담당 문서 |
|---|---|---|---|
| Runtime Home | 런타임 홈 | 운영 기록과 설정을 위한 로컬 Volicord 데이터 공간입니다. | [런타임 경계](runtime-boundaries.md) |
| Product Repository | 제품 저장소 | Volicord 런타임 상태와 구분되는 사용자의 프로젝트 작업 공간과 제품 파일입니다. | [런타임 경계](runtime-boundaries.md) |
| Task | 작업 | 구체화되거나, 진행되거나, 막히거나, 닫히는 사용자 가치 단위입니다. | [Core 모델](core-model.md) |
| Write Ticket | 쓰기 티켓 | 제안된 제품 파일 변경이 현재 작업과 범위에 맞는다는 Volicord 기록입니다. | [Core 모델](core-model.md) |
| Evidence | 증거 | 실행, 관찰, 증거 첨부를 포함해 특정 주장을 뒷받침하는 기록입니다. | [Core 모델](core-model.md) |
| User Judgment | 사용자 판단 | 사용자에게 속한 결정이며 Volicord 상태가 되어야 할 때 User Channel을 통해 기록합니다. | [Core 모델](core-model.md) |
| Close Status | 닫기 상태 | 현재 Volicord 기록에서 현재 작업을 정직하게 끝낼 수 있는지 판단하도록 돕는 상태입니다. | [Core 모델](core-model.md) |
| Agent Connection | 에이전트 연결 | 에이전트가 지원되는 Volicord workflow를 읽거나 참여할 수 있는 로컬 MCP 호스트 연결입니다. | [Agent Connection 참조](agent-connection.md) |
| User Channel | 사용자 채널 | 권한을 지니는 User Judgment를 기록하는 로컬 경로입니다. | [Core 모델](core-model.md) |
| Record profile | 기록 프로필 | detective host hook을 요구하지 않고 일반 기록 기반 workflow를 쓰는 Agent Connection profile입니다. | [관리 CLI](admin-cli.md) |
| Detective profile | 탐지 프로필 | 지원되는 host hook과 watcher 관찰을 더하는 Agent Connection profile입니다. | [Agent Connection 참조](agent-connection.md) |
| Local HTTP transport | 로컬 HTTP 전송 | 로컬 HTTP 동작에 쓰는 loopback 전용 MCP 전송입니다. | [MCP 전송](mcp-transport.md) |
