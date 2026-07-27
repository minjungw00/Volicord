use bufjson::{
    lexical::{fixed::FixedAnalyzer, Token},
    syntax::Parser as JsonParser,
};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use yaml_rust2::{
    parser::{Event, Parser as YamlParser, Tag},
    scanner::{Marker, TScalarStyle},
};

type FixedJsonParser<'a> = JsonParser<FixedAnalyzer<&'a [u8]>>;
type PullYamlParser<'a> = YamlParser<std::str::Chars<'a>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SourcePosition {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum StructuredParseError {
    DuplicateKey {
        instance_path: String,
        key: String,
        first: SourcePosition,
        repeated: SourcePosition,
    },
    Invalid {
        instance_path: String,
        position: Option<SourcePosition>,
        message: String,
    },
}

pub(crate) fn parse(language: &str, source: &str) -> Result<Value, StructuredParseError> {
    match language {
        "json" => parse_json(source),
        "yaml" | "yml" => parse_yaml(source),
        _ => Err(StructuredParseError::Invalid {
            instance_path: "/".to_owned(),
            position: None,
            message: "unsupported structured fence language".to_owned(),
        }),
    }
}

fn parse_json(source: &str) -> Result<Value, StructuredParseError> {
    let mut parser = FixedAnalyzer::new(source.as_bytes()).into_parser();
    let (token, position) = next_json(&mut parser, "")?;
    let value = parse_json_value(&mut parser, token, position, "")?;
    let (token, position) = next_json(&mut parser, "")?;
    if token != Token::Eof {
        return Err(invalid(
            "",
            Some(position),
            "JSON document contains content after its root value",
        ));
    }
    Ok(value)
}

fn parse_json_value(
    parser: &mut FixedJsonParser<'_>,
    token: Token,
    position: SourcePosition,
    path: &str,
) -> Result<Value, StructuredParseError> {
    match token {
        Token::LitNull => Ok(Value::Null),
        Token::LitTrue => Ok(Value::Bool(true)),
        Token::LitFalse => Ok(Value::Bool(false)),
        Token::Num => parser
            .content()
            .literal()
            .parse::<Number>()
            .map(Value::Number)
            .map_err(|error| {
                invalid(
                    path,
                    Some(position),
                    format!("invalid JSON number: {error}"),
                )
            }),
        Token::Str => Ok(Value::String(json_string(parser))),
        Token::ArrBegin => parse_json_array(parser, path),
        Token::ObjBegin => parse_json_object(parser, path),
        Token::Err => unreachable!("JSON errors are handled by next_json"),
        Token::Eof => Err(invalid(
            path,
            Some(position),
            "JSON document ended before a value",
        )),
        _ => Err(invalid(
            path,
            Some(position),
            "JSON parser produced an unexpected structural token",
        )),
    }
}

fn parse_json_array(
    parser: &mut FixedJsonParser<'_>,
    path: &str,
) -> Result<Value, StructuredParseError> {
    let mut array = Vec::new();
    loop {
        let (token, position) = next_json(parser, path)?;
        if token == Token::ArrEnd {
            return Ok(Value::Array(array));
        }
        let child_path = append_path(path, &array.len().to_string());
        array.push(parse_json_value(parser, token, position, &child_path)?);
    }
}

fn parse_json_object(
    parser: &mut FixedJsonParser<'_>,
    path: &str,
) -> Result<Value, StructuredParseError> {
    let mut object = Map::new();
    let mut positions = BTreeMap::new();
    loop {
        let (token, position) = next_json(parser, path)?;
        if token == Token::ObjEnd {
            return Ok(Value::Object(object));
        }
        if token != Token::Str {
            return Err(invalid(
                path,
                Some(position),
                "JSON object member name must be a string",
            ));
        }
        let key = json_string(parser);
        if let Some(first) = positions.get(&key) {
            return Err(StructuredParseError::DuplicateKey {
                instance_path: display_path(path),
                key,
                first: *first,
                repeated: position,
            });
        }
        positions.insert(key.clone(), position);

        let child_path = append_path(path, &key);
        let (value_token, value_position) = next_json(parser, &child_path)?;
        let value = parse_json_value(parser, value_token, value_position, &child_path)?;
        object.insert(key, value);
    }
}

fn next_json(
    parser: &mut FixedJsonParser<'_>,
    path: &str,
) -> Result<(Token, SourcePosition), StructuredParseError> {
    let token = parser.next_meaningful();
    if token == Token::Err {
        let error = parser.err();
        let position = SourcePosition {
            line: error.pos().line,
            column: error.pos().col,
        };
        return Err(invalid(path, Some(position), error.kind().to_string()));
    }
    let position = parser.pos();
    Ok((
        token,
        SourcePosition {
            line: position.line,
            column: position.col,
        },
    ))
}

fn json_string(parser: &FixedJsonParser<'_>) -> String {
    let unescaped = String::from(parser.content().unescaped());
    unescaped
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("bufjson string token includes quotes")
        .to_owned()
}

fn parse_yaml(source: &str) -> Result<Value, StructuredParseError> {
    let mut parser = YamlParser::new_from_str(source);
    expect_yaml_event(&mut parser, Event::StreamStart, "")?;
    expect_yaml_event(&mut parser, Event::DocumentStart, "")?;
    let (event, marker) = next_yaml(&mut parser, "")?;
    let value = parse_yaml_value(&mut parser, event, marker, "")?;
    expect_yaml_event(&mut parser, Event::DocumentEnd, "")?;
    let (event, marker) = next_yaml(&mut parser, "")?;
    if event != Event::StreamEnd {
        return Err(invalid(
            "",
            Some(yaml_position(marker)),
            "YAML contract examples must contain exactly one document",
        ));
    }
    Ok(value)
}

fn parse_yaml_value(
    parser: &mut PullYamlParser<'_>,
    event: Event,
    marker: Marker,
    path: &str,
) -> Result<Value, StructuredParseError> {
    let position = yaml_position(marker);
    match event {
        Event::Scalar(value, style, anchor, tag) => {
            reject_yaml_node_features(path, position, anchor, tag.as_ref())?;
            parse_yaml_scalar(&value, style, path, position)
        }
        Event::SequenceStart(anchor, tag) => {
            reject_yaml_node_features(path, position, anchor, tag.as_ref())?;
            parse_yaml_sequence(parser, path)
        }
        Event::MappingStart(anchor, tag) => {
            reject_yaml_node_features(path, position, anchor, tag.as_ref())?;
            parse_yaml_mapping(parser, path)
        }
        Event::Alias(_) => Err(invalid(
            path,
            Some(position),
            "YAML aliases are not supported by JSON contracts",
        )),
        Event::SequenceEnd | Event::MappingEnd | Event::DocumentEnd | Event::StreamEnd => Err(
            invalid(path, Some(position), "YAML document ended before a value"),
        ),
        Event::Nothing | Event::StreamStart | Event::DocumentStart => Err(invalid(
            path,
            Some(position),
            "YAML parser produced an unexpected structural event",
        )),
    }
}

fn parse_yaml_sequence(
    parser: &mut PullYamlParser<'_>,
    path: &str,
) -> Result<Value, StructuredParseError> {
    let mut array = Vec::new();
    loop {
        let (event, marker) = next_yaml(parser, path)?;
        if event == Event::SequenceEnd {
            return Ok(Value::Array(array));
        }
        let child_path = append_path(path, &array.len().to_string());
        array.push(parse_yaml_value(parser, event, marker, &child_path)?);
    }
}

fn parse_yaml_mapping(
    parser: &mut PullYamlParser<'_>,
    path: &str,
) -> Result<Value, StructuredParseError> {
    let mut object = Map::new();
    let mut positions = BTreeMap::new();
    loop {
        let (key_event, key_marker) = next_yaml(parser, path)?;
        if key_event == Event::MappingEnd {
            return Ok(Value::Object(object));
        }
        let key_position = yaml_position(key_marker);
        let (key, merge_key) = parse_yaml_key(key_event, key_position, path)?;
        if merge_key {
            return Err(invalid(
                path,
                Some(key_position),
                "YAML merge keys are not supported by JSON contracts",
            ));
        }
        if let Some(first) = positions.get(&key) {
            return Err(StructuredParseError::DuplicateKey {
                instance_path: display_path(path),
                key,
                first: *first,
                repeated: key_position,
            });
        }
        positions.insert(key.clone(), key_position);

        let child_path = append_path(path, &key);
        let (value_event, value_marker) = next_yaml(parser, &child_path)?;
        let value = parse_yaml_value(parser, value_event, value_marker, &child_path)?;
        object.insert(key, value);
    }
}

fn parse_yaml_key(
    event: Event,
    position: SourcePosition,
    path: &str,
) -> Result<(String, bool), StructuredParseError> {
    let Event::Scalar(value, style, anchor, tag) = event else {
        return match event {
            Event::Alias(_) => Err(invalid(
                path,
                Some(position),
                "YAML aliases are not supported by JSON contracts",
            )),
            _ => Err(invalid(
                path,
                Some(position),
                "YAML mapping keys must be strings for JSON compatibility",
            )),
        };
    };
    reject_yaml_node_features(path, position, anchor, tag.as_ref())?;
    let merge_key = style == TScalarStyle::Plain && value == "<<";
    let value = parse_yaml_scalar(&value, style, path, position)?;
    let Value::String(key) = value else {
        return Err(invalid(
            path,
            Some(position),
            "YAML mapping keys must be strings for JSON compatibility",
        ));
    };
    Ok((key, merge_key))
}

fn parse_yaml_scalar(
    value: &str,
    style: TScalarStyle,
    path: &str,
    position: SourcePosition,
) -> Result<Value, StructuredParseError> {
    if style != TScalarStyle::Plain {
        return Ok(Value::String(value.to_owned()));
    }
    match value {
        "" | "~" | "null" | "Null" | "NULL" => return Ok(Value::Null),
        "true" | "True" | "TRUE" => return Ok(Value::Bool(true)),
        "false" | "False" | "FALSE" => return Ok(Value::Bool(false)),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" | "-.inf" | "-.Inf" | "-.INF"
        | ".nan" | ".NaN" | ".NAN" => {
            return Err(invalid(
                path,
                Some(position),
                "YAML non-finite numbers are not supported by JSON contracts",
            ));
        }
        _ => {}
    }
    if let Some(number) = parse_yaml_integer(value) {
        return Ok(Value::Number(number));
    }
    if !digits_but_not_number(value) {
        let number = value.strip_prefix('+').unwrap_or(value);
        if let Ok(number) = number.parse::<f64>() {
            if let Some(number) = Number::from_f64(number) {
                return Ok(Value::Number(number));
            }
        }
    }
    Ok(Value::String(value.to_owned()))
}

fn parse_yaml_integer(value: &str) -> Option<Number> {
    if digits_but_not_number(value) {
        return None;
    }
    let positive = value.strip_prefix('+').unwrap_or(value);
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = positive.strip_prefix(prefix) {
            return u64::from_str_radix(digits, radix).ok().map(Number::from);
        }
        let negative_prefix = format!("-{prefix}");
        if let Some(digits) = value.strip_prefix(&negative_prefix) {
            return i64::from_str_radix(&format!("-{digits}"), radix)
                .ok()
                .map(Number::from);
        }
    }
    if let Some(negative) = value.strip_prefix('-') {
        if negative.bytes().all(|byte| byte.is_ascii_digit()) {
            return value.parse::<i64>().ok().map(Number::from);
        }
    } else if positive.bytes().all(|byte| byte.is_ascii_digit()) {
        return positive.parse::<u64>().ok().map(Number::from);
    }
    None
}

