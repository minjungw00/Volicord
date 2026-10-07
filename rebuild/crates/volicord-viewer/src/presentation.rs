//! Shared escaped HTML primitives and bundled, dependency-free presentation.
use super::ViewerLocale;

pub(super) fn definition(html: &mut String, term: &str, description: &str) {
    html.push_str(&format!(
        "<div><dt>{}</dt><dd>{}</dd></div>",
        escape(term),
        escape(description)
    ));
}

pub(super) fn heading(html: &mut String, level: u8, value: &str) {
    html.push_str(&format!("<h{level}>{}</h{level}>", escape(value)));
}

pub(super) fn section_start(html: &mut String, id: &str, title: &str) {
    html.push_str(&format!(
        "<section id=\"{}\" aria-labelledby=\"{}-heading\"><h2 id=\"{}-heading\">{}</h2>",
        escape(id),
        escape(id),
        escape(id),
        escape(title)
    ));
}

pub(super) fn section_end(html: &mut String) {
    html.push_str("</section>");
}

pub(super) fn list_item(html: &mut String, value: &str) {
    html.push_str(&format!("<li class=\"item\">{}</li>", escape(value)));
}

pub(super) fn empty_state(html: &mut String, value: &str) {
    html.push_str(&format!("<p class=\"empty-state\">{}</p>", escape(value)));
}

pub(super) const fn text<'a>(locale: ViewerLocale, english: &'a str, korean: &'a str) -> &'a str {
    match locale {
        ViewerLocale::English => english,
        ViewerLocale::Korean => korean,
    }
}

pub(crate) fn locale_key(locale: ViewerLocale) -> &'static str {
    match locale {
        ViewerLocale::English => "en",
        ViewerLocale::Korean => "ko",
    }
}

pub(super) fn hidden(html: &mut String, name: &str, value: &str) {
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
        escape(name),
        escape(value)
    ));
}

pub(super) fn percent_encode(value: &str) -> String {
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

pub(super) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) const STYLE: &str = concat!("<style>", include_str!("presentation.css"), "</style>");

// A presentation hint, never the arbitrary requested-language identity. Unknown
// values must not inherit the fixed interface locale as a translation claim.
pub(super) fn content_language_tag(requested: &str) -> String {
    let candidate = requested.trim().replace('_', "-");
    let parts = candidate.split('-').collect::<Vec<_>>();
    if candidate.len() <= 63
        && candidate.is_ascii()
        && parts.first().is_some_and(|p| {
            (2..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphabetic())
        })
        && parts
            .iter()
            .skip(1)
            .all(|p| !p.is_empty() && p.len() <= 8 && p.bytes().all(|b| b.is_ascii_alphanumeric()))
        && !parts.last().is_some_and(|p| p.len() == 1)
    {
        candidate
    } else {
        String::new()
    }
}

pub(super) const fn code_explanation_state_label(
    state: volicord_projections::CodeExplanationState,
    locale: ViewerLocale,
) -> &'static str {
    use volicord_projections::CodeExplanationState::*;
    match state {
        Current => text(locale, "Current", "최신"),
        Partial => text(locale, "Partial", "일부"),
        Stale => text(locale, "Stale", "오래됨"),
        Unsupported => text(locale, "Unsupported", "지원하지 않음"),
        Unavailable => text(locale, "Unavailable", "이용 불가"),
    }
}
