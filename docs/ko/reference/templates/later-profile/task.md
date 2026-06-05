# TASK 템플릿

## 권한 규칙

- 상태 보기(Projection)는 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- 상태 보기(Projection)는 Core 상태가 아닙니다.
- 사용자가 상태 보기(Projection)를 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- 채팅과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

## 사용 시점

전체 보고서가 명시적으로 유용한 나중 프로필(later-profile) 단계에서, 진행 중인 작업을 이어서 파악할 수 있는 연속성 보기 또는 참조 상태 보기가 필요할 때 `TASK`를 사용합니다. 이 템플릿은 범위, 사용자 판단, 증거, 닫기 준비 상태, 작업의 현재 위치, 사용자 판단 맥락, 차단 사유 소유자, 자율성 경계(Autonomy Boundary), `Write Authority Summary`, 구현 마이크로 계획, 검토 단계, 스튜어드십 영향, 다음 증거, 잔여 위험, 닫기 요약, 필요할 때의 커널 관문 상세, 활성 작업 조각(Change Unit), 대기 중인 판단, 관련 보고서 참조, 읽기용 보기 최신성을 보여줄 수 있습니다.

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 상태 보기(projection)입니다. `TASK`는 MVP-1 사용자 작업 루프 상태 보기가 아닙니다. MVP-1의 사용자 대상 상태는 [상태 카드](../status-card.md)가 담당하고, 사용자 판단이 필요하면 [판단 요청](../judgment-request.md) 또는 사용자 판단 리소스(user-judgment resource)가 담당합니다. 독립형 판단 패킷(Decision Packet)은 복잡한 판단을 위한 선택적 전체 형식 표시입니다. 전체 `TASK` 본문은 나중 프로필 다듬기 범위입니다.

이 저장소에 `TASK` 템플릿이 있다는 사실은 현재 단계에서 전체 `TASK` Markdown이 필요하다는 뜻이 아닙니다.

## 기준 기록

- `state.sqlite` Task와 Task 관문(task gate)
- 활성 작업 조각(Change Unit)과 작업 조각 의존성(Change Unit dependency)
- `mode`, `lifecycle`, 다음 행동(next action), 가장 먼저 해소할 차단 사유, 가장 작은 해소 방법, 보장 수준, 읽기용 보기 최신성(projection freshness)을 위한 현재 상태 표시 입력
- 기존 owner 기록, 관문, 차단 사유, 참조에서 파생되는 범위, 사용자 판단, 증거, 닫기 준비 상태 표시 그룹 입력
- 쓰기 승인 기록(Write Authorization)과 `Write Authority Summary` 표시 입력
- 사용자 판단(User Judgment) 기록과 잔여 위험(Residual Risk), 해당 프로필이 켜졌을 때 전체 형식 판단 패킷(Decision Packet) 표시 필드
- 최신 실행(Run), 증거 요약, ArtifactRef 참조, 그리고 일치하는 프로필이 활성화된 경우 증거 목록(Evidence Manifest), Eval(분리 검증 결과), 수동 QA 기록, 민감 동작 승인 기록
- 쓰기 승인 기록(Write Authorization), 사용자 판단(User Judgment), 민감 동작 승인 사용자 판단 참조, 나중의 민감 동작 승인(Approval) 참조, `evidence_ref` 참조와 파생 증거 요약, 활성화된 경우 증거 목록(Evidence Manifest), Eval(분리 검증 결과), 수동 QA, 최종 수락 맥락, 잔여 위험(Residual Risk), 아티팩트 참조, `redaction_state`, 읽기용 보기 최신성(projection freshness) 권한 주장을 표시할 때 필요한 간결한 출처 참조
- 가장 먼저 해소할 차단 사유, 추가 차단 사유, 가장 작은 해소 방법 표시 요약
- 변경된 범위, 민감 동작 승인, 증거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수락, 최종 수락, 면제 판단 상태, 닫기 이유를 포함하는 닫기 요약 표시 입력
- 이어가기 축(Journey Spine) 기준 기록
- `domain_terms`, `module_map_items`, `interface_contracts`, `feedback_loops`
- TDD가 선택된 경우 `tdd_traces`
- 설계 품질(design-quality) 검증기 결과
- 예상되는 증거 필요 항목
- 기존 owner 기록과 참조에서 온 검토 단계(Review Stage) 표시 입력
- 아티팩트 참조 및 읽기용 보기 최신성(projection freshness)

