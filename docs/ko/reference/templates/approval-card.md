# Approval Card Template

## 사용 시점

대기 중인 Approval의 민감 행동(sensitive-action) 요청 범위, 목적, 경계, 위험, 대안, 추천안을 사용자에게 간결하게 보여줄 때 Approval Card를 사용합니다. 이 card는 민감한 행동을 허용할지 묻는 표시일 뿐이며 사용자 소유의 제품 판단이나 중요한 기술 판단, 정확성 검토(correctness review), 결과 수락, 남은 위험을 받아들이는 판단, Write Authorization이 아닙니다.

## 기준 기록

- Approval 기록
- Approval 형태의 Decision Packet
- 민감 category와 요청 범위
- 허용된 path, tool, command, network target, secret
- baseline 참조
- 위험, 대안, 추천안

`{approval_covers}`, `{approval_does_not_cover}` 같은 coverage placeholder는 Approval 범위, 연결된 Approval 기록, 관련 Decision Packet ref, 현재 쓰기 또는 닫기 context에서 파생한 표시 전용 요약입니다. Approval 경계만 보여주며, Approval 기록과 decision path가 계속 기준 출처입니다.

## 렌더링 섹션

- Approval 필요 여부
- 요청 식별자
- purpose
- allowed paths
- allowed tools
- allowed commands (`allowed_commands`)
- network
- required secrets
- baseline
- risks
- alternatives
- recommendation
- 이 Approval이 포괄하는 것
- 이 Approval이 포괄하지 않는 것
- Approval 질문

## 전체 템플릿

````text
Approval이 필요합니다.
표시 전용: Approval은 여전히 기준 Approval 결정 경로를 통해 기록되어야 합니다.
민감 행동 허용만 묻습니다. 사용자 소유의 제품 판단이나 중요한 기술 판단, correctness, 최종 수락, 남은 위험을 받아들이는 판단, Write Authorization이 아닙니다.

{approval_id} {category}
요청: {summary}
목적: {why_needed}
이 Approval이 포괄하는 것:
{approval_covers}

이 Approval이 포괄하지 않는 것:
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

Baseline:
{baseline_ref}

위험:
{risks}

대안:
{alternatives}

추천:
{recommendation}

사용자 소유의 제품 판단이나 중요한 기술 판단, 결과 수락은 별도로 두고, 이 sensitive action과 범위만 승인하시겠습니까?
"go ahead" 또는 "진행해"라고 답하더라도, 다른 Decision Packet이 표시되고 해소되지 않는 한 Harness는 이 sensitive-action Approval만 기록합니다.
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 Approval 권한 자체가 아닙니다. Approval은 여전히 기준 Approval 결정 경로를 거쳐야 합니다.

Approval은 사용자 소유의 제품 판단이나 중요한 기술 판단을 해소하지 않고, correctness를 증명하지 않으며, verification이나 Manual QA를 대체하지 않고, 결과 수락을 암시하지 않으며, 남은 위험을 받아들이는 판단을 대신하지 않고, Write Authorization을 만들지 않습니다.

Approval Card는 Approval 경계를 명시해야 합니다. 예를 들어 dependency install Approval은 architecture 결정이 아니고, secret access Approval은 secret 값 노출 허가가 아니며, auth 또는 system Approval은 session/JWT/social-login 선택이 아니고, 최종 수락은 추가 write 허가가 아닙니다.
