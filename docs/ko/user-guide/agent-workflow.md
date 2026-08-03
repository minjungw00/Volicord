# 에이전트 가이드

advisor와 work Task는 shaping에서 시작합니다. 현재 checkpoint가 없으면 첫 checkpoint 생성을
명시하고, 연결된 live 결정이 남아 있지 않을 때만 정확한 현재 checkpoint 교체를 명시하여
분석을 `volicord.record_shaping`으로 기록합니다. 실행 가능한
사용자 소유 선택지를 제시하기 전에 현재 `UserActionRequest`를 만들며, 해결은 User
Channel에서만 받아들입니다. 결정은 현재 resolution 참조로 적용하고, Change Unit 생성이나
갱신은 단계를 바꾸지 않습니다. advisor는 비쓰기 Change Unit만 사용하며
`ready_to_finalize_advice` 뒤에 `volicord.record_shaping`의 advisor finalization을 호출한
다음 close review로 갑니다.
work는 태그 기반 workflow가 요구할 때만 `volicord.advance_task`를 호출합니다.
`volicord.record_run`은 direct 또는 implementation 실행에만 사용합니다.

<a id="purpose"></a>

Volicord에 연결된 세션에서 에이전트를 운영하거나 검토할 때 이 가이드를 사용합니다.
이 문서는 실무 작업 흐름을 설명합니다. 정확한 API, 스키마, 저장소, 보안, 닫기
계약은 [참조 색인](../reference/README.md)에 있습니다.

<a id="operating-loop"></a>

## 운영 순서

모든 작업에 적용할 메서드 순서를 외우지 않습니다. 최신 권한 상태인 태그 기반
`workflow`에서 시작합니다. null이 아닌 `required_action`을 정확한 `required_refs`와
`expected_state_version`으로 호출하고, `allowed_actions`를 닫힌 경계로 취급해 다른
workflow 도구를 시험하지 않습니다. 현재 Volicord 결과가 없거나 Task를 알 수 없거나
권한 상태를 새로고쳐야 할 때 먼저 현재 상태를 확인합니다. 안심하기 위해 단계 사이마다
status를 반복 조회하지 않습니다.

반환된 태그 기반 workflow 상태마다 다음을 수행합니다.

1. 의도한 Product Repository와 `Task`에 속한 결과인지 확인합니다.
2. 권한 상태를 바꾸지 않고 불확실성을 줄일 수 있는 파일, 문서, 테스트를 먼저
   확인합니다.
3. 사용자 소유 선택지를 보여 주기 전에 현재 UserAction 요청을 만들고, 대화 답변을
   그 해결로 취급하지 않습니다.
4. Volicord가 보여 주는 현재 적용 범위와 호환되는 쓰기 또는 민감 동작 경계 안에서만
   행동합니다.
5. 태그 기반 상태가 요구할 때 행동 뒤의 의미 있는 실행과 증거를 기록합니다.
6. 가장 중요한 차단 사유, 확인한 것, 빠진 것, 다음 행위자, 다음 안전한 행동 하나를
   보고합니다.
7. 증거, 최종 수락, 잔여 위험, Task 완료를 서로 구분합니다.

태그 기반 상태는 shaping, User Channel 해결, 결정 적용, Change Unit, 명시적 advance,
implementation 작업, close review 중 하나를 요구할 수 있습니다. 기록된 사실이 바뀌면
다음 응답이 다른 경로를 선택할 수 있습니다. 닫기 차단 사유의 해결 행동은 해당 차단
사유에만 속하며 현재 workflow 진행을 대신하지 않습니다.

## 에이전트 작업과 사용자 판단 분리

| 시점 | 에이전트 책임 | 사용자 책임 |
|---|---|---|
| 작업 구체화 | 맥락을 확인하고, 범위를 제안하고, 다음 안전한 행동을 이름 붙입니다. | 평소 말로 목표, 범위 밖 항목, 제한을 정합니다. |
| 판단 요청 | 현재 `UserActionRequest`를 만든 뒤 초점이 맞춰진 질문 하나, 선택지, 결과, 좁은 추천이 있다면 그 추천을 보여 줍니다. | 답하거나, 거절하거나, 미루거나, 범위를 줄이거나, 증거를 더 요구합니다. |
| 판단 기록 | 사용자를 지원되는 사용자 채널로 안내하고, 기록되지 않은 답변에 의존하지 않습니다. | 답변이 Volicord 상태가 되어야 하면 표시된 선택지 하나를 기록합니다. |
| 계속 진행 또는 닫기 | 상태를 새로고침하고, 쓰기를 준비하고, 증거와 차단 사유를 기록합니다. | 최종 수락, 잔여 위험 수락, 취소, 대체, 다음 차단 사유를 결정합니다. |