`TASK`의 생성된 관문 그룹 요약(gate group summary), 사용자 판단 표시 문구, 닫기, 면제, 검토 단계(review-stage), 스튜어드십(stewardship), 보기 최신성(projection-freshness) 항목은 표시 바인딩입니다. 위에 나열한 owner 기록, 관문, 아티팩트, 참조로 해소되어야 하며, 그런 출처가 없으면 명시적인 부재/차단 사유 상태로 렌더링해야 합니다. 제품 판단 또는 최종 수락 같은 라벨을 렌더링해도 기준 기록, 관문, `ProjectionKind` value, 증거, 수동 QA, 검증, 최종 수락, 잔여 위험 수락, 닫기, 쓰기 승인 기록(Write Authorization)을 만들지 않습니다.

## 렌더링 섹션

나중 프로필이 전체 보고서를 켜면 `TASK`는 다음과 같은 섹션을 렌더링할 수 있습니다.

- 관문 그룹 요약
- 현재 요약
- 현재 위치
- 사용자 판단 맥락
- 권한 출처 참조
- 자율성 경계(Autonomy Boundary)
- `Write Authority Summary`
- 구현 마이크로 계획
- 검토 단계
- 다음 증거
- 잔여 위험
- 닫기 요약
- 스튜어드십 영향
- 목표
- 범위
- 수용 기준
- 활성 작업 조각(Change Unit)
- 대기 중인 판단
- 증거와 보고서
- 사용자 메모와 제안

장기 `work` Task는 공유 설계, 도메인 용어 참조, 모듈/인터페이스 참조, 작업 조각 의존성(Change Unit dependency), 구현 세부사항, 이어가기 축(Journey Spine)을 위한 확장 관리 섹션을 표시할 수 있습니다.

## 전체 템플릿

이것은 향후/프로필 보고서 형태입니다. MVP 상태 카드가 아니며 기준 출처도 아닙니다.

````md
---
doc_type: task
task_id: TASK-0001
display_state: executing
projection_version: 7
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# TASK-0001 작업 제목

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링된 보기입니다. 관리 섹션은 생성된 표시 영역이며, 그 안의 편집은 상태 변경이 아니라 drift(불일치) 또는 조정(reconcile) 후보입니다.

<!-- HARNESS:BEGIN managed -->
## 관문 그룹 요약
- 범위:
  - 바뀔 수 있는 것:
  - 범위 밖:
  - 쓰기 전 범위 확인 / 쓰기 승인 기록(Write Authorization):
  - 차단 사유 / 가장 작은 해소 방법:
  - 출처 참조:
- 사용자 판단:
  - 대기 중인 항목(판단마다 한 줄, 병합하지 않음):
  - 판단 요청:
    - 제품 판단:
    - 기술 판단:
  - 권한:
    - 민감 동작 승인:
  - 면제:
    - 관련 사용자 판단 참조:
  - 수락:
    - 최종 수락:
  - 위험 수락:
    - 잔여 위험 수락:
    - 수락하는 이름 붙은 위험:
  - 판단 / 민감 동작 승인 / 면제 / 수락 / 위험 참조:
  - 차단 사유 / 가장 작은 해소 방법:
  - 에이전트가 계속할 수 있는 것:
- 증거:
  - 증거 상태:
  - 뒷받침 참조:
  - 빠졌거나 오래된 뒷받침:
  - 아티팩트 가림 또는 생략 상태:
  - 대체하지 않는 것: 검증, 수동 QA, 최종 수락, 잔여 위험 수락
  - 다음 증거 행동:
- 닫기 준비 상태:
  - 검증:
  - 수동 QA:
  - 민감 동작 승인:
  - 최종 수락:
  - 잔여 위험 표시:
  - 잔여 위험 수락:
  - 면제 상태:
  - 닫기 차단 사유 / 닫기 이유:
  - 가장 작은 해소 방법:
- 메모: 이 항목들은 표시 그룹일 뿐입니다. 정확한 관문(gate) 값, 재계산 규칙(recompute rule), 닫기 의미(close semantics)는 Core Model 참조가 담당합니다.