fn digits_but_not_number(value: &str) -> bool {
    let unsigned = value.strip_prefix(['-', '+']).unwrap_or(value);
    unsigned.len() > 1
        && unsigned.starts_with('0')
        && unsigned[1..].bytes().all(|byte| byte.is_ascii_digit())
}

fn reject_yaml_node_features(
    path: &str,
    position: SourcePosition,
    anchor: usize,
    tag: Option<&Tag>,
) -> Result<(), StructuredParseError> {
    if let Some(tag) = tag {
        return Err(invalid(
            path,
            Some(position),
            format!(
                "YAML tag {}{} is not supported by JSON contracts",
                tag.handle, tag.suffix
            ),
        ));
    }
    if anchor != 0 {
        return Err(invalid(
            path,
            Some(position),
            "YAML anchors are not supported by JSON contracts",
        ));
    }
    Ok(())
}

fn expect_yaml_event(
    parser: &mut PullYamlParser<'_>,
    expected: Event,
    path: &str,
) -> Result<(), StructuredParseError> {
    let (actual, marker) = next_yaml(parser, path)?;
    if actual != expected {
        return Err(invalid(
            path,
            Some(yaml_position(marker)),
            "YAML parser produced an unexpected document boundary",
        ));
    }
    Ok(())
}

fn next_yaml(
    parser: &mut PullYamlParser<'_>,
    path: &str,
) -> Result<(Event, Marker), StructuredParseError> {
    parser.next_token().map_err(|error| {
        invalid(
            path,
            Some(yaml_position(*error.marker())),
            error.info().to_owned(),
        )
    })
}

