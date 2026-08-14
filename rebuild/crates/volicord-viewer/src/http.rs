use crate::{ExplanationLevel, ViewerAdapter, ViewerError, ViewerLocale, ViewerRequest};
use std::{
    collections::BTreeMap,
    fmt,
    io::{self, Read, Write},
    net::SocketAddr,
    path::PathBuf,
};
use volicord_context::{
    CanonicalRecordId, CheckpointId, ContextItemId, DecisionId, ProjectId, QuestionId, SourceId,
};
use volicord_operations::{ConfirmationDecision, ConfirmationRequestId};
use volicord_projections::{DocumentKind, OutputFormat};

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_TARGET_BYTES: usize = 2 * 1024;

pub struct ViewerServer {
    adapter: ViewerAdapter,
    project_id: ProjectId,
    default_locale: ViewerLocale,
    default_level: ExplanationLevel,
    requested_language: String,
    session: String,
    authority: String,
    origin: String,
    request_authenticity: String,
}

impl ViewerServer {
    pub fn new(
        adapter: ViewerAdapter,
        project_id: ProjectId,
        default_locale: ViewerLocale,
        default_level: ExplanationLevel,
        requested_language: String,
        authority: SocketAddr,
    ) -> Result<Self, ViewerError> {
        if !authority.ip().is_loopback() {
            return Err(ViewerError::new(
                "the local viewer authority must use a loopback address",
            ));
        }
        let authority = authority.to_string();
        Ok(Self {
            adapter,
            project_id,
            default_locale,
            default_level,
            requested_language,
            session: format!("viewer-process-{}", std::process::id()),
            origin: format!("http://{authority}"),
            authority,
            request_authenticity: new_request_authenticity()?,
        })
    }

    pub fn adapter(&self) -> &ViewerAdapter {
        &self.adapter
    }