## 현재 요약
- `mode`:
- 생명주기 단계(`lifecycle phase`):
- 결과(`result`):
- 닫기 이유:
- 보장(`assurance`):
- 범위 요약:
- 범위 밖:
- 다음 행동:
- 확인한 것:
- 남은 것:
- 가장 먼저 해소할 차단 사유:
- 차단 사유 소유자:
- 가장 작은 해소 방법:
- 추가 차단 사유:
- 대기 중인 판단:
- 대기 중인 판단 유형:
- 사용자가 판단하는 것:
- 위험:
- 관문 표시 그룹: 범위=; 사용자 판단=; 증거=; 닫기 준비 상태=
- 보장 수준:
- 커널 관문 상세: scope=; decision=; approval=; design=; evidence=; verification=; 수동 QA=; acceptance=
- 활성 작업 조각(Change Unit):
- Write Authority Summary:
- 권한 출처 참조: 쓰기=; 판단=; 민감동작승인=; 증거요약=; 활성증거목록=; Eval=; 수동QA=; 최종수락=; 잔여위험=; 아티팩트=
- `redaction_state`:
- 최신 보고서:
- 보기 최신성:

## 현재 위치
- 현재 위치:
- 활성 경로:
- 확인한 것:
- 남은 것:
- 가장 먼저 해소할 차단 사유:
- 차단 사유 소유자:
- 가장 작은 해소 방법:
- 추가 차단 사유:
- 최신 의미 있는 증거:
- 다음 상태 전이:

## 사용자 판단 맥락
- 대기 중인 사용자 판단:
- 대기 중인 판단 항목:
- user_judgment_ref:
- 판단 유형:
- 판단 제목:
- judgment_kind:
- presentation:
- 표시 라벨(렌더링):
- 지금 필요한 이유:
- 사용자가 판단하는 것:
- 선택지:
- 장단점 비교:
- 추천:
- 불확실성:
- 미룰 때의 영향:
- 해당되는 경우 잔여 위험:
- 수락하는 이름 붙은 위험:
- 에이전트가 사용자 없이 결정해도 되는 것:
- 이 판단이 확정하지 않는 것:
- 일반 동의 표현 처리:
- 되돌릴 수 있는지:
- 영향받는 범위:
- 판단에 필요한 최소 맥락:
- 영향받는 표시 그룹:
- 영향받는 관문 참조:

## 권한 출처 참조
- 쓰기 승인 기록(Write Authorization):
- 사용자 판단:
- 민감 동작 승인 사용자 판단 / 민감 동작 승인(Approval) 참조:
- 증거 요약 / 활성화된 경우 증거 목록(Evidence Manifest):
- Eval(분리 검증 결과):
- 수동 QA:
- 최종 수락 사용자 판단:
- 최종 수락 맥락:
- 잔여 위험(Residual Risk):
- 아티팩트 참조와 `redaction_state`:
- 보기 최신성:

## 자율성 경계(Autonomy Boundary)
- 프로필:
- 에이전트가 할 수 있는 일:
- 필요한 사용자 판단:
- AFK 중단 조건:
- 경계 상태:

## Write Authority Summary
- 활성 작업 조각(Change Unit):
- 쓰기 승인 기록:
- 허용 경로:
- 허용 도구:
- 허용 명령:
- 허용 네트워크 대상:
- 비밀 정보 범위:
- 민감 범주:
- 민감 동작 승인 상태:
- 기준선:
- 보장 수준:
- 메모: 자율성 경계(Autonomy Boundary)는 판단 재량이지 쓰기 전 범위 확인이나 쓰기 승인 기록이 아니다.

## 구현 마이크로 계획
- 메모: 실행 보조 정보일 뿐입니다. 활성 작업 조각(Change Unit) 범위가 쓰기를 제한하고 `prepare_write`가 쓰기 승인 기록(Write Authorization)을 만듭니다.
- TDD 메모: `required`이면 선택된 피드백 루프, RED 대상, GREEN 대상, 테스트 외 구현(non-test implementation)이 실제 RED 증거 또는 면제를 기다리는지 표시합니다.

| 단계 / 조각 | 목적 | 활성 작업 조각(Change Unit) 범위 / 예상 경로 | 피드백 루프 / TDD | 예상 증거 | 멈추고 사용자에게 물을 때 |
|---|---|---|---|---|---|
| 1 | | | | | |

