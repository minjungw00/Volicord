//! Bounded lexical decoration of already admitted text. No I/O, parser claims,
//! retained state, or context outside this chunk. Static and live use this lexer.
//! Deliberately incomplete syntax (including chunk boundaries) is only a visual
//! approximation. Unknown formats and command streams receive no decoration.
use volicord_projections::{ExplanationCitation, ExplanationSourceUse};

const MAX_BYTES: usize = 4096;
const MAX_TOKENS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Language {
    JavaScript,
    TypeScript,
    Rust,
    Python,
    Shell,
    Json,
}

impl Language {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Shell => "shell",
            Self::Json => "json",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Shell => "Shell",
            Self::Json => "JSON",
        }
    }
}

/// A locator supplies a display hint only after scoped Source validation. It is
/// never opened. No language metadata exists in the current excerpt envelope;
/// field/use/representation distinguish file bytes from execution streams.
pub(crate) fn language(
    citation: &ExplanationCitation,
    representation: Option<&str>,
    execution: bool,
) -> Option<Language> {
    if execution
        || citation.use_kind == ExplanationSourceUse::ObservedExecution
        || citation.field != "body"
        || !matches!(representation, Some("whole_file" | "bounded_excerpt"))
    {
        return None;
    }
    match citation.locator.rsplit('.').next()? {
        "js" | "mjs" | "cjs" | "jsx" => Some(Language::JavaScript),
        "ts" | "mts" | "cts" | "tsx" => Some(Language::TypeScript),
        "rs" => Some(Language::Rust),
        "py" | "pyi" => Some(Language::Python),
        "sh" | "bash" | "zsh" => Some(Language::Shell),
        "json" => Some(Language::Json),
        _ => None,
    }
}

#[derive(Debug)]
struct Token {
    start: usize,
    end: usize,
    kind: &'static str,
}

/// Runs reference UTF-8 byte boundaries internally. Only colored runs allocate;
/// punctuation/whitespace remain one text node between runs. Excess complexity
/// falls back to the exact plain chunk rather than partial/misleading markup.
fn tokens(source: &str, language: Option<Language>) -> Vec<Token> {
    let Some(language) = language.filter(|_| source.len() <= MAX_BYTES) else {
        return Vec::new();
    };
    let b = source.as_bytes();
    let mut result: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let mut kind = None;
        let hash_comment = matches!(language, Language::Python | Language::Shell)
            && b[i] == b'#'
            && (language != Language::Shell
                || i == 0
                || b[i - 1].is_ascii_whitespace()
                || b";|&()".contains(&b[i - 1]));
        let slash_comment = matches!(
            language,
            Language::Rust | Language::JavaScript | Language::TypeScript
        );
        if hash_comment || (slash_comment && b[i..].starts_with(b"//")) {
            i = source[i..].find('\n').map_or(b.len(), |n| i + n);
            kind = Some("comment");
        } else if slash_comment && b[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < b.len() && depth > 0 {
                if language == Language::Rust && b[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if b[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            kind = Some("comment");
        } else if language == Language::Rust && rust_raw_end(source, i).is_some() {
            i = rust_raw_end(source, i).unwrap_or(b.len());
            kind = Some("string");
        } else if b[i] == b'"'
            || (b[i] == b'\'' && language != Language::Json)
            || (b[i] == b'`'
                && matches!(
                    language,
                    Language::JavaScript | Language::TypeScript | Language::Shell
                ))
        {
            let quote = b[i];
            // Rust lifetimes are identifiers, not unterminated string literals.
            if language == Language::Rust && quote == b'\'' && !rust_character(source, i) {
                i += 1;
            } else {
                let triple = language == Language::Python
                    && b.get(i..i + 3).is_some_and(|v| v == [quote; 3]);
                let width = if triple { 3 } else { 1 };
                i += width;
                while i < b.len() {
                    if b[i] == b'\\' && !(language == Language::Shell && quote == b'\'') {
                        i = (i + 2).min(b.len());
                    } else if b
                        .get(i..i + width)
                        .is_some_and(|v| v.iter().all(|c| *c == quote))
                    {
                        i += width;
                        break;
                    } else {
                        i += 1;
                    }
                }
                kind = Some("string");
            }
        } else if language == Language::Shell && b[i] == b'$' {
            i += 1;
            if b.get(i) == Some(&b'{') {
                i += 1;
                while i < b.len() && b[i] != b'}' && b[i] != b'\n' {
                    i += 1;
                }
                i += usize::from(b.get(i) == Some(&b'}'));
            } else {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b"_?@*#!-$".contains(&b[i])) {
                    i += 1;
                }
            }
            kind = Some("variable");
        } else if b[i].is_ascii_digit()
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b"._".contains(&b[i])) {
                i += 1;
            }
            kind = Some("number");
        } else if b[i].is_ascii_alphabetic() || b[i] == b'_' || b[i] >= 128 {
            i += source[i..].chars().next().map_or(1, char::len_utf8);
            while i < b.len() {
                let c = source[i..].chars().next().unwrap_or(' ');
                if !(c.is_alphanumeric() || c == '_' || c == '$') {
                    break;
                }
                i += c.len_utf8();
            }
            let word = &source[start..i];
            if keyword(word, language) {
                kind = Some("keyword");
            } else if language != Language::Json && language != Language::Shell {
                if word.starts_with(char::is_uppercase) {
                    kind = Some("type");
                } else if source[i..].trim_start().starts_with('(') {
                    kind = Some("function");
                }
            }
        } else {
            i += 1;
        }
        // ASCII delimiters cannot split UTF-8 except an escaped final scalar.
        while !source.is_char_boundary(i) {
            i += 1;
        }
        if let Some(kind) = kind {
            if let Some(last) = result
                .last_mut()
                .filter(|t| t.end == start && t.kind == kind)
            {
                last.end = i;
            } else {
                if result.len() == MAX_TOKENS {
                    return Vec::new();
                }
                result.push(Token {
                    start,
                    end: i,
                    kind,
                });
            }
        }
    }
    result
}

