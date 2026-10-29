#[test]
fn shared_presentation_escapes_text_attributes_and_preserves_unicode() {
    let hostile = "한국어 <script> & \"quoted\" 'single'";
    let mut html = String::new();
    super::section_start(&mut html, hostile, hostile);
    super::definition(&mut html, hostile, hostile);
    super::hidden(&mut html, hostile, hostile);
    assert!(!html.contains("<script>"));
    assert!(html.contains("한국어 &lt;script&gt; &amp; &quot;quoted&quot; &#39;single&#39;"));
    assert_eq!(super::percent_encode("ko &한"), "ko%20%26%ED%95%9C");
}

#[test]
fn reader_dates_keep_microseconds_explicit_without_guessing_seconds_or_missing_values() {
    use super::{timestamp_label, TimestampMicros, ViewerLocale};
    for locale in [ViewerLocale::English, ViewerLocale::Korean] {
        for (micros, expected) in [
            (0, "1970.01.01 00:00 UTC"),
            (-1, "1969.12.31 23:59 UTC"),
            (951_782_400_000_000, "2000.02.29 00:00 UTC"),
            (1_792_494_581_594_629, "2026.10.20 11:09 UTC"),
            (1_792_494_581, "1970.01.01 00:29 UTC"),
        ] {
            let label = timestamp_label(TimestampMicros::from_unix_micros(micros), locale);
            assert!(label.starts_with(expected), "{label}");
            assert!(label.ends_with(&format!("Unix µs: {micros}")));
        }
        for micros in [i64::MIN, i64::MAX] {
            let label = timestamp_label(TimestampMicros::from_unix_micros(micros), locale);
            assert!(label.contains(if locale == ViewerLocale::Korean {
                "날짜 확인 불가"
            } else {
                "Date unavailable"
            }));
            assert!(!label.contains("1970"));
            assert!(label.ends_with(&format!("Unix µs: {micros}")));
        }
    }
}
