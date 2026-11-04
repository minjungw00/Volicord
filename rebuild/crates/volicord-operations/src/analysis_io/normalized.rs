//! Request-owned decoding of the existing verified shape/value representation.
//! Container punctuation is consumed directly; scalars retain serde_json's
//! lexical/type checks. Plain strings are validated once per immutable symbol.
use super::{DenormalizingReader, Error};
use serde::de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::ops::Range;

type JsonError = serde_json::Error;
type Result<T> = std::result::Result<T, JsonError>;

fn corrupt(message: &'static str) -> JsonError {
    de::Error::custom(message)
}

#[derive(Clone, Copy)]
struct Symbol<'de> {
    raw: &'de [u8],
    plain: Option<&'de str>,
}

impl<'de> Symbol<'de> {
    fn new(raw: &'de [u8]) -> Result<Self> {
        if !matches!(
            raw.first(),
            Some(b'"' | b'-' | b'0'..=b'9' | b'n' | b't' | b'f')
        ) {
            return Err(corrupt("invalid normalized scalar"));
        }
        let plain = if raw.len() >= 2 && raw.first() == Some(&b'"') && raw.last() == Some(&b'"') {
            let inner = &raw[1..raw.len() - 1];
            if inner.iter().any(|b| *b < 0x20 || matches!(b, b'"' | b'\\')) {
                None
            } else {
                Some(std::str::from_utf8(inner).map_err(|_| corrupt("invalid normalized UTF-8"))?)
            }
        } else {
            None
        };
        Ok(Self { raw, plain })
    }
}

struct Cursor<'de> {
    shape: &'de [u8],
    values: &'de [u8],
    symbols: Vec<Symbol<'de>>,
    at: usize,
    reference: usize,
    bytes: u64,
    expected_bytes: u64,
    depth: u8,
}

impl<'de> Cursor<'de> {
    fn new(
        shape: &'de [u8],
        values: &'de [u8],
        ranges: &[Range<usize>],
        at: usize,
        reference: usize,
        expected_bytes: u64,
    ) -> Result<Self> {
        // Match the previous sentinel/length admission before decoding anything.
        expected_bytes
            .checked_add(1)
            .ok_or_else(|| corrupt("unsupported normalized length"))?;
        usize::try_from(expected_bytes).map_err(|_| corrupt("unsupported normalized length"))?;
        let symbols = ranges
            .iter()
            .map(|r| Symbol::new(&values[r.clone()]))
            .collect::<Result<_>>()?;
        Ok(Self {
            shape,
            values,
            symbols,
            at,
            reference,
            bytes: 0,
            expected_bytes,
            depth: 128,
        })
    }

    fn peek(&self) -> Option<u8> {
        self.shape.get(self.at).copied()
    }

    fn account(&mut self, count: usize) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(count as u64)
            .ok_or_else(|| corrupt("normalized length overflow"))?;
        if self.bytes > self.expected_bytes {
            return Err(corrupt("normalized length differs from manifest"));
        }
        Ok(())
    }

    fn punctuation(&mut self, expected: u8) -> Result<()> {
        if self.peek() != Some(expected) {
            return Err(corrupt("invalid normalized container punctuation"));
        }
        self.at += 1;
        self.account(1)
    }

    fn scalar(&mut self) -> Result<Symbol<'de>> {
        if self.peek() != Some(b'$') {
            return Err(corrupt("expected normalized scalar"));
        }
        let bytes = self
            .values
            .get(self.reference..)
            .and_then(|v| v.get(..4))
            .ok_or_else(|| corrupt("missing normalized scalar reference"))?;
        let index = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let symbol = *self
            .symbols
            .get(index)
            .ok_or_else(|| corrupt("invalid normalized scalar reference"))?;
        self.reference += 4;
        self.at += 1;
        self.account(symbol.raw.len())?;
        Ok(symbol)
    }

    fn enter(&mut self, token: u8) -> Result<()> {
        // serde_json's default limit rejects the 128th nested container.
        if self.depth <= 1 {
            return Err(corrupt("normalized recursion limit exceeded"));
        }
        self.punctuation(token)?;
        self.depth -= 1;
        Ok(())
    }

    fn array<V: Visitor<'de>>(&mut self, visitor: V) -> Result<V::Value> {
        self.enter(b'[')?;
        let result = visitor.visit_seq(Elements {
            cursor: self,
            first: true,
        });
        self.depth += 1;
        let value = result?;
        self.punctuation(b']')?;
        Ok(value)
    }

    fn object<V: Visitor<'de>>(&mut self, visitor: V, enumeration: bool) -> Result<V::Value> {
        self.enter(b'{')?;
        let entries = Entries {
            cursor: self,
            first: true,
        };
        let result = if enumeration {
            visitor.visit_enum(de::value::MapAccessDeserializer::new(entries))
        } else {
            visitor.visit_map(entries)
        };
        self.depth += 1;
        let value = result?;
        self.punctuation(b'}')?;
        Ok(value)
    }

    fn end(&self) -> Result<()> {
        if self.at != self.shape.len()
            || self.reference != self.values.len()
            || self.bytes != self.expected_bytes
        {
            return Err(corrupt("normalized trailing data or length mismatch"));
        }
        Ok(())
    }
}