fn keyword(word: &str, language: Language) -> bool {
    let words = match language {
        Language::JavaScript | Language::TypeScript => "as async await break case catch class const continue debugger declare default delete do else enum export extends false finally for from function if implements import in instanceof interface let new null of private protected public readonly return satisfies static super switch this throw true try type typeof undefined var void while with yield",
        Language::Rust => "as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self static struct super trait true type unsafe use where while Self",
        Language::Python => "and as assert async await break case class continue def del elif else except False finally for from global if import in is lambda match None nonlocal not or pass raise return True try while with yield",
        Language::Shell => "if then else elif fi for while until do done case esac in function select time coproc export local readonly declare return break continue",
        Language::Json => "true false null",
    };
    words.split_ascii_whitespace().any(|k| k == word)
}

fn rust_raw_end(source: &str, start: usize) -> Option<usize> {
    let b = source.as_bytes();
    let mut i = start;
    if b.get(i) == Some(&b'b') || b.get(i) == Some(&b'c') {
        i += 1;
    }
    if b.get(i) != Some(&b'r') {
        return None;
    }
    i += 1;
    let hashes = i;
    while b.get(i) == Some(&b'#') {
        i += 1;
    }
    let count = i - hashes;
    if b.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    while i < b.len() {
        if b[i] == b'"' && b.get(i + 1..i + 1 + count) == b.get(hashes..hashes + count) {
            return Some(i + 1 + count);
        }
        i += 1;
    }
    Some(b.len())
}

fn rust_character(source: &str, start: usize) -> bool {
    let tail = &source[start + 1..];
    if tail.starts_with('\\') {
        // Escaped chars (including Unicode escapes); don't consume later lines.
        tail.split('\n').next().is_some_and(|s| s.contains('\''))
    } else {
        tail.chars()
            .next()
            .is_some_and(|c| tail[c.len_utf8()..].starts_with('\''))
    }
}

fn escape(source: &str) -> String {
    // Literal CR would be normalized by HTML parsing; character references keep
    // CRLF and isolated CR exact in script-free code.textContent, too.
    source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\r', "&#13;")
}

pub(crate) fn html(source: &str, language: Option<Language>) -> String {
    // Complete static windows use the same bounded lexer as successive live
    // pages. Keep all source bytes and the existing per-page token limit;
    // decoration must not disappear solely because a window spans two pages.
    if source.len() > MAX_BYTES
        && source.len() <= volicord_projections::EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT
    {
        let mut rendered = String::new();
        let mut remaining = source;
        while !remaining.is_empty() {
            let mut end = MAX_BYTES.min(remaining.len());
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            rendered.push_str(&html(&remaining[..end], language));
            remaining = &remaining[end..];
        }
        return rendered;
    }
    let mut html = String::new();
    let mut end = 0;
    for token in tokens(source, language) {
        html.push_str(&escape(&source[end..token.start]));
        html.push_str(&format!(
            "<span class=\"syn-{}\">{}</span>",
            token.kind,
            escape(&source[token.start..token.end])
        ));
        end = token.end;
    }
    html.push_str(&escape(&source[end..]));
    html
}

/// Live transport contains offsets in DOM UTF-16 units, never captured HTML.
/// The client validates every range and only creates allowlisted span classes.
pub(crate) fn decoration(source: &str, language: Option<Language>) -> serde_json::Value {
    let mut cursor = 0;
    let mut utf16 = 0;
    let runs: Vec<_> = tokens(source, language)
        .into_iter()
        .map(|t| {
            utf16 += source[cursor..t.start].encode_utf16().count();
            let start = utf16;
            utf16 += source[t.start..t.end].encode_utf16().count();
            cursor = t.end;
            (start, utf16, t.kind)
        })
        .collect();
    serde_json::json!({"language":language.map(Language::key),"tokens":runs})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_chunk_rendering_cost_and_growth() {
        // Current real Rust input, not benchmark answers or a timing SLA.
        let full = include_str!("source_highlight.rs");
        let mut end = MAX_BYTES.min(full.len());
        while !full.is_char_boundary(end) {
            end -= 1;
        }
        let source = &full[..end];
        let language = Some(Language::Rust);
        let rendered = html(source, language);
        let metadata = decoration(source, language);
        let count = metadata["tokens"].as_array().map_or(0, Vec::len);
        assert!(count <= MAX_TOKENS);
        // Bound includes worst-case HTML escaping and the fixed span overhead.
        assert!(rendered.len() <= 6 * source.len() + 40 * MAX_TOKENS);
        let mut samples = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            for _ in 0..200 {
                std::hint::black_box(html(std::hint::black_box(source), language));
                std::hint::black_box(decoration(std::hint::black_box(source), language));
            }
            samples.push(start.elapsed().as_secs_f64() * 1000.0 / 200.0);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "{}",
            serde_json::json!({"source_bytes":source.len(),"static_html_bytes":rendered.len(),
            "colored_spans":count,"live_metadata_bytes":metadata.to_string().len(),
            "html_plus_live_tokenize_ms_min_median_max":[samples[0],samples[2],samples[4]],"iterations":1000})
        );
    }
}