에이전트 연결은 사용자를 대신해 판단을 기록하면 안 됩니다. 대화 문장, 생성된
Markdown, 지침, 상태 보기는 판단 필요를 보여 줄 수 있지만 기록된 사용자 답변은
아닙니다. 정확한 권한 의미는 [Core 모델](../reference/core-model.md), 정확한 연결 경계는
[Agent Connection 참조](../reference/agent-connection.md)에 있습니다.

<a id="infer-use"></a>

## 작업에 맞는 절차 선택

사용자는 작업을 시작하려고 “Volicord”나 API 메서드 이름을 말할 필요가 없습니다.
필요한 경계를 보존하는 가장 작은 작업 흐름을 고릅니다.

- **조언 또는 확인:** 사용할 수 있는 출처를 확인하고, 불확실성을 말하며, 쓰기나
  닫기 절차를 붙이지 않습니다.
- **작은 변경:** 좁은 범위를 확인하고, 그 안에서 편집하고, 집중 점검을 실행하고,
  짧게 보고합니다.
- **추적 작업:** 범위를 구체화하고, 사용자 판단을 보존하고, 쓰기를 확인하고,
  증거와 닫기 상태를 보고합니다.

범위 확장, 새 공개 인터페이스, 의존성이나 마이그레이션 선택, 파괴적 위험,
보안·개인정보 영향, 증거 한계, 최종 수락 필요, 잔여 위험, 다른 사용자 소유 판단을
발견하면 작은 변경을 추적 작업으로 전환합니다.

아래 대표 흐름은 의도적으로 정확한 API 순서가 아닙니다.

| 작업 형태 | 반환된 전달 지점을 따르는 방법 |
|---|---|
| 조언 또는 읽기 전용 조사 | 사용할 수 있는 출처를 확인하고 불확실성을 말합니다. 작업에 필요하지 않은 쓰기나 닫기 절차를 만들지 않고 끝냅니다. |
| 좁은 제품 파일 변경 | 필요할 때만 Task를 만들고, 편집 전에 호환되는 현재 쓰기 권한을 얻고, 집중 검증을 실행하고, 의미 있는 결과를 기록한 뒤 반환된 닫기 또는 계속 행동을 따릅니다. |
| 여러 파일 또는 장기 작업 | 범위, 현재 Change Unit, 증거, 사용자 소유 판단을 보이게 유지합니다. 대화에서 순서를 다시 만들지 않고 태그 기반 workflow projection에서 재개합니다. |
| 사용자 또는 다른 차단 사유 대기 | 차단 사유와 다음 행위자를 보고합니다. 세션은 끝날 수 있지만 Task가 완료됐다고 주장하지 않습니다. |
| 민감하거나 새로 확장된 작업 | 영향을 받는 행동 전에 멈추고 상태 보기에 나온 정책, 범위, User Channel 전달 지점을 따릅니다. 스스로 승인하거나 더 가벼운 경로를 조용히 유지하지 않습니다. |

## 연결 설정과 Task 통제 분리

Codex `record` 프로필은 연결 설정을 선택하며 Task 위험 등급이 아닙니다. 각 Task에는
요청한 통제 수준, 유효 통제 수준, 담당 경로가 제공하는 이유가 따로 있습니다. 유효
수준과 프로젝트 소유 정책을 권한 기준으로 취급하고, 범위, 민감성, 외부 효과가 바뀌면
반환된 상향 동작을 따릅니다.

정확한 값과 결정 규칙은 [Core 모델](../reference/core-model.md),
[Intake](../reference/api/method-intake.md), 공개 스키마 담당 문서에 있습니다.

<a id="project-selection"></a>

## 프로젝트를 의도적으로 선택

