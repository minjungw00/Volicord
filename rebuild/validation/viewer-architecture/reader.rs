// Disposable, read-only comparison server. Generated inputs/assets live in .local.
use std::{
    env, fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
include!("data.rs");
const STYLE: &str = include_str!("reader.css");
const SCRIPT: &str = include_str!("app.js");
const DATA: &str = include_str!("data.json");
struct Unit {
    role: &'static str,
    text: &'static str,
    refs: &'static [usize],
}
struct Source {
    label: &'static str,
    text: &'static str,
    start: usize,
    comparison: &'static str,
}
struct Work {
    key: &'static str,
    goal: &'static str,
    status: &'static str,
    audit: &'static str,
    units: &'static [Unit],
    sources: &'static [Source],
}
fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn label(role: &str) -> &str {
    match role {
        "purpose" => "문제",
        "reported_change" => "변경 / 조사",
        "reading_point" => "작동 원리",
        "observation" => "관측과 범위",
        "comparison" => "조건별 결과",
        "next_step" => "다음 행동",
        "limits" => "남은 한계",
        _ => "설명",
    }
}
fn render(key: Option<&str>, offline: bool, failure: bool) -> String {
    render_set(WORKS, key, offline, failure)
}
fn render_set(works: &[Work], key: Option<&str>, offline: bool, failure: bool) -> String {
    let mut s = format!("<!doctype html><html lang=ko><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>Viewer architecture reading comparison</title><style>{STYLE}</style><body class=\"{}\"><header><nav><a href=\"/\">Work 목록</a><a href=\"/?work=b\">Work B</a><a href=\"/?work=click\">Click 조사</a></nav><p>작성형 비교 자료 · 과거 실행과 현재 상태 구분 · 사용자 검토 대기</p></header><main>", if offline {"offline"} else {""});
    if let Some(w) = works.iter().find(|w| Some(w.key) == key) {
        s += &format!(
            "<p class=eyebrow>{} · {}</p><h1>{}</h1><div class=layout><article class=narrative>",
            esc(w.key),
            esc(w.status),
            esc(w.units[0].text)
        );
        for (i, u) in w.units.iter().enumerate().skip(1) {
            s += &format!(
                "<section class=\"unit {}\" id=claim-{i} tabindex=-1>",
                esc(u.role)
            );
            if u.role != "synopsis" {
                s += &format!("<h2>{}</h2>", label(u.role));
            }
            s += &format!(
                "<p class=copy data-unit={i}>{}</p><div class=links>",
                esc(u.text)
            );
            for n in u.refs {
                s += &format!("<a href=\"#source-{n}\" data-source=\"source-{n}\" data-claim=\"claim-{i}\">{}</a>",esc(w.sources[*n].label));
            }
            s += "</div></section>";
        }
        s += &format!("<details><summary>원래 지시와 control Goal</summary><p class=goal>{}</p></details><details id=audit><summary>출처·상태·출력 identity와 전체 진단</summary><pre>{}</pre></details></article><aside class=code-pane aria-label=\"설명에 연결된 Source\"><p class=legend>초록색: 설명이 가리키는 보존 범위 · diff 색상 아님</p><p class=selected-claim></p><a class=return href=\"#claim-1\">설명으로 돌아가기</a>",esc(w.goal),esc(w.audit));
        for (i, c) in w.sources.iter().enumerate() {
            s += &format!("<section class=source id=source-{i} tabindex=-1><h2 class=source-label>{}</h2><p class=legend>{}</p>",esc(c.label),esc(c.comparison));
            if failure {
                s += "<p class=unavailable>Source unavailable — 원본 본문을 읽을 수 없습니다. 설명의 identity와 한계를 확인하세요.</p>";
            } else {
                s += "<pre><code>";
                for (n, line) in c.text.split_inclusive('\n').enumerate() {
                    s += &format!(
                        "<span class=\"line explained\"><span class=number>{}</span>{}</span>",
                        c.start + n,
                        esc(line.trim_end_matches('\n'))
                    );
                }
                s += "</code></pre>";
            }
            if offline {
                for (n, u) in w.units.iter().enumerate().skip(1) {
                    if u.refs.contains(&i) {
                        s += &format!("<p><a data-return=\"claim-{n}\" href=\"#claim-{n}\">{} · {} {n}</a></p>","원래 문단으로",label(u.role));
                    }
                }
            }
            s += "</section>";
        }
        s += "</aside></div>";
    } else if key.is_some() {
        s +=
            "<h1>Work unavailable</h1><p class=unavailable>요청한 Work가 보존 입력에 없습니다.</p>";
    } else {
        s += "<p class=eyebrow>출처가 보존된 두 작업</p><h1>무엇을 했고, 코드는 어떻게 동작하나요?</h1>";
        for w in works {
            s+= &format!("<article class=work-card><h2><a data-work=\"{}\" href=\"/?work={}\">{}</a></h2><p>{}</p><p>{}</p></article>",w.key,w.key,esc(w.units[0].text),esc(w.units[1].text),esc(w.status));
        }
    }
    s += "<footer>Authored evaluation · Source readback, no new generation or acceptance</footer></main>";
    if !offline {
        s += "<script src=/app.js defer></script>";
    }
    s += "</body></html>";
    s
}
fn vue_shell() -> String {
    format!("<!doctype html><html lang=ko><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>Viewer architecture reading comparison</title><style>{STYLE}</style><div id=app></div><noscript>JavaScript가 없으면 offline.html을 여세요.</noscript><script src=/app.js defer></script></html>")
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = env::args().nth(1) {
        let key = env::args().nth(2).unwrap_or("b".into());
        fs::write(path, render(Some(&key), true, false))?;
        return Ok(());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let authority = listener.local_addr()?.to_string();
    println!("http://{authority}/");
    std::io::stdout().flush()?;
    for stream in listener.incoming() {
        // Accept errors are listener failures; read/parse/write failures belong
        // to one client and must never end a user-launched preview.
        let mut stream = stream?;
        if let Err(error) = serve_connection(&mut stream, &authority) {
            let _ = writeln!(io::stderr().lock(), "preview connection failed: {error}");
        }
    }
    Ok(())
}

fn serve_connection(stream: &mut TcpStream, authority: &str) -> io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = match read_headers(stream) {
        Ok(request) => request,
        Err(code) => return respond(stream, code, "text/plain", "invalid or incomplete request"),
    };
    let target = match request_target(&request, authority) {
        Ok(target) => target,
        Err(code) => return respond(stream, code, "text/plain", "read-only same-host evaluation"),
    };
    let (status, kind, body) = if target == "/app.js" {
        ("200 OK", "text/javascript", SCRIPT.to_owned())
    } else if target == "/data.json" {
        ("200 OK", "application/json", DATA.to_owned())
    } else if target.starts_with("/?") || target == "/" {
        let key = target
            .split('?')
            .nth(1)
            .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("work=")));
        (
            "200 OK",
            "text/html",
            if cfg!(vue) {
                vue_shell()
            } else {
                render(key, false, target.contains("failure=1"))
            },
        )
    } else {
        ("404 Not Found", "text/plain", "unavailable".to_owned())
    };
    respond(stream, status, kind, &body)
}

