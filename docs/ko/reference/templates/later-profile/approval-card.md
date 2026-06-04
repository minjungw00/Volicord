# 민감 동작 승인 카드(Approval Card) 템플릿

## 사용 시점

나중의 Approval 프로필에서 민감 행동(sensitive-action) 요청 범위, 목적, 경계, 위험, 대안, 만료/사용 방식, 추천안을 사용자에게 간결하게 보여줄 때 Approval Card를 사용합니다. 이 카드는 민감 동작 승인을 묻는 표시일 뿐이며 사용자 소유의 제품/UX 판단이나 기술 판단, 정확성 검토(correctness review), 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, Write Authorization이 아닙니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. Commit된 민감 동작 approval 지원이 active profile에 있을 때만 사용합니다. 최소 MVP-1의 민감 동작 승인 사용자 판단 요청은 간결한 판단 요청으로 충분하며 Approval Card가 필요하지 않습니다.

## 기준 기록

- committed Approval 기록
- 관련 민감 동작 승인 `user_judgment`
- 민감 category와 요청 범위
- 허용된 path, tool, command, network target, secret
- baseline 참조
- 위험, 대안, 추천안
- 표시될 때 관련 Write Authorization 경계, artifact refs, redaction state, projection freshness

`{approval_covers}`, `{approval_does_not_cover}` 같은 coverage placeholder는 Approval 범위, 관련 user judgment ref, 해당 profile이 active일 때의 연결된 Approval 기록, 현재 쓰기 또는 닫기 context에서 파생한 표시 전용 요약입니다. Approval 경계만 보여주며, active owner path가 계속 기준 출처입니다.

## 렌더링 섹션

- 민감 동작 승인 필요 여부
- 간결한 참조
- 요청 식별자
- 목적
- 허용 path
- 허용 tool
- 허용 command(`allowed_commands`)
- network
- 필요한 secret
- baseline
- 만료와 사용
- 위험
- 대안
- 추천
- 이 Approval이 포괄하는 것
- 이 Approval이 포괄하지 않는 것
- Approval 질문

## 전체 템플릿

````text
민감 동작 승인이 필요합니다.
표시 전용: Approval은 여전히 기준 Approval 결정 경로를 통해 기록되어야 합니다.
민감 동작 승인만 묻습니다. 사용자 소유의 제품/UX 판단이나 기술 판단, correctness, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, Write Authorization이 아닙니다.
참조: approval={approval_id}; judgment={user_judgment_ref|none}; write={write_authorization_ref|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

{approval_id} {category}
요청: {summary}
목적: {why_needed}
이 민감 동작 승인이 포괄하는 것:
{approval_covers}

이 민감 동작 승인이 포괄하지 않는 것:
{approval_does_not_cover}

허용 path:
{allowed_paths}

허용 tool:
{allowed_tools}

허용 command:
{allowed_commands}

네트워크:
{allowed_network}

필요한 secret:
{required_secrets}

Baseline(기준):
{baseline_ref}

만료와 사용:
expires={expires_at|scope_drift|none}; single_use={single_use_behavior|not_applicable}; write_authorization_requirement={later compatible prepare_write required}

위험:
{risks}

대안:
{alternatives}

추천:
{recommendation}

사용자 소유의 제품/UX 판단이나 기술 판단, 작업 수락, 잔여 위험 수용, 면제 판단은 별도로 두고, 이 sensitive action과 범위에만 민감 동작 승인을 부여하시겠습니까?
"go ahead", "proceed", "looks good", "좋아", "진행해"라고 답하더라도, 다른 user judgment가 표시되고 해소되지 않는 한 Harness는 이 민감 동작 승인만 기록합니다. 그 표현이 모호하면 기록하기 전에 다시 확인합니다.
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 Approval 권한 자체가 아닙니다. Commit된 Approval record는 later-profile이며, 최소 MVP-1의 민감 동작 승인 request는 간결한 판단 요청으로 보여줄 수 있습니다.

Approval은 사용자 소유의 제품/UX 판단이나 기술 판단을 해소하지 않고, correctness를 증명하지 않으며, 검증이나 수동 QA를 대체하지 않고, 작업 수락을 암시하지 않으며, 잔여 위험 수용을 대신하지 않고, QA나 검증을 면제하지 않고, Write Authorization을 만들지 않습니다. 실제 write에는 이후 호환되는 `prepare_write`와 Write Authorization이 여전히 필요합니다.

Approval Card는 민감 동작 승인 경계를 명시해야 합니다. 예를 들어 dependency install Approval은 architecture 결정이 아니고, secret access Approval은 secret 값 노출 허가가 아니며, auth 또는 system Approval은 session/JWT/social-login 선택이 아니고, 작업 수락은 추가 write 허가가 아니며, 잔여 위험 수용이나 면제 판단은 별도의 scoped decision path가 필요합니다.
