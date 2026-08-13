use std::{env, ffi::OsString, io::Write, net::TcpListener, path::PathBuf};
use volicord_context::ProjectId;
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_viewer::{ExplanationLevel, ViewerAdapter, ViewerLocale, ViewerRequest};

fn main() {
    if let Err(error) = run(env::args_os().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<OsString>) -> Result<(), String> {
    let mut runtime = None;
    let mut project = None;
    let mut bind = "127.0.0.1:3219".to_owned();
    let mut locale = ViewerLocale::English;
    let mut level = ExplanationLevel::Working;
    let mut language = "en".to_owned();
    let mut index = 0;
    while index < args.len() {
        let argument = args[index].to_string_lossy();
        let next = |position: usize| {
            args.get(position)
                .map(|value| value.to_string_lossy().into_owned())
                .ok_or_else(|| format!("missing value after {argument}"))
        };
        match argument.as_ref() {
            "--runtime" => runtime = Some(PathBuf::from(next(index + 1)?)),
            "--project" => project = Some(parse_project(&next(index + 1)?)?),
            "--bind" => bind = next(index + 1)?,
            "--locale" => {
                locale = match next(index + 1)?.as_str() {
                    "ko" => ViewerLocale::Korean,
                    "en" => ViewerLocale::English,
                    other => return Err(format!("unsupported fixed locale: {other}")),
                }
            }
            "--level" => {
                level = match next(index + 1)?.as_str() {
                    "overview" => ExplanationLevel::Overview,
                    "working" => ExplanationLevel::Working,
                    "deep" => ExplanationLevel::Deep,
                    other => return Err(format!("unknown explanation level: {other}")),
                }
            }
            "--language" => language = next(index + 1)?,
            _ => return Err(format!("unknown argument: {argument}")),
        }
        index += 2;
    }
    if !bind.starts_with("127.0.0.1:") && !bind.starts_with("[::1]:") {
        return Err("the local viewer binds only to a loopback address".into());
    }
    let project = project.ok_or_else(|| "--project is required".to_owned())?;
    let layout = match runtime {
        Some(path) => RuntimeLayout::new(path).map_err(|error| error.to_string())?,
        None => RuntimeLayout::from_environment().map_err(|error| error.to_string())?,
    };
    let adapter = ViewerAdapter::new(LocalOperations::new(layout));
    let page = adapter
        .render(&ViewerRequest {
            project_id: project,
            locale,
            explanation_level: level,
            requested_language: language,
            guarded_request: None,
        })
        .map_err(|error| error.to_string())?;
    let listener =
        TcpListener::bind(&bind).map_err(|error| format!("cannot bind {bind}: {error}"))?;
    eprintln!("Volicord local viewer: http://{bind}/");
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|error| format!("viewer connection failed: {error}"))?;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\n\r\n{}",
            page.html.len(),
            page.html
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|error| format!("viewer response failed: {error}"))?;
    }
    Ok(())
}

fn parse_project(value: &str) -> Result<ProjectId, String> {
    let value = value.as_bytes();
    if value.len() != 32 {
        return Err("Project ID must contain 32 hexadecimal digits".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|_| "Project ID contains a non-hexadecimal digit".to_owned())?;
    }
    Ok(ProjectId::from_bytes(bytes))
}