## 검토 단계
- 메모: 관리되는 표시 전용입니다. 역할 렌즈(Role Lens)/playbook 라벨은 관문, 기록, `ProjectionKind` value, 민감 동작 승인, 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 수락, 닫기, 쓰기 승인 기록(Write Authorization)을 만들지 않습니다. 같은 세션 검토는 분리 검증이 아닙니다. 발견 사항은 기존 owner 기록, 참조, 관문, 차단 사유로 연결합니다.

### 명세 준수 검토
- 수용 기준 뒷받침 범위:
- 작업 조각(Change Unit) 완료 조건:
- 범위 / Write Authorization 호환성:
- 사용자 판단 호환성:
- 증거 뒷받침 범위:
- 잔여 위험 표시:
- 라우팅된 결과(기존 경로/참조만):

### 코드 품질 / 스튜어드십 검토
- 도메인 언어:
- 모듈 / 인터페이스 경계:
- 수직 조각(vertical slice) 형태:
- 피드백 루프 / TDD:
- 코드베이스 스튜어드십:
- 맥락 정돈:
- 후속 위험:
- 라우팅된 결과(기존 경로/참조만):

## 다음 증거
- 다음 증거 행동:
- 증거가 필요한 이유:
- TDD RED 대상 / 계획:
- TDD RED 증거:
- TDD GREEN 증거:
- TDD 리팩터/확인 증거:
- 예상 아티팩트 참조:
- 생략/차단 아티팩트 영향:
- 오래되었거나 빠진 증거:

## 잔여 위험
- 닫기 관련 위험:
- 표시 상태:
- 상태 값:
- 수락하는 이름 붙은 위험:
- 잔여 위험 수락 상태:
- 수락한 잔여 위험 참조:
- 후속 작업 필요:
- 닫기 영향:

## 닫기 요약
- 변경된 범위:
- 증거:
- 검증:
- 수동 QA:
- 민감 동작 승인:
- 잔여 위험 표시:
- 잔여 위험 수락:
- 최종 수락:
- 최종 수락이 대체하지 않는 것:
- 면제 상태:
- 권한 출처 참조:
- 표시 상태 라벨(일반 문구, schema 값 아님):
- 자체 확인 참조:
- 분리 검증 Eval 참조:
- 검증 위험 수락 참조:
- QA 면제 판단 참조:
- 수락한 잔여 위험 참조:
- 닫기 이유:
- 남은 후속 작업:

## 스튜어드십 영향
- 요약 형태: StewardshipImpactSummary
- domain_language_impact: none | updated | conflict | unresolved
- module_boundary_impact: none | local | public_boundary | unresolved
- interface_contract_impact: none | compatible | breaking | unresolved
- feedback_loop_status: defined | missing | waived
- future_change_risk: none | visible | accepted | unresolved
- close_impact: none | blocks_close | requires_decision | residual_risk
- 참조:
  - 도메인 용어 참조:
  - 모듈 맵 항목 참조:
  - 인터페이스 계약 참조:
  - 피드백 루프 참조:
  - 선택된 경우 TDD 트레이스 참조:
  - 잔여 위험:
  - 사용자 판단:

## 목표
-

## 범위
### 포함
-

### 제외
-

## 수용 기준
- [ ] AC-01:
- [ ] AC-02:

## 활성 작업 조각(Change Unit)
| ID | 목적 | 상태 | 조각 유형 | TDD | 수동 QA | Core 검증 |
|---|---|---|---|---|---|---|
| CU-01 | | | vertical | trace 상태: required \| recorded \| waived \| not_required; RED/GREEN ref 표시 | pending | |

## 대기 중인 사용자 판단
| 표시 라벨 | 질문 | `judgment_kind` / 참조 | 상태 | 다음 행동 |
|---|---|---|---|---|
| 제품 판단 \| 기술 판단 \| 민감 동작 승인 \| 최종 수락 \| 잔여 위험 수락 | | | | |