struct Elements<'a, 'de> {
    cursor: &'a mut Cursor<'de>,
    first: bool,
}
impl<'de> SeqAccess<'de> for Elements<'_, 'de> {
    type Error = JsonError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.cursor.peek() == Some(b']') {
            return Ok(None);
        }
        if !self.first {
            self.cursor.punctuation(b',')?;
            if self.cursor.peek() == Some(b']') {
                return Err(corrupt("normalized trailing comma"));
            }
        }
        self.first = false;
        seed.deserialize(&mut *self.cursor).map(Some)
    }
}

struct Entries<'a, 'de> {
    cursor: &'a mut Cursor<'de>,
    first: bool,
}
impl<'de> MapAccess<'de> for Entries<'_, 'de> {
    type Error = JsonError;
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.cursor.peek() == Some(b'}') {
            return Ok(None);
        }
        if !self.first {
            self.cursor.punctuation(b',')?;
            if self.cursor.peek() == Some(b'}') {
                return Err(corrupt("normalized trailing comma"));
            }
        }
        self.first = false;
        let symbol = self.cursor.scalar()?;
        if symbol.raw.first() != Some(&b'"') {
            return Err(corrupt("normalized map key is not a string"));
        }
        seed.deserialize(Scalar { symbol, key: true }).map(Some)
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        self.cursor.punctuation(b':')?;
        seed.deserialize(&mut *self.cursor)
    }
}

#[derive(Clone, Copy)]
struct Scalar<'de> {
    symbol: Symbol<'de>,
    key: bool,
}

macro_rules! scalar_json {
    ($($method:ident),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            let raw = if self.key {
                self.symbol.raw.strip_prefix(b"\"").and_then(|s| s.strip_suffix(b"\""))
                    .ok_or_else(|| corrupt("invalid normalized numeric key"))?
            } else { self.symbol.raw };
            let mut parser = serde_json::Deserializer::from_slice(raw);
            let value = parser.$method(visitor)?;
            parser.end()?;
            Ok(value)
        }
    )*};
}

macro_rules! scalar_text {
    ($($method:ident),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            if let Some(text) = self.symbol.plain { return visitor.visit_borrowed_str(text); }
            let mut parser = serde_json::Deserializer::from_slice(self.symbol.raw);
            let value = parser.$method(visitor)?;
            parser.end()?;
            Ok(value)
        }
    )*};
}

