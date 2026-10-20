/// Inspect literal code content in actual consumer HTML, independently of the
/// lexer. Browser controls additionally check real parsed DOM/copy behavior.
pub fn code_texts(html: &str) -> Vec<String> {
    html.split("<code ")
        .skip(1)
        .map(|section| {
            let content = section
                .split_once('>')
                .expect("code attributes")
                .1
                .split_once("</code>")
                .expect("code closing tag")
                .0;
            let mut text = String::new();
            let mut tag = false;
            for c in content.chars() {
                match c {
                    '<' => tag = true,
                    '>' if tag => tag = false,
                    _ if !tag => text.push(c),
                    _ => {}
                }
            }
            text.replace("&#13;", "\r")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&amp;", "&")
        })
        .collect()
}
