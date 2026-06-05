# DEC 템플릿

## 사용 시점

특정 `user_judgment` 기록에 대해 독립형 전체 형식 판단 패킷(Decision Packet) 표시가 켜져 있을 때만 `DEC`를 사용합니다. 일반 MVP-1 경로는 상태, 다음 행동, 사용자 판단 리소스를 통한 간결한 판단 요청입니다. 작은 차단 사유 해소 질문은 한 화면에 들어가야 하며, 사용자가 세부 보기를 요청하지 않는 한 이 전체 템플릿을 노출하지 않습니다.

사용자에게 보이는 표시 라벨은 `judgment_kind`와 locale에서 파생하며, 한국어 렌더링은 다음 아홉 가지입니다.

- 제품 판단
- 기술 판단
- 범위 판단
- 민감 동작 승인
- QA 면제 판단
- 검증 위험 수락
- 최종 수락
- 잔여 위험 수락
- 취소 판단

경계: 상태 보기 템플릿(projection template)일 뿐이며 하네스 서버/런타임 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 단계와 상태 보기 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 선택적 전체 형식 판단 표시입니다. 독립형으로 저장되는 `DEC` Markdown 상태 보기는 독립형 판단 패킷(Decision Packet) 상태 보기 기능이 켜진 경우에만 선택적으로 사용합니다. "Decision Packet"은 표시 형식 라벨이고, `user_judgment`가 기준 기록 계열입니다.

## 기준 기록

- `state.sqlite.user_judgments`
- 관련 Task와 작업 조각(Change Unit) 참조
- `judgment_kind`, `presentation`, locale에서 파생한 표시 판단 라벨
- 관련 `decision_gate` 상태와 사용자 판단 이벤트
- `judgment_kind=sensitive_approval`의 `approval_scope`, 그리고 나중 민감 동작 승인(Approval) 프로필이 활성화된 경우에만 민감 동작 승인 기록
- 나중 프로필이 활성화된 경우 관련 reconcile 기록
- 잔여 위험 참조
- 최소 MVP-1의 증거 요약, 실행(Run) 참조, ArtifactRef 참조, 보이는 증거 공백. 전체 증거 목록(Evidence Manifest) 프로필이 활성화된 경우에만 증거 목록 참조
- 관련 권한 맥락으로 표시될 때 쓰기 승인 기록(Write Authorization), 민감 동작 승인, Eval(분리 검증 결과), 수동 QA, 최종 수락 맥락, 잔여 위험 참조, ArtifactRef 참조, `redaction_state`, 읽기용 보기 최신성(projection freshness)
- 영향받는 범위 표시 입력: 제품 영역, 화면/흐름, 모듈, 인터페이스, 경로, 수용 기준, 관문, 민감 범주
- 읽기용 보기 최신성(projection freshness) 입력

`decision_packet_id`, `judgment_category`, `judgment_route`, `display_depth`, canonical state에서 쓰는 `display_label` 같은 레거시 이름은 마이그레이션 메모 또는 호환성 세부 보기에서만 나타날 수 있습니다. 새 템플릿, 예시, fixture는 `user_judgment_id`, `judgment_kind`, `presentation`, locale에서 파생한 표시 라벨, `record_kind=user_judgment`를 사용해야 합니다.

민감 동작 승인 표시의 "포괄하는 것", "포괄하지 않는 것", "비밀 정보 노출 경계"는 `judgment_payload.approval_scope`, 관련 `user_judgment` 참조, 나중 프로필이 활성화된 경우에만 연결되는 민감 동작 승인(Approval) 기록, 현재 쓰기/닫기 맥락에서 파생한 표시용 요약입니다. 경계만 설명하며 별도 사용자 판단을 확정하거나 쓰기 승인 기록(Write Authorization)을 만들거나 최소 MVP-1에서 커밋된 민감 동작 승인(Approval) 기록을 암시하지 않습니다. 민감 동작 승인 표시는 최종 수락이나 잔여 위험 수락처럼 보여서는 안 됩니다.

해소된 사용자 판단이 민감 동작 승인을 부여하는 경우는 `judgment_kind=sensitive_approval`이고 호환되는 `approval_scope`를 가진 경우뿐입니다. 다른 사용자 판단 결과는 제품 판단, 기술 판단, 범위 판단, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단, 나중 프로필의 조정(reconcile) 선택을 확정할 수 있지만 민감 동작 승인을 부여하지 않습니다.

