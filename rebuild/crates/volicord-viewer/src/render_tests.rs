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