## 증거와 보고서
- 증거 요약 / 활성화된 경우 증거 목록(Evidence Manifest):
- 실행 요약:
- Eval(분리 검증 결과):
- 직접 작업 결과(Direct Result):
- TDD 트레이스:
- 수동 QA:
- 민감 동작 승인(Approval):
- 판단 요청(Decision):
- 변경 차이:
- 로그:
- `redaction_state`가 있는 아티팩트 참조:
- 보기 최신성:
<!-- HARNESS:END managed -->

## 사용자 메모와 제안
<!-- 사람이 편집 가능: 여기의 메모와 제안은 reconcile 입력이며, Core를 통해 accepted되기 전에는 상태를 바꾸지 않습니다. -->
-
````

장기 `work` Task를 위한 확장 TASK 섹션:

````md
<!-- HARNESS:BEGIN managed -->
## 공유 설계 개념
### 해소된 질문
| ID | 질문 | 사용자 답변 | 결정 / 가정 |
|---|---|---|---|

### 남은 모호함
- 항목 / 소유자 / 중단 조건:

## 도메인 용어 참조
- 적용 중인 용어:
  - 용어:

## 모듈과 인터페이스 참조
- 모듈 맵 항목 참조:
- 인터페이스 계약 참조:
- 표시되는 경우 렌더링된 상태 보기 참조: MODULE-MAP, INTERFACE-CONTRACT
- DESIGN:

## 작업 조각(Change Unit) 의존성
| ID | 차단 원인(`blocked_by`) | 해소 대상(`unblocks`) | 병렬 가능 항목(`parallelizable_with`) | 병합 위험 |
|---|---|---|---|---|

## 구현 마이크로 계획 상세
- 출처 정렬: 현재 Task, 활성 작업 조각(Change Unit), 관문, 관련 참조
- 경계: 기준 상태 아님, 범위 권한 아님, 민감 동작 승인(Approval) 아님, 쓰기 승인 기록(Write Authorization) 아님. 활성 작업 조각(Change Unit)이 범위의 기준 출처로 남음

### 단계 대기열(Step Queue)
| 단계 | 상태 정렬 | 범위 정렬 / 예상 경로 | 피드백 루프 / TDD 상태 | 증거 목표 | 중단 조건 |
|---|---|---|---|---|---|

## 이어가기 축(Journey Spine)
### 적용 중인 사실
- 사실 / 증거 참조:

### 적용 중인 가정
- 가정 / 만료 조건:

### 적용 중인 결정
- DEC-0001:

### 적용 중인 도메인 용어
- 용어 / 의미 / 코드 표현:

### 모듈 / 인터페이스 영향
- 모듈 / 영향 / 인터페이스 / 테스트 경계:

### 거절한 선택지
- 선택지 / 이유 / DEC:

### 주의 지점(Watchpoints)
- 회귀:
- 보안/성능/운영:
- 아키텍처 불일치(drift):

### 이어가기 메모
- 다음 세션이 알아야 할 것:
- 가장 먼저 해소할 차단 사유:
- 가장 작은 해소 방법:
<!-- HARNESS:END managed -->
````

작업 조각(Change Unit) 블록 하위 템플릿:

````md
### CU-01 제목
- 목적:
- 목표가 아닌 것:
- 조각 유형: vertical | enabling | cleanup | horizontal-exception
- 수평 예외(horizontal exception) 이유:
- 후속 수직 CU:
- 자율성 프로필:
- 에이전트가 할 수 있는 일:
  - 구현 세부사항:
  - 범위 안의 로컬 리팩터:
  - 증거 수집:
- 필요한 사용자 판단:
  - 제품 판단:
  - 기술 판단:
  - 민감 동작 승인:
  - 최종 수락:
  - 잔여 위험 수락:
  - 공개 인터페이스 또는 호환성 약속:
- AFK 중단 조건:
  - 경계 초과:
  - 증거를 만들 수 없음:
  - 닫기 관련 위험 발견:
- 끝에서 끝까지(end-to-end) 경로:
  - 트리거 / 입력:
  - 도메인 로직:
  - 영속화:
  - API / 호출자 경계:
  - UI / 관찰 가능한 출력:
- 허용 경로:
  - `src/...`
  - `tests/...`
- 허용 도구:
  - read
  - edit
  - shell: `npm test -- ...`
- 확인 프로필:
  - changed_paths
  - approval_scope
  - evidence_sufficiency
