# 통합 UserAction 요청과 해결

## 맥락

agent는 진행에 로컬 사용자의 판단이 필요함을 발견할 수 있습니다. 요청은 영속적이고
재개 가능해야 하지만 agent-facing channel이 그 사용자를 가장하면 안 됩니다.

## 결정

Core는 엄격한 `UserActionRequest` 하나와 최대 하나의 immutable
`UserActionResolution`을 소유합니다. `volicord.request_user_action`은 pending
요청을 생성하거나 명시적 read-only resume branch를 사용합니다. MCP 어댑터는
agent-safe summary를 반환하며 완전한 resolving form을 받지 않습니다.

로컬 CLI 받은 편지함이 엄격한 form을 읽고 표시합니다. CLI resolution 경로만 local-user
provenance를 제공하고 `volicord.resolve_user_action`을 호출합니다. resolution은 하나의
원자적 mutation에서 요청, expiry, 현재 작업 좌표, 정규 answer, replay identity를 다시
검증합니다.

Guard prompt capture는 관찰입니다. delivery channel로 동작하거나 답을 제출하거나 user
authority를 만들 수 없습니다.

## 결과

- 원래 request result와 이후 resolution은 별도 레코드로 남습니다.
- 한 요청에는 최대 하나의 resolution이 있습니다. 일치하는 replay는 원래 결과를
  반환하고 conflict는 분기할 수 없습니다.
- expired, stale, corrupt, irrelevant 요청은 answer mutation 없이 소유된 branch로
  실패합니다.
- MCP는 요청을 생성하거나 재개할 수 있지만 해결할 수 없습니다.
- schema의 현재 delivery path는 `channel_kind=cli` 하나입니다.

[User Action Schema](../../reference/api/schema-user-action.md),
[Request User Action](../../reference/api/method-request-user-action.md),
[Resolve User Action](../../reference/api/method-resolve-user-action.md)를 봅니다.
