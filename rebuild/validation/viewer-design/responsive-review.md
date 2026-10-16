# Viewer 피드백 반영: 근거 패널·모바일 코드·검토 분리

2026-10-16, 시작 HEAD `ebcfc118`, clean. 사용자 요청은 desktop 근거 sidebar,
더 작은 mobile 코드, Viewer와 디자인 검토의 분리였다. 직전 전달물에 이어진
대화의 피드백이며 별도 screenshot/manifest 응답은 없었다. 재현한 직전 패키지는
`viewer-journeys/review/manifest.json` SHA-256
`f87c90d76836455a6693e1eef7e93b55c7fa2426d906b5ec7cb8fea9dda2bb8d`이다.
이 구체적인 디자인 방향을 적용했으며 전체 UX 승인이나 H1/H2/H3 응답으로
확대하지 않는다. 이전 패키지·평가·보고는 그대로 보존했다.

## 수정과 관찰

| 요청 / 이전 동작 | 수정 | 확인 |
| --- | --- | --- |
| Desktop도 modal을 열어 본문이 사라짐 | 900px 초과에서는 오른쪽 nonmodal 근거 패널. 본문을 유지하고 선택한 문단을 보이는 위치로 맞춤 | 1440/1024px에서 본문·코드 동시 표시, 배경 inert/modal/focus trap 없음. 본문에서 다른 근거를 선택하고 원래 문단으로 복귀 |
| Mobile 15px 코드의 가로 넘침 | 601–900px에서는 13px, 600px 이하에서는 12px. 줄 번호 여백도 줄임. 원문·들여쓰기·좌표는 유지 | 이전 패키지와 같은 Source 텍스트를 직접 비교. 390/320px에서 코드 전체 가로 폭 약 20% 감소 |
| Viewer 메뉴·배지·푸터·근거에 검토 링크가 섞임 | `viewer/`에 Viewer와 필요한 Source/script만 둠. 검토용 메뉴·배지·푸터·manifest/baseline 링크 제거. Preview 기본 URL도 Viewer를 엶 | Viewer의 모든 활성 링크는 subtree 안에서 닫힘. 의도적으로 추가한 `../index.html` 검토 링크는 거부. Viewer만 복사해도 독립 실행 |

390px에서 B의 코드 전체 scrollWidth는 2905→2308px, Click은 843→670px였다.
영역 폭도 358→366px로 늘었다. 긴 줄에는 여전히 가로 스크롤이 필요하다.
이는 browser geometry이며 실제 사용자 불편 감소나 독해 속도 관측은 아니다.
Desktop은 15px를 유지한다. Agent가 desktop/mobile 캡처를 각각 확인했다.

모바일은 전체 화면 dialog와 focus containment를 유지한다. Desktop에서는
Tab으로 본문과 근거를 자유롭게 이동한다. 열어 둔 Source는 1440→390→1024px
전환에도 유지되고 Escape로 같은 문단에 돌아간다. Desktop 패널에서 새 문단의
근거를 선택한 경우 split layout의 scroll 좌표를 일반 문서 좌표로 오인하지 않는다.

20개 원래 prose unit, 14개 fact, 전체 Goal/audit, 13개 Source range는 보존했다.
제거한 것은 검토용 UI 안내/링크다. 원래 기록이나 Goal에 포함된 fixture 설명,
historical identity·실행·실패·검토·수락 상태는 바꾸지 않았다. B/Click과 설명 없는
Goal-only control의 의미·상태도 동일하다. 별도 검토에는 baseline·이전 후보·측정이
계속 남지만 Viewer가 미구현 목적지를 구현한 것처럼 표시하지 않는다.

## 전달물과 상태