- ValidatorResult IDs:
  - vertical_slice_shape
  - shared_design_alignment
  - decision_quality_check
  - autonomy_boundary_check
  - feedback_loop_check
  - tdd_trace_required
  - domain_language_consistency
  - module_interface_review
  - codebase_stewardship_check
  - residual_risk_visibility_check
  - manual_qa_required
- 민감 범주:
  - none
- TDD:
  - trace 상태: required | recorded | waived | not_required
  - 요구/출처:
  - RED 대상 / 계획:
  - RED 증거(실제):
  - GREEN 증거:
  - 비 TDD(Non-TDD) 증거:
- 수동 QA:
  - required: yes | no
  - profile: ui_quality | workflow | copy | accessibility | browser_smoke | none
- 의존성:
  - 차단 원인(`blocked_by`):
  - 해소 대상(`unblocks`):
  - 병렬 가능 항목(`parallelizable_with`):
  - 병합 위험:
- 완료 조건:
  - [ ]
- 평가자(evaluator) 초점:
  - 항목:
````

## 메모

`TASK`의 스튜어드십 영향은 owner 기록, 검증기 결과, 참조에서 파생되는 `StewardshipImpactSummary` 표시입니다. 도메인 언어(Domain Language), 모듈 맵(Module Map), 인터페이스 계약(Interface Contract), 피드백 루프(Feedback Loop), TDD 트레이스(TDD Trace), 잔여 위험(Residual Risk), 사용자 판단(User Judgment) owner 기록을 대체하지 않습니다.