`presentation=short`는 간단한 차단 사유 해소 질문과 간결한 질문의 기본값입니다. `presentation=full`은 복잡하거나 위험이 크거나 닫기에 영향을 주거나 조정(reconcile)/나중 프로필 판단을 위한 전체 형식 판단 패킷(Decision Packet) 스타일 표시입니다. `presentation`은 렌더링되는 맥락 양만 바꾸며 권한을 바꾸지 않습니다.

## 렌더링 섹션

- 지금 필요한 이유
- 현재 상태
- 판단 유형과 표시 형식
- 해당되는 경우 민감 동작 승인 맥락
- 사용자가 판단하는 것
- 에이전트가 사용자 없이 결정해도 되는 것
- 해당되는 경우 자율성 경계 영향
- 영향받는 범위와 경계
- 선택지
- 추천
- 판단을 미룰 때의 영향
- 판단에 필요한 최소 맥락
- 사용자 판단
- 해당되는 경우 잔여 위험 수락
- 후속 조치
- 참조

충분한 렌더링 판단 요청(Decision Packet)은 하나의 사용자 소유 판단에 답합니다. 넓은 승인을 묻지 않습니다. 정확한 공개 요청/응답 필드는 [`harness.request_user_judgment`](../../api/mvp-api.md#harnessrequest_user_judgment)가 소유하고, 기준 권한 규칙은 [사용자 판단(User Judgment)](../../core-model.md#user-judgment)와 [Decision Gate](../../core-model.md#decision-gate)가 소유합니다. 이 템플릿은 기존 사용자 판단 필드를 요약할 수 있지만 schema 필드, 관문(gate), 대체 권한을 추가하면 안 됩니다.

사용자에게 보이는 질문은 판단을 직접 물어야 합니다. 선택지를 고를지, 명시된 결과를 감수하고 미룰지, 해당 경로를 거절할지, 민감 동작 승인을 허용/거절할지, QA 면제 판단을 기록할지, 검증 위험을 수락할지, 결과를 수락/거절할지, 이름 붙은 잔여 위험을 수락/거절할지, 취소 판단이나 나중 프로필 조정(reconcile) 결과를 기록할지처럼 기록될 값을 분명히 말합니다. "approve"나 "승인"은 민감 동작 승인 또는 나중 민감 동작 승인(Approval) 기록에만 씁니다. 여러 판단이 대기 중이면 별도 질문 또는 별도 줄로 렌더링합니다. 민감 동작 승인, 최종 수락, 잔여 위험 수락을 하나의 답변으로 합치면 안 됩니다.

**예시 단서:**

아래의 일반적인 전체 형식 사용자 판단 형태에는 같은 렌더링 섹션을 사용합니다. 이 단서들은 추가 템플릿 섹션이 아닙니다.

- 작은 차단 사유 해소 질문(`judgment_kind=product_decision`, `presentation=short`): 이미 범위가 정해진 설정 문구 변경 안에서 버튼 라벨을 "Save"로 할지 "Update"로 할지 고릅니다. 간결한 선택, 범위, 참조, 효과가 아닌 것을 `사용자가 판단하는 것`과 `참조`에 둡니다. 전체 아키텍처 장단점 비교 레이아웃을 강제하지 않습니다.
- 제품 판단(`judgment_kind=product_decision`): 로그인 실패 안내를 인라인 레이어, 토스트, 모달 중에서 고르거나 로그인 실패 문구를 일반형, 구체형, 혼합형 중에서 정합니다. 흐름, 방해 정도, 접근성, 문구, 제품 톤, 사용자 위험 차이는 `선택지`와 `추천`에 둡니다.
- 기술 판단(`judgment_kind=technical_decision`): 세션 쿠키, bearer/JWT 토큰, OAuth/OIDC 제공자, 소셜 로그인 제공자 통합 중에서 세션 모델을 고릅니다. 철회, CSRF/XSS 노출, 클라이언트 호환성, 구현 비용, ID 제공자 경계, 마이그레이션 영향은 `선택지`와 `판단에 필요한 최소 맥락`에 둡니다.
- 기술 판단(`judgment_kind=technical_decision`): 의존성 채택, 스키마/데이터 모델 마이그레이션, 공개 API/인터페이스 방향, 모듈 경계 변경, 개인정보/로깅 정책, QA 기대치, 검증 기대치, 나중 프로필이 활성화된 경우의 조정(reconcile) 선택을 다룹니다.
- 범위 판단(`judgment_kind=scope_decision`): 범위 확장, 비목표 제거, 작업 조각(Change Unit) 경계, 자율성 경계(Autonomy Boundary) 변경을 다룹니다. 정확한 범위와 그렇지 않은 효과를 `영향받는 범위와 경계`에 둡니다.
- QA 면제 판단(`judgment_kind=qa_waiver`): 생략되는 QA surface, policy 허용 여부, 사유, 닫기 영향, 위험/후속 작업을 보여줍니다. QA 면제 판단은 QA 증거나 통과한 QA 결과를 만들지 않습니다.
- 검증 위험 수락(`judgment_kind=verification_risk_acceptance`): 생략되거나 빠진 검증, 사용자가 수락하는 위험, 후속 작업, 닫기 영향을 보여줍니다. 분리 검증을 만들지 않습니다.
- 민감 동작 승인(`judgment_kind=sensitive_approval`): 의존성 설치, 비밀 정보 접근, 네트워크 쓰기, 파괴적 쓰기 또는 다른 범위 지정 민감 동작입니다. 민감 동작 승인 경계는 `민감 동작 승인 맥락`에 두고, 제품 판단이나 기술 판단을 해소한 것으로 취급하지 않습니다.
- 최종 수락(`judgment_kind=final_acceptance`): 최종 결과, 증거 상태, 수동 QA와 검증 상태, 닫기에 영향을 주는 잔여 위험 표시 상태를 `현재 상태`와 `판단에 필요한 최소 맥락`에 둡니다. 최종 수락을 새 민감 동작, 추가 쓰기, 배포, 병합을 승인하거나 잔여 위험 수락을 대신하는 판단처럼 취급하지 않습니다.
- 잔여 위험 수락(`judgment_kind=residual_risk_acceptance`): 보이는 한계, 기존 증거, 사용자에게 수락 여부를 묻는 위험 참조, 남은 후속 작업을 `현재 상태`, `판단에 필요한 최소 맥락`, `잔여 위험 수락`, `후속 조치`에 둡니다.
- 취소 판단(`judgment_kind=cancellation`): 무엇을 멈추는지, 무엇이 남는지, 후속 작업이 필요한지, 어떤 close reason 또는 state를 기록할지 보여줍니다.
- 넓은 "go ahead" 답변: 질문이 이 특정 판단 유형과 선택지를 묻는 이유를 보여줍니다. 일반적인 동의 표현은 이 질문이 그 정확한 판단을 기록하는 경우가 아니면 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단을 해소하지 않습니다.

**렌더링 예시: 최소 판단**

```text
판단 요청: 설정 라벨 문구
기록: user_judgment_id=UJ-0001
판단 유형: product_decision
표시 형식: short
표시 라벨(렌더링): 제품 판단
질문: 이 범위가 지정된 설정 라벨을 "Save"로 할까요, "Update"로 할까요?
범위/참조: CU-04의 설정 폼 문구; 출처 참조 TASK-012/CU-04; 민감 동작 또는 닫기 위험 참조 없음.
기록할 선택: Save | Update
결정하지 않는 것: 더 넓은 설정 흐름 동작, 현지화 전략, 최종 수락, 잔여 위험 수락, 쓰기 전 범위 확인이나 쓰기 승인 기록(Write Authorization).
```

**렌더링 예시: 민감 동작 승인**

```text
판단 요청: 의존성 설치 승인
기록: user_judgment_id=UJ-0002
판단 유형: sensitive_approval
표시 형식: short
표시 라벨(렌더링): 민감 동작 승인
질문: 이 Task에 대해 이름 붙은 의존성 설치/업데이트 동작을 승인하시겠습니까?
민감 동작 승인(Approval) 범위: 이름 붙은 설치 명령 또는 의존성 파일 업데이트, 이름 붙은 매니페스트/잠금 파일 경로, 현재 Task와 승인 유효 기간만 포함.
포괄하는 것: 범위가 지정된 민감 동작.
포괄하지 않는 것: 의존성이 올바른 아키텍처 방향인지, 향후 설치, 관련 없는 제품 파일 쓰기, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락.
별도 판단 필요: 의존성 선택 자체가 사용자 소유 기술 판단이면 `judgment_kind=technical_decision`를 사용합니다.
참조: 승인 범위 참조, `prepare_write` 후보 참조, 의존성 비교 참조, 사용 가능한 경우 영향받는 파일 참조.
```

**렌더링 예시: 전체 기술 장단점 비교**

```text
판단 요청: 로그인 세션 아키텍처
기록: user_judgment_id=UJ-0003
판단 유형: technical_decision
표시 형식: full
표시 라벨(렌더링): 기술 판단
질문: 이 로그인 작업은 어떤 세션 모델을 써야 합니까?
선택지: 서버 쪽 세션 쿠키, 클라이언트 보유 bearer/JWT 토큰, OAuth/OIDC 제공자와 로컬 세션 전략, 소셜 로그인 제공자 통합.
추천: 자사 웹 앱이면 현재 요구사항이 제3자 ID, 브라우저 밖 클라이언트, 소셜 로그인을 요구하지 않는 한 서버 쪽 세션 쿠키.
불확실성: 기존 세션 미들웨어, 철회 요구사항, SSO 요구사항, CSRF 자세, 마이그레이션 제약.
미룰 때의 영향: 저장소, 토큰 수명, 제공자, 미들웨어 동작을 확정하지 않는 읽기 전용 조사와 UI 골격 작업만 계속할 수 있습니다.
참조: 인증 모델 참조, 영향받는 수용 기준, 사용 가능한 경우 보안 증거 참조, 잔여 위험 또는 마이그레이션 참조.
```

## 전체 템플릿

````md
---
doc_type: user_judgment_decision_packet
projection_kind: DEC
projection_id: DEC-PROJ-0001
user_judgment_id: UJ-0001
task_id: TASK-0001
change_unit_id: CU-01
judgment_kind: product_decision
presentation: full
status: pending_user
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# UJ-0001 판단 요청 제목

> 상태 보기(Projection): `source_state_version`와 `updated_at` 기준으로 렌더링되며 상태의 `user_judgment_id`와 관련 참조를 표시합니다. 이 Markdown을 편집해도 판단은 해소되지 않습니다. 답변은 `harness.record_user_judgment`를 통해 기록합니다.

## 지금 필요한 이유
- 트리거:
- 차단 사유(blocker):
- 영향받는 작업:
- 현재 상태에서 진행할 수 없는 이유:

## 현재 상태
- Task 상태:
- 활성 작업 조각(Change Unit):
- 현재 관문:
- 최신 증거:
- 잔여 위험:
- 출처 참조: 판단={user_judgment_id}; 쓰기={write_authorization_ref|none}; 민감동작승인={user_judgment_ref|approval_ref_when_profile_active|none}; 증거={evidence_ref|evidence_manifest_ref_when_profile_active|none}; Eval={eval_ref|none}; 수동QA={manual_qa_ref|none}; 최종수락={final_acceptance_user_judgment_ref|none}; 잔여위험={residual_risk_refs|none}; 아티팩트={artifact_refs|none}; 가림={redaction_availability_summary|none}; 최신성={projection_freshness}

## 판단 유형과 표시 형식
- `judgment_kind`: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
- `presentation`: short | full
- 표시 라벨: 제품 판단 | 기술 판단 | 범위 판단 | 민감 동작 승인 | QA 면제 판단 | 검증 위험 수락 | 최종 수락 | 잔여 위험 수락 | 취소 판단, `judgment_kind`와 locale에서 파생
- 최종 기록 답변:
- 이 판단이 기록할 수 있는 것:
- 이 판단이 기록할 수 없는 것:
- 일반 동의 표현 처리:

## 해당되는 경우 민감 동작 승인 맥락
- 카드 라벨: 민감 동작 승인
- `judgment_kind=sensitive_approval` 범위:
- 연결된 민감 동작 승인 기록(나중 프로필에서만):
- 민감 범주:
- 이 민감 동작 승인이 포괄하는 것:
- 이 민감 동작 승인이 포괄하지 않는 것:
- 렌더링하면 안 되는 형태: 최종 수락 또는 잔여 위험 수락
- 여전히 필요한 별도 사용자 소유 판단:
- 민감 동작 승인 경계:
- 쓰기 승인 기록 경계:
- 비밀 정보 노출 경계:

## 사용자가 판단하는 것
- 판단 유형:
- 표시 라벨(렌더링):
- 사용자에게 보이는 질문:
- 결정:
- 이 결정이 확정하는 것:
- 이 결정이 확정하지 않는 것:
- 넓은 동의 표현이 부족한 이유:

## 에이전트가 사용자 없이 결정해도 되는 것
- 구현 세부사항:
- 허용된 범위 안의 코드 구조:
- 증거 수집:
- 후속 제안:

## 해당되는 경우 자율성 경계 영향
- 현재 경계 영향:
- 제안된 경계 업데이트:
- 필요한 사용자 판단:

## 영향받는 범위와 경계
- 범위 안:
- 범위 밖:
- 영향받는 제품 영역:
- 영향받는 화면 또는 흐름:
- 영향받는 모듈/인터페이스/경로:
- 영향받는 수용 기준:
- 영향받는 관문:
- 민감 범주:

## 선택지
### 선택지 A
- 선택:
- 장단점:
- 이점:
- 비용:
- 위험:
- 되돌릴 수 있음: reversible | partially_reversible | irreversible | unknown
- 신뢰도: low | medium | high

### 선택지 B
- 선택:
- 장단점:
- 이점:
- 비용:
- 위험:
- 되돌릴 수 있음: reversible | partially_reversible | irreversible | unknown
- 신뢰도: low | medium | high

## 추천
- 추천 선택지:
- 증거:
- 신뢰도:
- 추천이 바뀌는 조건:

## 판단을 미룰 때의 영향
- 계속할 수 있는 것:
- 계속 막히는 것:
- 닫기 영향:

## 판단에 필요한 최소 맥락
- 보이는 증거:
- 모르는 것:
- QA/검증 상태:
- 잔여 위험 표시 상태:
- 닫기 또는 쓰기 영향:

## 사용자 판단
- 선택한 항목:
- 기록 값(value): selected | rejected | deferred | granted | denied | expired | accepted
- 메모(note):
- 결정한 사람:
- 결정 시각:
- 넓은 동의 표현 확인: "proceed", "go ahead", "looks good", "좋아", "진행해"는 자동으로 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락, 취소 판단이 되지 않습니다.

## 해당되는 경우 잔여 위험 수락
- 이름 붙은 위험:
- 보이는 위험 참조:
- 수락 범위:
- 수락할 때의 영향:
- 후속 조치:

## 후속 조치
- 쓰기 전에 필요한 것:
- 닫기 전에 필요한 것:
- 제안된 후속 조치:

## 참조
- 작업(Task):
- 작업 조각(Change Unit):
- 사용자 판단:
- Write Authorization / 쓰기 전 범위 확인:
- 증거:
- 검증:
- 수동 QA:
- 잔여 위험:
- 아티팩트:
- 보기 최신성:
````

## 메모

이 템플릿은 렌더링 형태이지 기준 상태가 아닙니다. 활성 단계/프로필이 요구하는 사용자 판단 표시성은 상태 카드, 판단 요청, `status`/`next` 응답, 판단 맥락 리소스, 사용자 판단 리소스를 통해 제공될 수 있습니다. 독립형 `DEC` 상태 보기는 선택 사항입니다.

판단 패킷(Decision Packet) 상태 보기는 권한 맥락 참조를 간결하고 명시적으로 유지해야 합니다. 이 템플릿에 쓰기 승인 기록(Write Authorization), 민감 동작 승인 참조, 증거 요약, 해당 프로필이 활성화된 경우의 증거 목록(Evidence Manifest), Eval(분리 검증 결과), 수동 QA, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 아티팩트, `redaction_state`, 최신성 참조를 표시하더라도 문장이 그 기록의 권한이 되지는 않습니다.

판단 패킷(Decision Packet) 카드는 한 번에 하나의 판단 유형만 표시해야 합니다. 민감 동작 승인 질문은 승인 언어를 쓰고, 최종 수락 질문은 최종 수락 언어를 쓰며, 잔여 위험 수락 질문은 수락하는 구체적 위험을 이름 붙입니다.