Viewer: `rebuild/.local/viewer-responsive/review/viewer/index.html`.
Viewer-only ZIP: `rebuild/.local/viewer-responsive/viewer.zip` (31 files,
445,241 bytes). 압축을 풀고 `viewer/index.html`을 연다. 별도 디자인 검토는
`rebuild/.local/viewer-responsive/review/index.html`이다.
Full package에서 `python3 serve.py`를 실행하면 기본 URL은 Viewer,
`/index.html`은 검토 자료다. 종료는 Ctrl-C. Viewer는 review 파일 없이도 읽을 수 있다.

| Identity | SHA-256 |
| --- | --- |
| Final package manifest | `48521e9ef889b678575a5976eeba27066126ff351b9c574039970585fdac35a9` |
| Tested candidate manifest | `2d4012d50a4e6f06b1eb52160df651511827061cd6930e2d4c61d1ba0587c250` |
| Browser result | `8c0a0fd4052d96f871ef96c9c744a4a3c29d7b2b99aa30ba873b01c60e2ece81` |
| Journey measurements | `bd4c941b6d2e62264f34c4cb3b39046f5859869cc832cce7aa7f17a5475fbbf0` |
| Viewer-only ZIP | `8e909722d341bfdfabaf40575d16507b0ee388079821c6fb8e54cb16f1828c50` |

Implementation support commit: `661b54ac4ec0e143a43085307c3b12fc7cb6f200`.
Only disposable validation support changed; Product/legacy crates, schema, provider
authority and polyglot capability are unchanged. Generated files, raw Source,
screenshots and logs remain ignored. No framework selection or Product migration.
The [provisional composition](experience-requirements.md) reflects this feedback;
the active owner's opening-limit placement and broader adoption gate remain unchanged.

## 검증

다음은 `rebuild/.local/validation/` 아래의 실제 focused 결과이며 모두 exit 0,
termination/spawn error 없음이다. 완전한 stdout/stderr와 inner 결과를 확인했다.

| Run | 결과 |
| --- | --- |
| `20261016T114553.342366Z-viewer-responsive-committed-build-ma2mpq9n` | Clean commit에서 원본 입수·현재 Product source 동등성·분리된 artifact 생성 |
| `20261016T114553.631406Z-viewer-responsive-committed-controls-rvh3g7vw` | 21개 positive/adverse input·Source·상태·원문·링크·분리 검사 |
| `20261016T114555.287993Z-viewer-responsive-committed-browser-xf3h5x38` | 14 groups, 30 비교 경로, 132 citation 왕복. 1440/1024/768/390/320px, exact Source, desktop 본문 가시성·선택·복귀, mobile focus, resize, history, missing explanation/Source, hostile/long, offline |
| `20261016T114457.303999Z-viewer-responsive-listener-ydaww5yj` | 변경된 preview가 idle/partial/disconnect/dribble/rejection 뒤 같은 PID로 다음 GET 200 제공 |
| `20261016T114644.780557Z-viewer-responsive-review-package-4lt7mina` | 이전 후보와 현재 후보·근거·측정을 별도 보존하여 패키징 |
| `20261016T114644.976264Z-viewer-responsive-review-controls-78l89q5z` | 최종 패키지 21개 검사, 모든 file/fragment·hash 및 Viewer subtree closure |
| `20261016T114647.183571Z-viewer-responsive-review-browser-nto6wkjc` | Checkout 밖으로 이동 후 Python 실행; review 파일을 제외한 Viewer-only 복사에서 1440/390px Source 열기·복귀; 서버 종료·network off·script-free 320px 읽기 |
| `20261016T114707.853331Z-viewer-responsive-archive-u3w0b4ap` | Viewer-only ZIP의 모든 파일이 검사된 subtree와 byte 단위로 동일 |

Rust/workspace 전체 검사·Final/V11·provider/legacy 검사는 실행하지 않았다.
Rust/Product/dependency 변경이 없는 presentation support 범위이기 때문이다.
필요한 도구 누락은 없었다. Native zoom·screen reader·새 후보의 실제 사용자
독해/선호/전체 승인은 미관찰/pending이다. 다음 검토를 완료 조건으로 강요하거나
기술 검사 성공을 사람의 수락으로 바꾸지 않는다.