`TASK`의 구현 마이크로 계획(Implementation Micro-Plan)은 현재 Task와 작업 조각(Change Unit) 상태에서 생성되거나 그 상태와 정렬된 가벼운 실행 보조 정보입니다. [상태 보기와 템플릿 참조(Projection And Templates Reference)](../../projection-and-templates.md#projection-principles)의 상태 보기/보고서(projection/report) 경계 안에 머물며, `prepare_write`나 owner 상태 변경을 대체하지 않습니다.

`TASK`의 검토 단계(Review Stages)는 역할 렌즈(Role Lens), playbook, 2단계 검토 지침(two-stage review guidance)을 위한 관리되는 표시 섹션입니다. 정확한 권한 없음 규칙은 [설계 품질 정책(Design Quality Policies)](../../design-quality-policies.md#two-stage-review-display)과 [에이전트 통합(Agent Integration)](../../agent-integration.md#role-lens-동작)이 담당합니다. 기준 기록, `ProjectionKind` value, 민감 동작 승인, 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 수락, 닫기, 쓰기 승인 기록(Write Authorization)을 만들지 않으며, 발견 사항은 기존 owner 경로로 연결해야 합니다.

생성된 요약은 사용자가 읽기 쉬운 평범한 말을 먼저 쓰고, 정확한 Harness 용어는 유용한 라벨이나 참조로 붙입니다. 상태 보기가 명령어처럼 보이거나 표시 문구만으로 상태가 만들어진 것처럼 암시하면 안 됩니다.

관문 그룹 요약은 읽는 사람이 원시 관문 상세(gate detail)보다 실제 차단 사유 이야기를 먼저 보도록 첫 관리 섹션으로 둡니다. 범위, 사용자 판단, 증거, 닫기 준비 상태는 기존 owner 기록, 관문, 차단 사유, 참조에서 파생되는 표시 그룹입니다. 기준 필드(field), 정확한 관문 값(gate value)의 별칭(alias), 새 관문(gate), 재계산 입력(recompute input), 닫기 의미(close semantics), 권한 경로(authority path)가 아닙니다. 사용자 판단은 구조화되어 있으며 하나의 넓은 판단 또는 승인 묶음처럼 렌더링하면 안 됩니다. 정확한 관문(gate) 값과 재계산 규칙(recompute rule)은 [Core Model 참조](../../core-model.md#gates)가 담당하고, 닫기 동작은 [`close_task`](../../core-model.md#close_task)가 담당합니다.

`TASK`의 사용자 판단(User Judgment) 표시는 기준 schema field와 렌더링 라벨을 분리해야 합니다. `judgment_kind`은 내부 판단 유형이고, `presentation`은 compact 또는 full 표시 깊이를 제어합니다. Friendly label은 `judgment_kind`와 locale에서 파생합니다. 지원하는 `judgment_kind` 값은 `product_decision`, `technical_decision`, `scope_decision`, `sensitive_approval`, `qa_waiver`, `verification_risk_acceptance`, `final_acceptance`, `residual_risk_acceptance`, `cancellation`입니다. 한국어 표시는 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단을 사용합니다. 판단이 여러 영역에 걸쳐 있으면 라벨을 배타적으로 다루지 말고 부차적인 고려사항을 장단점, 영향받는 관문, 위험, 증거, 후속 조치에 렌더링해야 합니다. `judgment_category`, `judgment_route`, `display_depth` 또는 canonical state에서 쓰는 `display_label` 같은 레거시 필드는 마이그레이션 메모나 호환성 세부 보기에서만 나타날 수 있습니다. 이런 필드는 새 payload branch selector, gate, status value, gate recompute input, close aggregation rule, authority path, `judgment_kind`의 대체물이 아닙니다. 표시용 라벨은 validator input이 아니며 민감 동작 승인, 최종 수락, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락, 닫기, 쓰기 승인 기록(Write Authorization)의 owner contract를 흐리면 안 됩니다.

대기 중인 사용자 판단은 한 줄로 합치면 안 됩니다. 민감 동작 승인, 최종 수락, 잔여 위험 수락이 모두 대기 중이면 세 가지 라벨로 세 항목을 렌더링합니다. 민감 동작 승인 카드(Approval Card)는 최종 수락처럼 보이면 안 되고, 잔여 위험 수락은 수락하는 위험을 이름 붙여야 합니다.

`TASK`의 권한 주장은 출처 참조 또는 명시적 부재로 해소되어야 합니다. 제품 쓰기 호환성 주장은 compatible하게 소비된 쓰기 승인 기록(Write Authorization) 참조를 가리킵니다. Attempted invalid authorization ref는 violation/audit 또는 validator-finding context로만 나타날 수 있습니다. 민감 동작 승인은 최소 MVP-1에서는 `judgment_kind=sensitive_approval`인 해소된 `user_judgment`를 가리키고, 나중의 민감 동작 승인(Approval) 프로필이 활성화된 경우에만 Approval 참조를 가리킵니다. 최소 MVP-1 증거 표시는 있을 때 `evidence_ref`, 실행(Run) 참조, ArtifactRef 참조, 보이는 공백 요약을 가리킵니다. 활성 담당 경로가 전체 증거 충분성을 세울 수 없으면 충분성을 주장하지 않아야 합니다. 전체 기준-증거 충분성은 증거 목록(Evidence Manifest) 프로필이 활성화된 경우에만 증거 목록 참조를 가리킵니다. 분리 검증은 해당 프로필이 활성화된 경우에만 Eval(분리 검증 결과) 참조를 가리킵니다. 수동 QA는 해당 프로필이 활성화된 경우에만 수동 QA 기록 또는 유효한 면제 참조를 가리킵니다. 최종 수락은 최종 수락 사용자 판단 경로를 가리킵니다. MVP-1에서 잔여 위험 표시는 차단 사유/사용자 판단 참조 또는 `ResidualRiskSummary.status=none`을 가리키고, 풍부한 Residual Risk 참조는 해당 프로필이 활성화된 경우에만 가리킵니다. MVP-1에서 잔여 위험 수락은 잔여 위험 수락 사용자 판단과 관련 차단 사유/증거 참조를 가리키고, 수락된 Residual Risk 참조는 해당 나중 프로필이 활성화된 경우에만 가리킵니다. 참조가 없으면 완료된 권한이 아니라 빠진 뒷받침으로 렌더링해야 합니다.

잔여 위험 표시는 `status=none`과 `not_visible`을 구분해야 합니다. `status=none`은 요청된 행동에 대해 알려진 닫기 관련 잔여 위험이 없다는 뜻입니다. `not_visible`은 알려진 닫기 관련 위험이 있지만 최종 수락 또는 닫기에 충분히 보이지 않았다는 뜻이므로, 위험과 참조가 보일 때까지 차단 사유 또는 다음 행동으로 남아야 합니다.

`TASK`의 닫기와 보증 표시는 자체 확인된 작업, `detached_verified`, `verification_gate=waived_by_user`, QA 면제 판단, 잔여 위험 수락 닫기를 눈에 보이게 분리해야 합니다. 잔여 위험 수락 닫기는 MVP-1에서는 잔여 위험 수락 사용자 판단과 관련 차단 사유/증거 참조를 가리키고, 수락된 Residual Risk 참조는 해당 나중 프로필이 활성화된 경우에만 가리켜야 합니다. 검증을 면제한 표시는 `verification_gate=waived_by_user`와 필요한 경우 검증 위험 수락 사용자 판단을, QA 면제 판단은 `qa_gate=waived`, 수동 QA 기록 또는 면제 사유, 필요한 경우 QA 면제 사용자 판단을 가리켜야 합니다.

`TASK`의 면제 표시는 요약일 뿐입니다. 닫기에 영향을 주는 QA 면제 판단이나 `verification_gate=waived_by_user` 상태는 그 경로를 유효하게 만드는 기존 기록을 가리켜야 합니다. QA 면제 판단은 `manual_qa_records`/`qa_gate=waived`와 필요한 경우 QA 면제 사용자 판단을, `verification_gate=waived_by_user` 상태는 필요한 경우 검증 위험 수락 사용자 판단을 가리킵니다. 정책 또는 관문, Task와 작업 조각(Change Unit), 생략한 확인이나 대상, 사유, 행위자, 필요할 때 만료 또는 잔여 위험 후속 작업, 관련 참조, 닫기 영향, 그리고 필요할 때 잔여 위험 경로로 보여주거나 수락해야 하는 닫기 관련 잔여 위험도 함께 보여줘야 합니다. QA 면제 판단은 수동 QA가 되지 않고, 검증 위험 수락은 분리 검증을 만들지 않습니다.

`TASK`의 닫기 요약은 진행 중이거나 최근 닫힌 `work` Task를 위한 이어가기 표시 요약입니다. 관문(Gate) 상태나 잔여 위험을 숨기면 안 됩니다. 닫기가 성공했거나, 막혔거나, 취소됐거나, 잔여 위험 수락으로 닫혔을 때 변경된 범위, 민감 동작 승인, 증거, 검증, 수동 QA, 잔여 위험 표시, 잔여 위험 수락, 최종 수락, 면제 판단 상태, 닫기 이유, 잔여 위험 후속 작업을 해당되는 만큼 보여주고 owner 기록으로 돌아가는 참조를 포함해야 합니다. 민감 동작 승인, 최종 수락, 잔여 위험 수락은 반드시 별도 줄로 유지합니다. 민감 동작 승인은 이름 붙은 민감 동작에 대한 승인이고, 최종 수락은 사용자의 결과 판단이며, 잔여 위험 수락은 수락한 위험을 이름 붙이고 MVP-1에서는 잔여 위험 수락 사용자 판단과 관련 차단 사유/증거 참조를 인용해야 합니다. 수락된 Residual Risk 참조는 해당 나중 프로필이 활성화된 경우에만 인용합니다.

닫기 요약은 민감 동작 승인, 증거, 검증, 수동 QA, 최종 수락, 잔여 위험 표시, 잔여 위험 수락을 하나의 "완료" 표시로 합치면 안 됩니다. 테스트가 통과했지만 민감 동작 승인, 수동 QA, 최종 수락, 잔여 위험 수락이 대기 중이면 닫기 표시는 정확히 그 범주를 차단 사유로 보여줘야 합니다.

직접 작업은 `DIRECT-RESULT`에서 가벼운 닫기 영향 요약을 보여주고, 이어가기 카드(Journey Card) 닫기 맥락은 간결한 상태/이어가기 표시입니다. `TASK` 닫기 요약은 [projection/report 경계](../../projection-and-templates.md#projection-principles) 안의 이어가기 표시이며, 닫기와 관문 영향은 여전히 owner 기록에서 옵니다.

`TASK`, 이어가기/증거/보고서 섹션(Journey/evidence/report section)에 표시되는 아티팩트 참조(artifact ref)는 `redaction_state`를 보존해야 합니다. `secret_omitted` ref는 보이는 비밀 정보가 아닌 증거만 뒷받침할 수 있고, `blocked` ref는 원본 내용이 아니라 커밋된 메타데이터 전용 알림(metadata-only notice)과 사용할 수 없는 입력을 보여줍니다.
