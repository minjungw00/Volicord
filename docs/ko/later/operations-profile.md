# 이후: 운영 프로필

이 문서는 운영, 복구, export, handoff 관련 향후 내용을 MVP 구현 경로에 섞지 않고 찾아가도록 돕습니다.

향후 하네스 동작을 위한 계획 및 길잡이 문서입니다. 상태: MVP-1 요구사항 아님, 구현된 런타임 아님. 이 저장소에서 런타임/서버 구현, 생성된 운영 파일, 실행 가능한 fixture, 런타임 데이터, 제품 코드를 허가하지 않습니다.

## 이런 때 읽기

- 보증 프로필 이후에 무엇이 속하는지 확인할 때.
- Operator, diagnostic, recovery, export, artifact integrity, projection refresh, handoff 담당 문서를 찾아야 할 때.
- 향후 운영 작업을 내부 엔지니어링 점검이나 MVP-1 사용자 작업 루프와 분리해야 할 때.

## 버킷 경계

운영 프로필은 MVP-1과 보증 프로필 이후 범위입니다. 첫 사용자 가치 루프가 증명된 뒤의 local operator와 handoff hardening을 다룹니다. 운영 surface를 초기 MVP 요구사항으로 만들지 않습니다.

| 운영 버킷 | 여기에 속하는 것 | 승격 전까지 밖에 둘 것 |
|---|---|---|
| Export | Task export bundle, artifact integrity manifest, redaction/omission note, retained 또는 unavailable artifact reporting, export non-leakage check. | Hosted sharing, import/sync workflow, deployment authority, 넓은 release automation은 로드맵 향후 후보입니다. |
| Recovery | Interrupted operation classification, compensating event, lock recovery, projection failure handling, artifact repair/replacement routing, manual recovery escalation. | Automated rollback, production recovery, remote fleet repair, external system recovery는 로드맵 향후 후보입니다. |
| Handoff | Release Handoff report/export profile, close-relevant summary, evidence/verification/QA/risk ref, deployment 권한 없는 external checklist guidance. | Merge, deploy, canary, rollback, production monitoring automation은 로드맵 향후 후보입니다. |
| Operator readiness | Local project/runtime registration health, MCP availability, surface capability posture, artifact-store health, 다음 안전한 operator action. | Hosted operator console, team permission, remote/shared operations, connector marketplace는 로드맵 향후 후보입니다. |
| Doctor/readiness surfaces | Full `doctor` category set, readiness level, security posture diagnostic, projection freshness check, reconcile visibility, docs-maintenance report exposure. | Dashboard, analytics, long-term metrics, diagnostic을 권한처럼 취급하는 automation은 로드맵 향후 후보입니다. |

Projection refresh, reconcile, artifact check, conformance run entrypoint는 owner Reference 문서가 동작을 정의할 때 운영 프로필에 속합니다. 그래도 파생, 진단, repair surface이며 두 번째 state model을 만들지 않습니다.

## 읽는 경로

먼저 [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md)에서 단계 경계를 확인합니다. 그다음 필요한 질문의 담당 문서만 엽니다.

| 필요한 것 | 담당 문서 |
|---|---|
| Operator command, diagnostic, recover, reconcile, export, artifact check, conformance run entrypoint | [운영과 Conformance 참조](../reference/operations-and-conformance.md) |
| Runtime layout, artifact storage, lock, migration, projection job, validator storage | [Storage 참조](../reference/storage.md) |
| Security posture, trust boundary, threat category, control, guarantee wording | [보안 참조](../reference/security.md) |
| Runtime space, Core placement, transaction order, projection/reconcile placement, recovery overview | [런타임 아키텍처 참조](../reference/runtime-architecture.md) |
| Projection freshness와 rendered output 경계 | [Projection과 Template 참조](../reference/projection-and-templates.md)와 [Template 참조](../reference/templates/README.md) |
| 운영 fixture mechanics와 향후 운영 scenario | [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 [향후 Fixtures](future-fixtures.md) |

## 경계

운영 프로필은 export, recovery, handoff, operator readiness, doctor/readiness surface를 정리하는 곳입니다. Dashboard, hosted workflow, team workflow, connector marketplace, deployment automation, 넓은 orchestration profile이 아닙니다.

이 프로필은 Runtime Home을 tamper-proof로 만들거나, 읽기용 요약을 운영 기준으로 만들거나, hosted dashboard를 만들지 않습니다. 또한 승격된 owner 경로가 정확한 mechanism을 증명하기 전에는 OS-level sandboxing, arbitrary-tool permission control, preventive blocking, isolation을 제공한다고 주장하지 않습니다.

여기에 이름이 있다고 해서 MVP-1 요구사항이 되거나, 구현된 런타임 동작이 되거나, 실행 가능한 conformance가 되지는 않습니다. 향후 operations fixture row는 owner가 정확한 동작을 승격하고 exact-shape fixture를 materialize하기 전까지 [향후 Fixtures](future-fixtures.md)에 남습니다.
