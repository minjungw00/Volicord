# 민감 동작 승인 카드(Approval Card) 템플릿

## 사용 시점

나중의 Approval 프로필에서 민감 동작 요청 범위, 목적, 경계, 위험, 대안, 만료/사용 방식, 추천안을 사용자에게 간결하게 보여줄 때 민감 동작 승인 카드(Approval Card)를 사용합니다. 이 카드는 민감 동작 승인을 묻는 표시일 뿐이며 사용자 소유의 제품/UX 판단이나 기술 판단, 정확성 검토(correctness review), 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 쓰기 허가 기록(Write Authorization)이 아닙니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 보증 프로필 보고서입니다. 커밋된 민감 동작 승인(Approval) 지원이 활성 프로필에 있을 때만 사용합니다. 최소 MVP-1의 민감 동작 승인 사용자 판단 요청은 간결한 판단 요청으로 충분하며 Approval Card가 필요하지 않습니다.

## 기준 기록

- 커밋된 민감 동작 승인(Approval) 기록
- 관련 민감 동작 승인 `user_judgment`
- 민감 범주와 요청 범위
- 허용 경로, 도구, 명령, 네트워크 대상, 필요한 비밀 정보
- 기준선 참조
- 위험, 대안, 추천안
- 표시될 때 관련 쓰기 허가 기록(Write Authorization) 경계, 아티팩트 참조, 가림 상태, 읽기용 보기 최신성(projection freshness)

`{approval_covers}`, `{approval_does_not_cover}` 같은 coverage placeholder는 민감 동작 승인(Approval) 범위, 관련 사용자 판단 참조, 해당 프로필이 활성화된 경우의 연결된 Approval 기록, 현재 쓰기 또는 닫기 맥락에서 파생한 표시 전용 요약입니다. 민감 동작 승인 경계만 보여주며, 활성 담당 경로가 계속 기준 출처입니다.

## 렌더링 섹션

- 민감 동작 승인 필요 여부
- 간결한 참조
- 요청 식별자
- 목적
- 허용 경로
- 허용 도구
- 허용 command(`allowed_commands`)
- 네트워크
- 필요한 비밀 정보
- 기준선
- 만료와 사용
- 위험
- 대안
- 추천
- 이 민감 동작 승인이 포괄하는 것
- 이 민감 동작 승인이 포괄하지 않는 것
- 민감 동작 승인 질문

## 전체 템플릿

````text
민감 동작 승인이 필요합니다.
표시 전용: 민감 동작 승인(Approval)은 여전히 기준 Approval 결정 경로를 통해 기록되어야 합니다.
민감 동작 승인만 묻습니다. 사용자 소유의 제품/UX 판단이나 기술 판단, 정확성, 작업 수락, 잔여 위험 수용, QA 면제 판단, 검증 면제 판단, 쓰기 허가 기록(Write Authorization)이 아닙니다.
참조: approval={approval_id}; judgment={user_judgment_ref|none}; write={write_authorization_ref|none}; artifacts={artifact_refs|none}; redaction={redaction_availability_summary|none}; freshness={projection_freshness}

{approval_id} {category}
요청: {summary}
목적: {why_needed}
이 민감 동작 승인이 포괄하는 것:
{approval_covers}

이 민감 동작 승인이 포괄하지 않는 것:
{approval_does_not_cover}

허용 경로:
{allowed_paths}

허용 도구:
{allowed_tools}

허용 command:
{allowed_commands}

네트워크:
{allowed_network}

필요한 비밀 정보:
{required_secrets}

기준(Baseline):
{baseline_ref}

만료와 사용:
expires={expires_at|scope_drift|none}; single_use={single_use_behavior|not_applicable}; write_authorization_requirement={later compatible prepare_write required}

위험:
{risks}

대안:
{alternatives}

추천:
{recommendation}

사용자 소유의 제품/UX 판단이나 기술 판단, 작업 수락, 잔여 위험 수용, 면제 판단은 별도로 두고, 이 민감 동작과 범위에만 민감 동작 승인을 부여하시겠습니까?
"go ahead", "proceed", "looks good", "좋아", "진행해"라고 답하더라도, 다른 사용자 판단이 표시되고 해소되지 않는 한 Harness는 이 민감 동작 승인만 기록합니다. 그 표현이 모호하면 기록하기 전에 다시 확인합니다.
````

## 메모

이 template은 렌더링 결과인 카드 형태일 뿐 민감 동작 승인(Approval) 권한 자체가 아닙니다. 커밋된 민감 동작 승인(Approval) 기록은 later-profile이며, 최소 MVP-1의 민감 동작 승인 요청은 간결한 판단 요청으로 보여줄 수 있습니다.

민감 동작 승인(Approval)은 사용자 소유의 제품/UX 판단이나 기술 판단을 해소하지 않고, 정확성을 증명하지 않으며, 검증이나 수동 QA를 대체하지 않고, 작업 수락을 암시하지 않으며, 잔여 위험 수용을 대신하지 않고, QA나 검증을 면제하지 않고, 쓰기 허가 기록(Write Authorization)을 만들지 않습니다. 실제 쓰기에는 이후 호환되는 `prepare_write`와 쓰기 허가 기록(Write Authorization)이 여전히 필요합니다.

민감 동작 승인 카드(Approval Card)는 민감 동작 승인 경계를 명시해야 합니다. 예를 들어 의존성 설치 승인(Approval)은 아키텍처 결정이 아니고, 비밀 정보 접근 승인(Approval)은 비밀 정보 값 노출 허가가 아니며, auth 또는 system 승인(Approval)은 session/JWT/social-login 선택이 아니고, 작업 수락은 추가 쓰기 허가가 아니며, 잔여 위험 수용이나 면제 판단은 별도의 범위 지정 판단 경로가 필요합니다.
