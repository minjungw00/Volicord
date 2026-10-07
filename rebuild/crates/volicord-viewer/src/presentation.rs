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
