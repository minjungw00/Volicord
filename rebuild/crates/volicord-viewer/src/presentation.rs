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

/// Calendar presentation of the actual observation instant, in explicit UTC.
/// This does not infer Work creation/completion from a later observation.
pub(super) fn observation_time(
    timestamp: volicord_context::TimestampMicros,
) -> Option<(String, String)> {
    let seconds = timestamp.as_unix_micros().div_euclid(1_000_000);
    // RFC 3339 calendar years 0001..9999. Invalid instants keep their raw
    // identity in supporting detail, never an epoch/current-time substitute.
    if !(-62_135_596_800..=253_402_300_799).contains(&seconds) {
        return None;
    }
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    // Gregorian civil date from days since 1970-01-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3600;
    let minute = day_seconds % 3600 / 60;
    let second = day_seconds % 60;
    Some((
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"),
        format!("{year:04}.{month:02}.{day:02} {hour:02}:{minute:02} UTC"),
    ))
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

pub(super) const STYLE: &str = concat!(
    "<style>",
    include_str!("reader.css"),
    include_str!("presentation.css"),
    include_str!("shell.css"),
    include_str!("evidence.css"),
    include_str!("catalogue.css"),
    "</style>"
);

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

#[cfg(test)]
mod calendar_tests {
    #[test]
    fn observation_dates_handle_epoch_negative_and_leap_day() {
        for (seconds, date) in [
            (0, "1970-01-01T00:00:00Z"),
            (-1, "1969-12-31T23:59:59Z"),
            (951_782_400, "2000-02-29T00:00:00Z"),
            (4_107_542_400, "2100-03-01T00:00:00Z"),
        ] {
            assert_eq!(
                super::observation_time(volicord_context::TimestampMicros::from_unix_micros(
                    seconds * 1_000_000
                ))
                .map(|v| v.0),
                Some(date.into())
            );
        }
    }
}