fn read_headers(stream: &mut TcpStream) -> Result<String, &'static str> {
    const LIMIT: usize = 16384;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("408 Request Timeout");
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| "400 Bad Request")?;
        let mut chunk = [0; 1024];
        let n = match stream.read(&mut chunk) {
            Ok(0) => return Err("400 Bad Request"),
            Ok(n) => n,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err("408 Request Timeout")
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("400 Bad Request"),
        };
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > LIMIT {
            return Err("431 Request Header Fields Too Large");
        }
        if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            if end + 4 != bytes.len() {
                return Err("400 Bad Request");
            }
            return String::from_utf8(bytes).map_err(|_| "400 Bad Request");
        }
    }
}

fn request_target<'a>(request: &'a str, authority: &str) -> Result<&'a str, &'static str> {
    let mut lines = request.split("\r\n");
    let parts: Vec<_> = lines.next().unwrap_or("").split(' ').collect();
    if parts.len() != 3
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].starts_with("//")
        || parts[1].bytes().any(|c| c <= 32 || c == 127)
    {
        return Err("400 Bad Request");
    }
    let mut host = None;
    for line in lines.take_while(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("400 Bad Request")?;
        if name.is_empty()
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || value.bytes().any(|c| c < 32 || c == 127)
        {
            return Err("400 Bad Request");
        }
        if name.eq_ignore_ascii_case("host") {
            if host.replace(value.trim()).is_some() {
                return Err("400 Bad Request");
            }
        }
        if name.eq_ignore_ascii_case("transfer-encoding")
            || (name.eq_ignore_ascii_case("content-length") && value.trim() != "0")
        {
            return Err("400 Bad Request");
        }
    }
    if parts[0] != "GET" || host != Some(authority) {
        return Err("403 Forbidden");
    }
    Ok(parts[1])
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &str) -> io::Result<()> {
    let head=format!("HTTP/1.1 {status}\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",body.len());
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()))
}

#[test]
fn hostile_text_is_escaped_by_the_actual_renderer() {
    const ATTACK: &str = "</script><script>window.__attack=1</script><img src=https://example.invalid onerror='window.__attack=1'> & \" '";
    let work = Work {
        key: "b",
        goal: Box::leak(ATTACK.repeat(1000).into_boxed_str()),
        status: ATTACK,
        audit: ATTACK,
        units: &[
            Unit {
                role: "heading",
                text: ATTACK,
                refs: &[],
            },
            Unit {
                role: "synopsis",
                text: ATTACK,
                refs: &[0],
            },
        ],
        sources: &[Source {
            label: ATTACK,
            text: ATTACK,
            start: 1,
            comparison: "missing_before_state",
        }],
    };
    let page = render_set(&[work], Some("b"), true, false);
    assert!(!page.contains("<script"));
    assert!(!page.contains("<img"));
    assert!(page.contains("&lt;script&gt;window.__attack=1&lt;/script&gt;"));
    if let Ok(output) = env::var("VIEWER_SECURITY_HTML") {
        fs::write(output, page).unwrap();
    }
}
