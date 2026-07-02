# Glossary

This glossary is a compact public terminology guide. It lists the terms users
need to decide what to do in ordinary Volicord workflows.

Complete structured terminology metadata lives in
[`docs/terminology-map.yaml`](../../terminology-map.yaml). The terminology map
also records visibility, allowed document layers, avoided synonyms, identifier
preservation, and owner routing. For exact contracts, use the focused owner
documents linked below or the [Reference Index](README.md).

Architecture and reference documents may use technical terms where precision is
needed. Terms such as Core, Change Unit, expected write, host-hook installation,
`actor_source`, `operation_category`, `ArtifactRef`, and
`StagedArtifactHandle` are intentionally not public glossary entries.

## Public Terms

| Term | Korean term | Short meaning | Primary owner |
|---|---|---|---|
| Runtime Home | 런타임 홈 | The local Volicord data space for operational records and configuration. | [Runtime Boundaries](runtime-boundaries.md) |
| Product Repository | 제품 저장소 | The user's project workspace and product files, separate from Volicord runtime state. | [Runtime Boundaries](runtime-boundaries.md) |
| Task | 작업 | The user-value unit being shaped, worked, blocked, or closed. | [Core Model](core-model.md) |
| Write Ticket | 쓰기 티켓 | A Volicord record that a proposed product-file change is compatible with the current task and scope. | [Core Model](core-model.md) |
| Evidence | 증거 | Recorded support for a specific claim, including runs, observations, or evidence attachments. | [Core Model](core-model.md) |
| User Judgment | 사용자 판단 | A decision that belongs to the user and must be recorded through the User Channel when it becomes Volicord state. | [Core Model](core-model.md) |
| Close Status | 닫기 상태 | Decision support for whether the current task can honestly finish from current Volicord records. | [Core Model](core-model.md) |
| Agent Connection | 에이전트 연결 | A local MCP host connection through which an agent can read or participate in supported Volicord workflows. | [Agent Connection Reference](agent-connection.md) |
| User Channel | 사용자 채널 | The local path for recording authority-bearing User Judgment. | [Core Model](core-model.md) |
| Record profile | 기록 프로필 | The Agent Connection profile for ordinary record-backed workflow use without requiring detective host hooks. | [Administrative CLI](admin-cli.md) |
| Detective profile | 탐지 프로필 | The Agent Connection profile that adds supported host-hook and watcher observations. | [Agent Connection Reference](agent-connection.md) |
| Local HTTP transport | 로컬 HTTP 전송 | The local MCP HTTP transport for localhost and Docker host-loopback operation. | [MCP Transport](mcp-transport.md) |