impl<'de> de::Deserializer<'de> for Scalar<'de> {
    type Error = JsonError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if let Some(text) = self.symbol.plain {
            return visitor.visit_borrowed_str(text);
        }
        let mut parser = serde_json::Deserializer::from_slice(self.symbol.raw);
        let value = parser.deserialize_any(visitor)?;
        parser.end()?;
        Ok(value)
    }
    scalar_json!(
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64
    );
    scalar_text!(
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_identifier
    );
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.key {
            return self.deserialize_any(visitor);
        }
        let mut parser = serde_json::Deserializer::from_slice(self.symbol.raw);
        let value = parser.deserialize_unit(visitor)?;
        parser.end()?;
        Ok(value)
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if !self.key && self.symbol.raw == b"null" {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        if let Some(text) = self.symbol.plain {
            return de::value::BorrowedStrDeserializer::<JsonError>::new(text)
                .deserialize_enum(name, variants, visitor);
        }
        let mut parser = serde_json::Deserializer::from_slice(self.symbol.raw);
        let value = parser.deserialize_enum(name, variants, visitor)?;
        parser.end()?;
        Ok(value)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if let Some(text) = self.symbol.plain {
            return visitor.visit_borrowed_bytes(text.as_bytes());
        }
        let mut parser = serde_json::Deserializer::from_slice(self.symbol.raw);
        let value = parser.deserialize_bytes(visitor)?;
        parser.end()?;
        Ok(value)
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }
    serde::forward_to_deserialize_any! {
        seq tuple tuple_struct map struct ignored_any
    }
}

macro_rules! cursor_scalar {
    ($($method:ident),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            Scalar { symbol: self.scalar()?, key: false }.$method(visitor)
        }
    )*};
}

impl<'de> de::Deserializer<'de> for &mut Cursor<'de> {
    type Error = JsonError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.peek() {
            Some(b'[') => self.array(visitor),
            Some(b'{') => self.object(visitor, false),
            Some(b'$') => Scalar {
                symbol: self.scalar()?,
                key: false,
            }
            .deserialize_any(visitor),
            _ => Err(corrupt("invalid normalized value")),
        }
    }
    cursor_scalar!(
        deserialize_bool,
        deserialize_i8,
        deserialize_i16,
        deserialize_i32,
        deserialize_i64,
        deserialize_i128,
        deserialize_u8,
        deserialize_u16,
        deserialize_u32,
        deserialize_u64,
        deserialize_u128,
        deserialize_f32,
        deserialize_f64,
        deserialize_char,
        deserialize_str,
        deserialize_string,
        deserialize_unit,
        deserialize_identifier
    );
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.peek() == Some(b'$') {
            let bytes = self
                .values
                .get(self.reference..)
                .and_then(|s| s.get(..4))
                .ok_or_else(|| corrupt("missing normalized scalar reference"))?;
            let index = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            if self.symbols.get(index).is_some_and(|s| s.raw == b"null") {
                self.scalar()?;
                return visitor.visit_none();
            }
        }
        visitor.visit_some(self)
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.array(visitor)
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value> {
        self.array(visitor)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.array(visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.object(visitor, false)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        if self.peek() == Some(b'[') {
            self.array(visitor)
        } else {
            self.object(visitor, false)
        }
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        if self.peek() == Some(b'{') {
            self.object(visitor, true)
        } else {
            Scalar {
                symbol: self.scalar()?,
                key: false,
            }
            .deserialize_enum(name, variants, visitor)
        }
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.peek() == Some(b'[') {
            self.array(visitor)
        } else {
            Scalar {
                symbol: self.scalar()?,
                key: false,
            }
            .deserialize_bytes(visitor)
        }
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }
}