fn yaml_position(marker: Marker) -> SourcePosition {
    SourcePosition {
        line: marker.line(),
        column: marker.col() + 1,
    }
}

fn append_path(path: &str, segment: &str) -> String {
    format!("{path}/{}", segment.replace('~', "~0").replace('/', "~1"))
}

fn display_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

fn invalid(
    path: &str,
    position: Option<SourcePosition>,
    message: impl Into<String>,
) -> StructuredParseError {
    StructuredParseError::Invalid {
        instance_path: display_path(path),
        position,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn duplicate(error: StructuredParseError) -> (String, String, SourcePosition, SourcePosition) {
        let StructuredParseError::DuplicateKey {
            instance_path,
            key,
            first,
            repeated,
        } = error
        else {
            panic!("expected duplicate key error, got {error:?}");
        };
        (instance_path, key, first, repeated)
    }

    #[test]
    fn json_rejects_top_level_and_nested_duplicates_with_source_positions() {
        let top = parse("json", "{\n  \"entry\": 1,\n  \"entry\": 2\n}").unwrap_err();
        assert_eq!(
            duplicate(top),
            (
                "/".to_owned(),
                "entry".to_owned(),
                SourcePosition { line: 2, column: 3 },
                SourcePosition { line: 3, column: 3 },
            )
        );

        let nested = parse("json", "{\"outer\":{\"entry\":1,\"entry\":2}}").unwrap_err();
        assert_eq!(duplicate(nested).0, "/outer");
    }

    #[test]
    fn json_rejects_duplicates_inside_objects_in_arrays() {
        let error = parse("json", "{\"items\":[{\"entry\":1,\"entry\":2}]}").unwrap_err();
        assert_eq!(duplicate(error).0, "/items/0");
    }

    #[test]
    fn yaml_rejects_top_level_and_nested_duplicates_with_source_positions() {
        let top = parse("yaml", "entry: 1\nentry: 2\n").unwrap_err();
        assert_eq!(
            duplicate(top),
            (
                "/".to_owned(),
                "entry".to_owned(),
                SourcePosition { line: 1, column: 1 },
                SourcePosition { line: 2, column: 1 },
            )
        );

        let nested = parse("yaml", "outer:\n  entry: 1\n  entry: 2\n").unwrap_err();
        assert_eq!(duplicate(nested).0, "/outer");
    }

    #[test]
    fn yaml_rejects_duplicates_inside_mappings_in_sequences() {
        let error = parse("yaml", "items:\n  - entry: 1\n    entry: 2\n").unwrap_err();
        assert_eq!(duplicate(error).0, "/items/0");
    }

    #[test]
    fn matching_keys_in_distinct_sibling_objects_are_valid() {
        for (language, source) in [
            ("json", "{\"left\":{\"entry\":1},\"right\":{\"entry\":2}}"),
            ("yaml", "left:\n  entry: 1\nright:\n  entry: 2\n"),
        ] {
            assert!(parse(language, source).is_ok(), "{language}");
        }
    }

    #[test]
    fn yaml_rejects_non_string_keys_anchors_aliases_and_merge_keys() {
        for (source, expected) in [
            ("1: value\n", "mapping keys must be strings"),
            ("entry: &shared value\n", "anchors"),
            ("entry: *shared\n", "unknown anchor"),
            ("<<: {entry: value}\n", "merge keys"),
        ] {
            let error = parse("yaml", source).unwrap_err();
            let StructuredParseError::Invalid { message, .. } = error else {
                panic!("expected invalid YAML error");
            };
            assert!(message.contains(expected), "{message}");
        }
        assert_eq!(
            parse("yaml", "\"<<\": value\n").unwrap(),
            json!({"<<": "value"})
        );
    }

    #[test]
    fn valid_unique_json_and_yaml_materialize_the_same_instance() {
        let expected = json!({
            "entry": "value",
            "enabled": true,
            "items": [1, null],
            "nested": {"count": 2}
        });
        assert_eq!(
            parse(
                "json",
                r#"{"entry":"value","enabled":true,"items":[1,null],"nested":{"count":2}}"#
            )
            .unwrap(),
            expected
        );
        assert_eq!(
            parse(
                "yaml",
                "entry: value\nenabled: true\nitems: [1, null]\nnested:\n  count: 2\n"
            )
            .unwrap(),
            expected
        );
    }
}