에이전트 연결 하나에 둘 이상의 Product Repository가 명시적으로 연결될 수
있습니다. 기억, 폴더 라벨, 현재 작업 디렉터리만으로 프로젝트를 고르면 안 됩니다.

대상이 불분명하면 `volicord.list_projects`를 호출합니다. 작업 흐름 도구에
`project_selector` 인자가 있으면 반환된 값을 사용합니다. 프로젝트 선택이 모호해
호출이 거부되면 연결된 프로젝트를 나열하고, 의도한 프로젝트를 고른 뒤 다시
시도합니다.

정확한 선택 규칙은 [MCP 전송](../reference/mcp-transport.md)과
[Agent Connection 참조](../reference/agent-connection.md)를 보세요. 운영자 설정은
[여러 저장소 에이전트 설정](multi-repository-agent-setup.md)에 있습니다.

<a id="keep-context-small"></a>

## 맥락을 작게 유지

다음 행동에 필요한 정보만 유지합니다.

- 현재 `Task`, 현재 적용 범위, 범위 밖 항목, 관련 경로
- 현재 에이전트 연결의 역량 한계
- 대기 중인 사용자 행동이나 승인
- 다음 주장에 영향을 주는 증거 요약과 공백
- 현재 차단 사유, 오래된 상태 경고, 보이는 잔여 위험
- 다음 안전한 행동 하나

다음 행동에 필요할 때만 정확한 참조 섹션을 불러옵니다. 모든 프롬프트에 전체 스키마,
DDL, 템플릿, 로그, 증거 첨부 본문, 관련 없는 계약, 두 언어 문서를 넣지 않습니다.

<a id="clarify-focused"></a>
<a id="request-judgment-narrowly"></a>

## 집중된 질문으로 구체화

먼저 확인합니다. 답변이 다음 안전한 행동을 바꾸거나 사용자 소유 판단을 해결할 때만
질문합니다. 한 번에 막히는 질문 하나를 우선합니다.

좋은 질문은 다음을 보여 줍니다.

- 확인한 것과 남은 불확실성
- 현재 목표, 현재 적용 범위, 범위 밖 항목
- 선택지와 각 결과
- 현재 사실이 뒷받침할 때 좁은 추천
- 답변이 확정하는 것과 확정하지 않는 것
- 사용자가 미룰 때 안전하게 계속할 수 있는 일

에이전트가 안전하게 확인, 새로고침, 재시도, 범위 축소, 기록할 수 있는 일을
사용자에게 묻지 않습니다.

<a id="preserve-user-judgment"></a>
<a id="route-user-interaction"></a>

## 사용자 소유 판단 보존

사용자는 사용자에게 보이는 제품 동작, 중요한 기술 방향, 범위 변경, 새 의존성이나
서비스, 보안·개인정보 선택, 호환성을 깨는 변경, 되돌리기 어려운 선택, 민감 동작,
최종 수락, 잔여 위험 수락, 취소, 대체를 결정합니다.

에이전트는 받아들인 범위 안에서 제품 동작을 보존하는 지역 구현 세부사항을 보통
결정할 수 있습니다. 세부사항이 사용자에게 보이거나, 범위나 검증 기준을 바꾸거나,
의존성을 추가하거나, 보안·개인정보에 영향을 주거나, 호환성을 깨거나, 되돌리기
어려워지면 사용자에게 다시 올립니다.

“승인”, “좋아 보여”, “계속해”를 모든 대기 판단으로 해석하지 않습니다. 제품 방향,
기술 방향, 범위, 민감 동작 승인, 최종 수락, 잔여 위험 수락을 분리합니다.

판단이 Volicord 상태가 되어야 하면 먼저 현재 `UserActionRequest`를 만들고 지원되는
사용자 채널 경로를 보여 줍니다. 그 요청에 저장된 resolution만 권한을 제공하며 대화
문장은 제공하지 않습니다. 결정을 적용할 때 반환된 현재 resolution 참조를 사용합니다.
안정적인 CLI 대체 경로는 다음과 같습니다.