pub(super) fn decode<T: DeserializeOwned>(
    reader: DenormalizingReader,
    logical_bytes: u64,
) -> std::result::Result<T, Error> {
    let DenormalizingReader {
        shape,
        values,
        symbols,
        shape_at,
        reference_at,
        pending,
    } = reader;
    if pending.is_some() {
        return Err(Error::new("normalized reader was already consumed"));
    }
    let run = || -> Result<T> {
        let mut cursor = Cursor::new(
            &shape,
            &values,
            &symbols,
            shape_at,
            reference_at,
            logical_bytes,
        )?;
        let value = T::deserialize(&mut cursor)?;
        cursor.end()?;
        Ok(value)
    };
    run().map_err(|e| Error::with_source("normalized Analysis Snapshot is corrupt", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_io::{Normalized, NormalizingWriter};
    use serde::{Deserialize, Serialize};
    use std::{collections::BTreeMap, fmt::Debug, io::Write};

    fn normalized(input: &[u8]) -> std::result::Result<Normalized, Error> {
        let mut writer = NormalizingWriter::default();
        writer
            .write_all(input)
            .map_err(|e| Error::with_source("fixture normalization failed", e))?;
        writer.finish()
    }
    fn decode_bytes<T: DeserializeOwned>(
        input: &[u8],
        length: u64,
    ) -> std::result::Result<T, Error> {
        let value = normalized(input)?;
        decode(
            DenormalizingReader::new(value.shape, value.values, value.scalar_count)?,
            length,
        )
    }
    fn roundtrip<T: DeserializeOwned + Serialize + PartialEq + Debug>(
        value: T,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let original = serde_json::to_vec(&value)?;
        let expected: T = serde_json::from_slice(&original)?;
        assert_eq!(
            decode_bytes::<T>(&original, original.len() as u64)?,
            expected
        );
        Ok(())
    }

    #[test]
    fn direct_values_match_independent_json_including_escapes_numbers_and_ignored_fields(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        roundtrip(
            serde_json::json!({"plain":"한글 λ", "escaped":"\0\n\r\t\"\\", "long":"abc한글".repeat(100_000),
            "values":[true,false,null,0,-1,1.25,1e100,-0.0], "nested":[{},[],{"x":null}]}),
        )?;
        roundtrip((
            u128::MAX,
            i128::MIN,
            u64::MAX,
            i64::MIN,
            Some(false),
            None::<String>,
        ))?;
        roundtrip(BTreeMap::from([
            (i128::MIN, vec![Some(u128::MAX)]),
            (0, vec![None]),
        ]))?;
        roundtrip(BTreeMap::from([
            (true, "yes".to_owned()),
            (false, "no".to_owned()),
        ]))?;
        #[derive(Debug, Deserialize, PartialEq)]
        struct Known {
            name: String,
        }
        let input = br#"{"unknown":{"array":[null,{"x":"y"}]},"name":"ok"}"#;
        assert_eq!(
            decode_bytes::<Known>(input, input.len() as u64)?,
            Known { name: "ok".into() }
        );
        for input in [br#""\uD83C\uDF0D""#.as_slice(), br#""\u0061\t\n""#] {
            assert_eq!(
                decode_bytes::<String>(input, input.len() as u64)?,
                serde_json::from_slice::<String>(input)?
            );
        }
        Ok(())
    }

    #[test]
    fn direct_enums_and_struct_forms_keep_serde_domain_semantics(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        enum External {
            Unit,
            New(String),
            Tuple(u64, Option<String>),
            Record { value: bool },
        }
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(tag = "kind", content = "value")]
        enum Adjacent {
            Unit,
            Values(Vec<External>),
        }
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(tag = "kind")]
        enum Internal {
            Record { nested: Adjacent },
        }
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Untagged {
            Text(String),
            Number(u64),
            Record(Internal),
        }
        roundtrip(Untagged::Record(Internal::Record {
            nested: Adjacent::Values(vec![
                External::Unit,
                External::New("x\ny".into()),
                External::Tuple(3, None),
                External::Record { value: true },
            ]),
        }))?;
        roundtrip(Untagged::Text("text".into()))?;
        roundtrip(Untagged::Number(42))?;
        #[derive(Debug, Deserialize, PartialEq)]
        struct Pair {
            first: String,
            second: u64,
        }
        let input = br#"["value",12]"#;
        assert_eq!(
            decode_bytes::<Pair>(input, input.len() as u64)?,
            serde_json::from_slice::<Pair>(input)?
        );
        Ok(())
    }

    #[test]
    fn direct_decoder_rejects_bad_grammar_lengths_and_the_same_depth_boundary(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        for input in [
            b"[1,]".as_slice(),
            b"{\"a\":1,}",
            b"{1:2}",
            b"[1 2]",
            b"{\"a\" 1}",
            b"[]true",
            b"01",
            b"--1",
            b"1.",
            b"1e",
            b"truefalse",
            br#""bad\q""#,
            br#""\uD800""#,
        ] {
            assert!(serde_json::from_slice::<serde_json::Value>(input).is_err());
            assert!(
                decode_bytes::<serde_json::Value>(input, input.len() as u64).is_err(),
                "{input:?}"
            );
        }
        for length in [1, 3, u64::MAX] {
            assert!(decode_bytes::<Vec<String>>(b"[]", length).is_err());
        }
        for depth in [1, 126, 127, 128, 129] {
            let input = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
            let expected = serde_json::from_str::<serde_json::Value>(&input);
            let actual = decode_bytes::<serde_json::Value>(input.as_bytes(), input.len() as u64);
            assert_eq!(actual.is_ok(), expected.is_ok(), "depth {depth}");
        }
        Ok(())
    }

    #[test]
    fn typed_prefix_does_not_require_json_expansion_and_cannot_admit_a_partial_document(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        #[derive(Debug, PartialEq)]
        struct First(String);
        impl<'de> Deserialize<'de> for First {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                struct FirstVisitor;
                impl<'de> Visitor<'de> for FirstVisitor {
                    type Value = First;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("a sequence")
                    }
                    fn visit_seq<A: SeqAccess<'de>>(
                        self,
                        mut seq: A,
                    ) -> std::result::Result<First, A::Error> {
                        Ok(First(
                            seq.next_element()?
                                .ok_or_else(|| de::Error::custom("missing first"))?,
                        ))
                    }
                }
                d.deserialize_seq(FirstVisitor)
            }
        }
        let input = serde_json::to_vec(&vec!["first".to_owned(), "tail 한글".repeat(512 * 1024)])?;
        assert!(decode_bytes::<First>(&input, input.len() as u64).is_err());
        // Observe the same cursor's sequence access: one typed value is available
        // while references and the rest of the document remain unconsumed.
        let n = normalized(&input)?;
        let reader = DenormalizingReader::new(n.shape, n.values, n.scalar_count)?;
        let mut cursor = Cursor::new(
            &reader.shape,
            &reader.values,
            &reader.symbols,
            reader.shape_at,
            reader.reference_at,
            input.len() as u64,
        )?;
        cursor.punctuation(b'[')?;
        let mut seq = Elements {
            cursor: &mut cursor,
            first: true,
        };
        assert_eq!(seq.next_element::<String>()?, Some("first".into()));
        assert!(cursor.bytes < input.len() as u64 && cursor.reference < cursor.values.len());
        assert!(cursor.end().is_err());
        Ok(())
    }

    #[test]
    fn typed_scalar_requests_do_not_coerce_values_through_permissive_visitors() {
        #[derive(Debug)]
        struct Text;
        impl<'de> Deserialize<'de> for Text {
            fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                struct Permissive;
                impl<'de> Visitor<'de> for Permissive {
                    type Value = Text;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("text")
                    }
                    fn visit_u64<E: de::Error>(self, _: u64) -> std::result::Result<Text, E> {
                        Ok(Text)
                    }
                }
                d.deserialize_string(Permissive)
            }
        }
        assert!(serde_json::from_slice::<Text>(b"12").is_err());
        assert!(decode_bytes::<Text>(b"12", 2).is_err());
        for input in [b"true".as_slice(), b"12", br#""text""#] {
            assert!(decode_bytes::<()>(input, input.len() as u64).is_err());
        }
    }

    #[test]
    fn generated_mixed_values_match_the_independent_json_decoder(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        fn next(state: &mut u64) -> u64 {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            *state
        }
        fn value(state: &mut u64, depth: u8) -> serde_json::Value {
            let n = next(state);
            match n % if depth == 0 { 6 } else { 8 } {
                0 => serde_json::Value::Null,
                1 => serde_json::json!(n & 1 != 0),
                2 => serde_json::json!(n),
                3 => serde_json::json!(n as i64),
                4 => serde_json::json!(format!("한글 {n:x} \n\t\0\\\" λ")),
                5 => serde_json::json!((n as f64) / 13.0),
                6 => {
                    serde_json::Value::Array((0..n % 5).map(|_| value(state, depth - 1)).collect())
                }
                _ => serde_json::Value::Object(
                    (0..n % 5)
                        .map(|i| (format!("key {i} λ"), value(state, depth - 1)))
                        .collect(),
                ),
            }
        }
        let mut seed = 0x315b_06a3_eb80_d541;
        for _ in 0..256 {
            roundtrip(value(&mut seed, 5))?;
        }
        Ok(())
    }
}
