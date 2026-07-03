use crate::prelude::*;
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) target: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
}

impl HttpRequest {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[derive(Debug)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) reason: &'static str,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl HttpResponse {
    pub(crate) fn json(
        status: u16,
        reason: &'static str,
        value: Value,
        headers: Vec<(String, String)>,
    ) -> Self {
        let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
        Self {
            status,
            reason,
            headers: with_content_type(headers, "application/json"),
            body,
        }
    }

    pub(crate) fn empty(status: u16, reason: &'static str, headers: Vec<(String, String)>) -> Self {
        Self {
            status,
            reason,
            headers,
            body: Vec::new(),
        }
    }

    pub(crate) fn html(status: u16, reason: &'static str, body: String) -> Self {
        Self {
            status,
            reason,
            headers: vec![(
                "Content-Type".to_owned(),
                "text/html; charset=utf-8".to_owned(),
            )],
            body: body.into_bytes(),
        }
    }

    pub(crate) fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

pub(crate) fn structured_http_error(
    status: u16,
    reason: &'static str,
    code: &'static str,
    message: &str,
) -> HttpResponse {
    structured_http_error_with_headers(status, reason, code, message, Vec::new())
}

pub(crate) fn structured_http_error_with_headers(
    status: u16,
    reason: &'static str,
    code: &'static str,
    message: &str,
    headers: Vec<(String, String)>,
) -> HttpResponse {
    HttpResponse::json(
        status,
        reason,
        json!({
            "disclosure": detective_observation_disclosure_json(),
            "error": {
                "code": code,
                "message": message
            }
        }),
        headers,
    )
}

pub(crate) fn detective_observation_disclosure_json() -> Value {
    serde_json::to_value(GuaranteeDisclosure::detective_observation())
        .expect("guarantee disclosure should serialize")
}

pub(crate) fn with_content_type(
    mut headers: Vec<(String, String)>,
    content_type: &str,
) -> Vec<(String, String)> {
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    {
        headers.push(("Content-Type".to_owned(), content_type.to_owned()));
    }
    headers
}

pub(crate) fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, HttpResponse> {
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 1024];
        let read = stream.read(&mut chunk).map_err(|error| {
            structured_http_error(
                400,
                "Bad Request",
                "HTTP_READ_FAILED",
                &format!("failed to read HTTP request: {error}"),
            )
        })?;
        if read == 0 {
            return Err(structured_http_error(
                400,
                "Bad Request",
                "HTTP_REQUEST_INCOMPLETE",
                "HTTP request ended before headers completed",
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > HTTP_HEADER_LIMIT_BYTES {
            return Err(structured_http_error(
                431,
                "Request Header Fields Too Large",
                "HTTP_HEADERS_TOO_LARGE",
                "HTTP request headers exceed the Volicord limit",
            ));
        }
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
    };

    let head = str::from_utf8(&buffer[..header_end]).map_err(|_| {
        structured_http_error(
            400,
            "Bad Request",
            "HTTP_HEADER_ENCODING_INVALID",
            "HTTP headers must be valid UTF-8",
        )
    })?;
    let (method, target, headers) = parse_http_head(head)?;
    let content_length = match headers.get("content-length") {
        Some(value) => value.parse::<usize>().map_err(|_| {
            structured_http_error(
                400,
                "Bad Request",
                "CONTENT_LENGTH_INVALID",
                "Content-Length must be a decimal byte count",
            )
        })?,
        None => 0,
    };
    if content_length > HTTP_BODY_LIMIT_BYTES {
        return Err(structured_http_error(
            413,
            "Payload Too Large",
            "HTTP_BODY_TOO_LARGE",
            "HTTP request body exceeds the Volicord limit",
        ));
    }

    let body_start = header_end + 4;
    let mut body = buffer.get(body_start..).unwrap_or_default().to_vec();
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let mut chunk = vec![0_u8; remaining.min(8192)];
        let read = stream.read(&mut chunk).map_err(|error| {
            structured_http_error(
                400,
                "Bad Request",
                "HTTP_BODY_READ_FAILED",
                &format!("failed to read HTTP request body: {error}"),
            )
        })?;
        if read == 0 {
            return Err(structured_http_error(
                400,
                "Bad Request",
                "HTTP_BODY_INCOMPLETE",
                "HTTP request ended before the declared body length",
            ));
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        target,
        headers,
        body,
    })
}

pub(crate) fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(crate) fn parse_http_head(
    head: &str,
) -> Result<(String, String, BTreeMap<String, String>), HttpResponse> {
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or_else(|| {
        structured_http_error(
            400,
            "Bad Request",
            "HTTP_REQUEST_LINE_MISSING",
            "HTTP request line is missing",
        )
    })?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    if method.is_empty() || target.is_empty() || version != "HTTP/1.1" || parts.next().is_some() {
        return Err(structured_http_error(
            400,
            "Bad Request",
            "HTTP_REQUEST_LINE_INVALID",
            "HTTP request line must be METHOD TARGET HTTP/1.1",
        ));
    }

    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(structured_http_error(
                400,
                "Bad Request",
                "HTTP_HEADER_INVALID",
                "HTTP header line must contain ':'",
            ));
        };
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty() {
            return Err(structured_http_error(
                400,
                "Bad Request",
                "HTTP_HEADER_INVALID",
                "HTTP header name must not be empty",
            ));
        }
        headers.insert(name, value.trim().to_owned());
    }

    Ok((method.to_ascii_uppercase(), target.to_owned(), headers))
}

pub(crate) fn write_http_response(
    stream: &mut TcpStream,
    response: HttpResponse,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.reason,
        response.body.len()
    )?;
    if !response
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("Cache-Control"))
    {
        stream.write_all(b"Cache-Control: no-store\r\n")?;
    }
    if !response
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("X-Content-Type-Options"))
    {
        stream.write_all(b"X-Content-Type-Options: nosniff\r\n")?;
    }
    for (name, value) in response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(&response.body)?;
    stream.flush()
}

pub(crate) fn accepts_content_type(header: Option<&str>, expected: &str) -> bool {
    let Some(header) = header else {
        return false;
    };
    header.split(',').any(|item| {
        let media_type = item
            .trim()
            .split_once(';')
            .map(|(media_type, _)| media_type.trim())
            .unwrap_or_else(|| item.trim());
        media_type == expected || media_type == "*/*"
    })
}

pub(crate) fn content_type_is_json(header: Option<&str>) -> bool {
    let Some(header) = header else {
        return false;
    };
    header
        .split_once(';')
        .map(|(media_type, _)| media_type.trim())
        .unwrap_or_else(|| header.trim())
        == "application/json"
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