    pub fn serve_connection(
        &self,
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<(), ViewerError> {
        let response = match HttpRequest::read(reader).and_then(|request| self.route(request)) {
            Ok(response) => response,
            Err(error) => HttpResponse::error(error),
        };
        response.write(writer).map_err(|error| {
            ViewerError::new(format!("cannot write local viewer response: {error}"))
        })
    }

    fn route(&self, request: HttpRequest) -> Result<HttpResponse, HttpFailure> {
        self.validate_authority(&request)?;
        if request.method == "POST" {
            self.validate_mutation_request(&request)?;
        }
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/") => self.render(request.query, None),
            ("POST", "/memory/context/correct") => {
                let form = request.form()?;
                form.require_only(&[
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                    "user_turn",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let source = self.user_source(form.required("user_turn")?)?;
                self.adapter
                    .correct_context(
                        self.project_id,
                        ContextItemId::from_bytes(identity(form.required("record_id")?)?),
                        form.u64("expected_revision")?,
                        form.required("corrected_text")?.to_owned(),
                        source,
                    )
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            ("POST", "/memory/decision/correct") => {
                let form = request.form()?;
                form.require_only(&[
                    "record_id",
                    "expected_revision",
                    "corrected_text",
                    "user_turn",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let source = self.user_source(form.required("user_turn")?)?;
                self.adapter
                    .correct_decision(
                        self.project_id,
                        DecisionId::from_bytes(identity(form.required("record_id")?)?),
                        form.u64("expected_revision")?,
                        form.required("corrected_text")?.to_owned(),
                        source,
                    )
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            ("POST", "/memory/decision/supersede") => {
                let form = request.form()?;
                form.require_only(&[
                    "record_id",
                    "alternative",
                    "rationale",
                    "user_turn",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let source = self.user_source(form.required("user_turn")?)?;
                self.adapter
                    .supersede_decision(
                        self.project_id,
                        DecisionId::from_bytes(identity(form.required("record_id")?)?),
                        source,
                        form.required("alternative")?.to_owned(),
                        form.optional("rationale").map(ToOwned::to_owned),
                    )
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            ("POST", "/memory/forget") => {
                let form = request.form()?;
                form.require_only(&[
                    "record_kind",
                    "record_id",
                    "user_turn",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let record = canonical_record(
                    form.required("record_kind")?,
                    identity(form.required("record_id")?)?,
                )?;
                let source = self.user_source(form.required("user_turn")?)?;
                self.adapter
                    .forget(self.project_id, record, source)
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            ("POST", "/guarded/confirm") => {
                let form = request.form()?;
                form.require_only(&[
                    "confirmation_request_id",
                    "request_revision",
                    "effect_fingerprint",
                    "decision",
                    "user_turn",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let decision = match form.required("decision")? {
                    "confirm" => ConfirmationDecision::Confirmed,
                    "deny" => ConfirmationDecision::Denied,
                    _ => return Err(HttpFailure::bad_request("decision must be confirm or deny")),
                };
                self.adapter
                    .confirm_guarded(
                        ConfirmationRequestId::from_bytes(identity(
                            form.required("confirmation_request_id")?,
                        )?),
                        form.u64("request_revision")?,
                        form.required("effect_fingerprint")?,
                        decision,
                        self.session.clone(),
                        form.required("user_turn")?.to_owned(),
                    )
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            ("POST", "/documents/export") => {
                let form = request.form()?;
                form.require_only(&[
                    "kind",
                    "format",
                    "destination",
                    "level",
                    "locale",
                    "language",
                    "guarded",
                    "request_authenticity",
                ])?;
                let view = self.view_parameters(&form.values)?;
                self.adapter
                    .export_document(
                        self.project_id,
                        document_kind(form.required("kind")?)?,
                        output_format(form.required("format")?)?,
                        &PathBuf::from(form.required("destination")?),
                        view.language,
                        view.locale,
                    )
                    .map_err(domain_failure)?;
                Ok(HttpResponse::redirect(form.return_location()))
            }
            (_, path) if path.starts_with("/guarded/") => {
                if request.method != "GET" {
                    return Err(HttpFailure::method_not_allowed("GET"));
                }
                let value = path.strip_prefix("/guarded/").unwrap_or_default();
                if value.is_empty() || value.contains('/') {
                    return Err(HttpFailure::not_found());
                }
                self.render(
                    request.query,
                    Some(ConfirmationRequestId::from_bytes(identity(value)?)),
                )
            }
            ("GET", _) | ("POST", _) => Err(HttpFailure::not_found()),
            _ => Err(HttpFailure::method_not_allowed("GET, POST")),
        }
    }

    fn validate_authority(&self, request: &HttpRequest) -> Result<(), HttpFailure> {
        if request.host.as_deref() != Some(self.authority.as_str()) {
            return Err(HttpFailure::new(
                421,
                "Misdirected Request",
                "request Host does not match the active local viewer authority",
            ));
        }
        Ok(())
    }

    fn validate_mutation_request(&self, request: &HttpRequest) -> Result<(), HttpFailure> {
        if request.origin.as_deref() != Some(self.origin.as_str()) {
            return Err(HttpFailure::forbidden());
        }
        if request
            .fetch_site
            .as_deref()
            .is_some_and(|value| value != "same-origin")
        {
            return Err(HttpFailure::forbidden());
        }
        let form = request.form()?;
        let supplied = form
            .optional("request_authenticity")
            .ok_or_else(HttpFailure::forbidden)?;
        if !constant_time_equal(supplied.as_bytes(), self.request_authenticity.as_bytes()) {
            return Err(HttpFailure::forbidden());
        }
        Ok(())
    }

    fn render(
        &self,
        query: FormData,
        guarded_path: Option<ConfirmationRequestId>,
    ) -> Result<HttpResponse, HttpFailure> {
        query.require_only(&["level", "locale", "language", "guarded"])?;
        let view = self.view_parameters(&query.values)?;
        let guarded_query = query
            .optional("guarded")
            .map(identity)
            .transpose()?
            .map(ConfirmationRequestId::from_bytes);
        if guarded_path.is_some() && guarded_query.is_some() && guarded_path != guarded_query {
            return Err(HttpFailure::bad_request(
                "guarded path and query identities do not match",
            ));
        }
        let guarded = guarded_path.or(guarded_query);
        let page = self
            .adapter
            .render(
                &ViewerRequest {
                    project_id: self.project_id,
                    locale: view.locale,
                    explanation_level: view.level,
                    requested_language: view.language,
                    guarded_request: guarded,
                },
                &self.request_authenticity,
            )
            .map_err(domain_failure)?;
        Ok(HttpResponse::html(page.html))
    }

    fn view_parameters(
        &self,
        values: &BTreeMap<String, String>,
    ) -> Result<ViewParameters, HttpFailure> {
        let level = match values.get("level").map(String::as_str) {
            None => self.default_level,
            Some("overview") => ExplanationLevel::Overview,
            Some("working") => ExplanationLevel::Working,
            Some("deep") => ExplanationLevel::Deep,
            Some(_) => return Err(HttpFailure::bad_request("unknown explanation level")),
        };
        let locale = match values.get("locale").map(String::as_str) {
            None => self.default_locale,
            Some("en") => ViewerLocale::English,
            Some("ko") => ViewerLocale::Korean,
            Some(_) => return Err(HttpFailure::bad_request("unsupported fixed locale")),
        };
        let language = values
            .get("language")
            .cloned()
            .unwrap_or_else(|| self.requested_language.clone());
        if language.is_empty() || language.len() > 128 {
            return Err(HttpFailure::bad_request(
                "requested content language must contain 1 to 128 bytes",
            ));
        }
        Ok(ViewParameters {
            level,
            locale,
            language,
        })
    }

    fn user_source(&self, turn: &str) -> Result<SourceId, HttpFailure> {
        if turn.is_empty() || turn.len() > 8 * 1024 {
            return Err(HttpFailure::bad_request(
                "user_turn must contain 1 to 8192 bytes",
            ));
        }
        let outcome = self
            .adapter
            .operations()
            .record_user_source(
                self.project_id,
                "local-viewer".into(),
                self.session.clone(),
                turn.to_owned(),
            )
            .map_err(|error| domain_failure(ViewerError::new(error.to_string())))?;
        Ok(SourceId::from_bytes(identity(&outcome.identity)?))
    }
}

struct ViewParameters {
    level: ExplanationLevel,
    locale: ViewerLocale,
    language: String,
}

struct HttpRequest {
    method: String,
    path: String,
    query: FormData,
    content_type: Option<String>,
    host: Option<String>,
    origin: Option<String>,
    fetch_site: Option<String>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn read(reader: &mut impl Read) -> Result<Self, HttpFailure> {
        let mut header = Vec::new();
        let mut byte = [0_u8; 1];
        while !header.ends_with(b"\r\n\r\n") {
            if header.len() >= MAX_HEADER_BYTES {
                return Err(HttpFailure::new(
                    431,
                    "Request Header Fields Too Large",
                    "request headers exceed 16384 bytes",
                ));
            }
            match reader.read(&mut byte) {
                Ok(0) => {
                    return Err(HttpFailure::bad_request(
                        "request ended before headers were complete",
                    ))
                }
                Ok(_) => header.push(byte[0]),
                Err(error) => {
                    return Err(HttpFailure::bad_request(format!(
                        "cannot read request headers: {error}"
                    )))
                }
            }
        }
        let header_text = std::str::from_utf8(&header)
            .map_err(|_| HttpFailure::bad_request("request headers must be UTF-8"))?;
        let mut lines = header_text[..header_text.len() - 4].split("\r\n");
        let request_line = lines
            .next()
            .ok_or_else(|| HttpFailure::bad_request("request line is missing"))?;
        let fields = request_line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || !matches!(fields[2], "HTTP/1.0" | "HTTP/1.1") {
            return Err(HttpFailure::bad_request(
                "request line must use HTTP/1.0 or HTTP/1.1",
            ));
        }
        if fields[1].len() > MAX_TARGET_BYTES || !fields[1].starts_with('/') {
            return Err(HttpFailure::bad_request(
                "request target is invalid or too long",
            ));
        }
        let mut content_length = None;
        let mut content_type = None;
        let mut host = None;
        let mut origin = None;
        let mut fetch_site = None;
        for line in lines {
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| HttpFailure::bad_request("malformed request header"))?;
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            match name.as_str() {
                "content-length" => {
                    if content_length.is_some() {
                        return Err(HttpFailure::bad_request("duplicate Content-Length"));
                    }
                    content_length = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| HttpFailure::bad_request("invalid Content-Length"))?,
                    );
                }
                "content-type" => {
                    if content_type.is_some() {
                        return Err(HttpFailure::bad_request("duplicate Content-Type"));
                    }
                    content_type = Some(value.to_ascii_lowercase());
                }
                "host" => set_unique_header(&mut host, value, "Host")?,
                "origin" => set_unique_header(&mut origin, value, "Origin")?,
                "sec-fetch-site" => set_unique_header(&mut fetch_site, value, "Sec-Fetch-Site")?,
                "transfer-encoding" => {
                    return Err(HttpFailure::bad_request(
                        "Transfer-Encoding is not supported",
                    ))
                }
                _ => {}
            }
        }
        let body_len = content_length.unwrap_or(0);
        if body_len > MAX_BODY_BYTES {
            return Err(HttpFailure::new(
                413,
                "Payload Too Large",
                "request body exceeds 65536 bytes",
            ));
        }
        let mut body = vec![0_u8; body_len];
        reader.read_exact(&mut body).map_err(|error| {
            HttpFailure::bad_request(format!("request body is incomplete: {error}"))
        })?;
        let (path, raw_query) = fields[1].split_once('?').unwrap_or((fields[1], ""));
        Ok(Self {
            method: fields[0].to_owned(),
            path: percent_decode(path)?,
            query: FormData::parse(raw_query.as_bytes())?,
            content_type,
            host,
            origin,
            fetch_site,
            body,
        })
    }

    fn form(&self) -> Result<FormData, HttpFailure> {
        if !self.content_type.as_deref().is_some_and(|value| {
            value == "application/x-www-form-urlencoded"
                || value.starts_with("application/x-www-form-urlencoded;")
        }) {
            return Err(HttpFailure::new(
                415,
                "Unsupported Media Type",
                "POST requires application/x-www-form-urlencoded",
            ));
        }
        FormData::parse(&self.body)
    }
}

#[derive(Default)]
struct FormData {
    values: BTreeMap<String, String>,
}

impl FormData {
    fn parse(bytes: &[u8]) -> Result<Self, HttpFailure> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| HttpFailure::bad_request("form data must be UTF-8"))?;
        let mut values = BTreeMap::new();
        if text.is_empty() {
            return Ok(Self { values });
        }
        for pair in text.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = percent_decode_form(key)?;
            let value = percent_decode_form(value)?;
            if key.is_empty() || values.insert(key, value).is_some() {
                return Err(HttpFailure::bad_request(
                    "form keys must be non-empty and unique",
                ));
            }
        }
        Ok(Self { values })
    }

    fn required(&self, key: &str) -> Result<&str, HttpFailure> {
        self.values
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| HttpFailure::bad_request(format!("{key} is required")))
    }

    fn optional(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    fn u64(&self, key: &str) -> Result<u64, HttpFailure> {
        self.required(key)?
            .parse()
            .map_err(|_| HttpFailure::bad_request(format!("{key} must be an unsigned integer")))
    }

    fn require_only(&self, allowed: &[&str]) -> Result<(), HttpFailure> {
        if let Some(key) = self
            .values
            .keys()
            .find(|key| !allowed.contains(&key.as_str()))
        {
            return Err(HttpFailure::bad_request(format!("unknown field: {key}")));
        }
        Ok(())
    }

    fn return_location(&self) -> String {
        let mut fields = Vec::new();
        for key in ["level", "locale", "language", "guarded"] {
            if let Some(value) = self.values.get(key).filter(|value| !value.is_empty()) {
                fields.push(format!("{key}={}", percent_encode(value)));
            }
        }
        if fields.is_empty() {
            "/".into()
        } else {
            format!("/?{}", fields.join("&"))
        }
    }
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    location: Option<String>,
}

impl HttpResponse {
    fn html(body: String) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
            location: None,
        }
    }

    fn redirect(location: String) -> Self {
        Self {
            status: 303,
            reason: "See Other",
            content_type: "text/plain; charset=utf-8",
            body: b"See Other\n".to_vec(),
            location: Some(location),
        }
    }

    fn error(error: HttpFailure) -> Self {
        Self {
            status: error.status,
            reason: error.reason,
            content_type: "text/plain; charset=utf-8",
            body: format!("{}\n", error.message).into_bytes(),
            location: None,
        }
    }

    fn write(self, writer: &mut impl Write) -> io::Result<()> {
        write!(writer, "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'\r\n", self.status, self.reason, self.content_type, self.body.len())?;
        if let Some(location) = self.location {
            write!(writer, "Location: {location}\r\n")?;
        }
        writer.write_all(b"\r\n")?;
        writer.write_all(&self.body)
    }
}

struct HttpFailure {
    status: u16,
    reason: &'static str,
    message: String,
}

impl HttpFailure {
    fn new(status: u16, reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            reason,
            message: message.into(),
        }
    }
    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, "Bad Request", message)
    }
    fn not_found() -> Self {
        Self::new(404, "Not Found", "unknown viewer path")
    }
    fn method_not_allowed(allowed: &'static str) -> Self {
        Self::new(
            405,
            "Method Not Allowed",
            format!("allowed methods: {allowed}"),
        )
    }
    fn forbidden() -> Self {
        Self::new(
            403,
            "Forbidden",
            "local viewer request authenticity check failed",
        )
    }
}

impl fmt::Display for HttpFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn domain_failure(error: ViewerError) -> HttpFailure {
    HttpFailure::new(422, "Unprocessable Content", error.to_string())
}

fn identity(value: &str) -> Result<[u8; 16], HttpFailure> {
    if value.len() != 32 {
        return Err(HttpFailure::bad_request(
            "identity must contain 32 hexadecimal digits",
        ));
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair =
            std::str::from_utf8(pair).map_err(|_| HttpFailure::bad_request("invalid identity"))?;
        bytes[index] = u8::from_str_radix(pair, 16)
            .map_err(|_| HttpFailure::bad_request("identity contains a non-hexadecimal digit"))?;
    }
    Ok(bytes)
}

fn canonical_record(kind: &str, bytes: [u8; 16]) -> Result<CanonicalRecordId, HttpFailure> {
    match kind {
        "source" => Ok(CanonicalRecordId::Source(SourceId::from_bytes(bytes))),
        "question" => Ok(CanonicalRecordId::Question(QuestionId::from_bytes(bytes))),
        "decision" => Ok(CanonicalRecordId::Decision(DecisionId::from_bytes(bytes))),
        "context_item" => Ok(CanonicalRecordId::ContextItem(ContextItemId::from_bytes(
            bytes,
        ))),
        "checkpoint" => Ok(CanonicalRecordId::Checkpoint(CheckpointId::from_bytes(
            bytes,
        ))),
        _ => Err(HttpFailure::bad_request("record_kind is not forgettable")),
    }
}

fn document_kind(value: &str) -> Result<DocumentKind, HttpFailure> {
    match value {
        "project-architecture-guide" => Ok(DocumentKind::ProjectArchitectureGuide),
        "decision-report" => Ok(DocumentKind::DecisionReport),
        "implementation-plan" => Ok(DocumentKind::ImplementationPlan),
        "handoff-resume" => Ok(DocumentKind::HandoffResume),
        _ => Err(HttpFailure::bad_request("unknown document kind")),
    }
}

fn output_format(value: &str) -> Result<OutputFormat, HttpFailure> {
    match value {
        "markdown" => Ok(OutputFormat::Markdown),
        "html" => Ok(OutputFormat::Html),
        _ => Err(HttpFailure::bad_request("format must be markdown or html")),
    }
}

fn percent_decode_form(value: &str) -> Result<String, HttpFailure> {
    percent_decode(&value.replace('+', " "))
}

fn percent_decode(value: &str) -> Result<String, HttpFailure> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(HttpFailure::bad_request("invalid percent encoding"));
            }
            let pair = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| HttpFailure::bad_request("invalid percent encoding"))?;
            decoded.push(
                u8::from_str_radix(pair, 16)
                    .map_err(|_| HttpFailure::bad_request("invalid percent encoding"))?,
            );
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| HttpFailure::bad_request("decoded value is not UTF-8"))
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn set_unique_header(
    destination: &mut Option<String>,
    value: &str,
    name: &str,
) -> Result<(), HttpFailure> {
    if destination.is_some() || value.is_empty() {
        return Err(HttpFailure::bad_request(format!(
            "{name} must be present at most once and non-empty"
        )));
    }
    *destination = Some(value.to_owned());
    Ok(())
}

fn new_request_authenticity() -> Result<String, ViewerError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        ViewerError::new(format!(
            "operating-system randomness for viewer request authenticity is unavailable: {error}"
        ))
    })?;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }
    Ok(encoded)
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}