Resolution은 shaping 결정을 적용하지 않습니다. 정확한 outcome을 확인해야 합니다.
Accepted scope gap은 `volicord.update_scope`로 보냅니다. work의 accepted 제품·기술·민감
gap은 `volicord.advance_task`에 제공합니다. advisor에서는 정확한 accepted resolution을 보존하다가
`ready_to_finalize_advice`가 요구하는 `volicord.record_shaping` advisor finalization에
제공합니다. Finalization은 해당
결정을 적용하고 결과와 evidence/risk lineage를 기록하며 checkpoint를 보존하고 close
basis를 만듭니다. Scope와 다른 결정이 함께 있으면 scope gap만 먼저 적용하고 다른 gap은
모드별 owner에 남깁니다.
거부, 보류, 만료는 권한을 부여하지 않고 `decision_recovery_required`를 선택합니다.
`volicord.record_shaping`으로 계획을 수정해야 하며 종료되었거나 만료된 요청의 resolution을
다시 시도하면 안 됩니다. 수정된 계획에도 판단이 필요하면 successor UserAction 요청을
만들고 chat을 resolution으로 취급하지 말고 User Channel에 제시합니다.

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

지원되는 CLI inbox 전달 경계는
[Agent Connection 참조](../reference/agent-connection.md#supported-surface)에 있고,
정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md#user-channel-commands)에 있습니다.

<a id="check-before-writes"></a>

## 쓰기 전에 확인

제품 파일을 쓰기 전에 의도한 경로와 효과를 평가할 수 있을 만큼 구체화합니다.
`direct/implementation` 또는 `work/implementation`만 쓰기를 준비할 수 있습니다.
`workflow.required_action`이 쓰기 준비를 가리키면 호환되는 현재 쓰기 티켓을 발급받거나
재사용한 뒤 다음을 보여 줍니다.

- 의도한 변경
- 현재 적용 범위에 맞는지
- 대기 중인 사용자 행동이나 민감 동작 승인
- 오래됐거나 사용할 수 없는 맥락
- 쓰기 티켓을 발급할 수 없을 때 다음 행동

호환성에는 현재 정규화된 프로젝트 쓰기 권한에 대한 null이 아닌 정확한 결속이
포함됩니다. 정책 변경은 Task 통제 수준이 높아지지 않아도 이전 티켓을 무효화하거나
오래된 것으로 만들 수 있습니다. 현재 정책으로 제안된 쓰기를 다시 평가하도록 쓰기
준비를 새로 요청합니다. 그 결과 `sensitive` 통제로 높아지거나 새 민감 동작 승인이
필요할 수 있습니다. 쓰기 뒤 최종 수락은 쓰기 전에 필요했던 승인을 대신할 수 없습니다.

범위가 바뀌면 다음 쓰기 전에 반환된 범위 전달 지점을 따릅니다. 계획, 오래된 대화
맥락, 넓은 호응, 시간 경과만 있는 상태, 생성 요약만으로 쓰기 호환성을 주장하지
않습니다. 정확한 메서드 동작은
[쓰기 준비](../reference/api/method-prepare-write.md)에 있습니다.

<a id="record-evidence"></a>

## 행동 뒤에 증거 기록

의미 있는 편집, 명령, 검토, 관찰 뒤에는 다음을 보고합니다.

- 무엇을 실행했거나 바꿨는지
- 증거가 어떤 수락 기준 또는 보충 주장 대상을 뒷받침하는지
- 무엇이 통과하거나 실패했는지
- 무엇이 빠졌거나 오래됐거나 가려졌거나 막혔거나 부족한지

지원되는 실행 또는 관찰 경로로 대상별 증거를 기록합니다. 증거 첨부는 그 기록의
입력일 뿐이며, 첨부가 있다는 사실만으로 주장을 증명하지 않습니다. 증거, 닫기 상태,
최종 수락, 잔여 위험 수락을 구분합니다.

정확한 실행 동작은 [실행 기록](../reference/api/method-record-run.md)을 보세요. 정확한
첨부 동작은 [아티팩트 스키마](../reference/api/schema-artifacts.md)와
[아티팩트 저장소](../reference/storage-artifacts.md)에 있습니다.

<a id="reconcile-unrecorded-changes"></a>

## 미기록 변경 조정

탐지 프로필이 미기록 변경을 보고하면 범위가 제한된 관찰로 다룹니다. 파일을 바꾼
사람이나 악의적 동작을 증명하지 않습니다.

사용할 수 있으면 `volicord.reconcile_changes`를 사용합니다. MCP를 사용할 수 없으면
사용자를 `volicord changes reconcile`로 안내합니다. 사용자 수락은 지원되는 사용자
채널을 거쳐야 합니다. 관찰 불가 진단은 경로 finding과 분리하고, 담당 경로가 제공한
닫기 상태와 다음 행동을 보고합니다. 정확한 호출, delta, finding 의미는
[저장소 관찰](../reference/repository-observation.md)이 담당하고, 해결 동작은
[`volicord.reconcile_changes`](../reference/api/method-reconcile-changes.md)가
담당합니다.

<a id="report-status"></a>
<a id="handle-close"></a>

## 상태 보고와 닫기

가장 중요한 차단 사유와 이를 푸는 행동부터 말합니다. 간결한 상태 보고에는 현재 작업
경계, 현재 적용 범위, 최신 관련 사실, 대기 사용자 행동이나 승인, 증거 공백, 닫기 차단 사유,
다음 안전한 행동 하나가 들어갑니다.

닫기 전에는 다음 사실을 보여 줍니다.

- 범위와 결과
- 점검과 증거
- 필요한 사용자 판단
- 보이는 잔여 위험
- 남은 차단 사유
- 닫기를 가능하게 하는 다음 행동

작업이 검토 가능한 상태가 된 뒤 의도적인 close review 중에만 닫기 준비 상태를
사용합니다. 태그 기반 workflow가 허용할 때 읽기 전용 닫기 상태 점검으로 그 사실을
새로고치며, 이 검토는 workflow kind를 바꾸거나 대신하지 않습니다. 닫기 준비 상태로
shaping이나 implementation 진행을 선택하지 않고, 완료가 가까워졌다는 고정 절차 때문에
별도 점검을 넣지 않습니다. Task 상태는 지원되는 닫기 경로로만 바꿉니다. 산문,
테스트만 있는 상태, 넓은 수락 문구, 생성된
보기, 오래된 상태만으로 닫지 않습니다. 최종 수락과 잔여 위험 수락은 빠진 필수
증거를 대신하지 않습니다.

권한 상태를 새로고칠 수 없으면 마지막 결과를 만들어 내지 말고 Volicord 상태를
검증하지 못했다고 공개합니다.

정확한 닫기 의미는 [Core 모델](../reference/core-model.md), 정확한 메서드 동작은
[`Task` 닫기](../reference/api/method-close-task.md)에 있습니다.

<a id="instructions-and-guidance"></a>
<a id="respect-boundaries"></a>

## 범위와 보장 한계

Volicord 지침은 도구 선택을 유도할 수 있지만 접근 제어나 모델이 지침을 따랐다는
증명이 아닙니다. 쓰기 티켓은 파일시스템 권한이 아닙니다. 탐지 관찰은 OS 강제나
행위자 증명이 아닙니다. 증거와 닫기 상태는 정확성, QA, 배포, 사람 검토를 증명하지
않습니다.

지원되는 기능과 지원 범위 밖 기능은 [범위](../reference/scope.md), 정확한 보장과
비보장은 [보안](../reference/security.md)을 보세요. 이 가이드에 새 품질 관문이나 면제
경로를 만들지 않습니다.

<a id="language-context"></a>

## 언어 맥락

현재 사용자와 작업에 필요한 언어를 사용합니다. 정확한 API 이름, 명령, 필드, enum
값, 경로, 오류 코드는 보존합니다. 한국어 작업에서는 불필요한 영어 명사구를 그대로
옮기지 말고 일반 개념을 자연스러운 한국어로 씁니다.

<a id="where-next"></a>

## 다음 경로

- 연결 설정과 제거: [에이전트 호스트 설정](agent-host-setup.md)
- 연결 하나로 명시적으로 연결된 여러 저장소 처리:
  [여러 저장소 에이전트 설정](multi-repository-agent-setup.md)
- 사용자 협업 흐름: [사용자 작업 흐름](user-workflow.md)
- 다음 행동에 정확한 계약이 필요할 때: [참조 색인](../reference/README.md)
