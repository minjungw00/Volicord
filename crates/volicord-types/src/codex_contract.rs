use std::{collections::BTreeSet, fmt};

use serde::{
    de::{MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Number, Value};

pub(crate) fn u32be(value: usize) -> Result<[u8; 4], String> {
    let value = u32::try_from(value).map_err(|_| "canonical value length exceeds u32")?;
    Ok(value.to_be_bytes())
}

pub(crate) fn blob(value: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::with_capacity(4 + value.len());
    encoded.extend_from_slice(&u32be(value.len())?);
    encoded.extend_from_slice(value);
    Ok(encoded)
}

pub(crate) fn string(value: &str) -> Result<Vec<u8>, String> {
    blob(value.as_bytes())
}

pub(crate) fn list(items: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(items.len())?);
    for item in items {
        encoded.extend_from_slice(&blob(&item)?);
    }
    Ok(encoded)
}

pub(crate) fn record(fields: Vec<(&str, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&u32be(fields.len())?);
    for (name, value) in fields {
        encoded.extend_from_slice(&string(name)?);
        encoded.extend_from_slice(&blob(&value)?);
    }
    Ok(encoded)
}

pub(crate) fn nullable(value: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    match value {
        None => Ok(vec![0]),
        Some(value) => {
            let mut encoded = vec![1];
            encoded.extend_from_slice(&blob(&value)?);
            Ok(encoded)
        }
    }
}

pub(crate) fn require_exact_fields<'a>(
    value: &'a OrderedJsonValue,
    expected: &[&str],
    name: &str,
) -> Result<Vec<&'a OrderedJsonValue>, String> {
    let OrderedJsonValue::Object(fields) = value else {
        return Err(format!("{name} must be an object"));
    };
    let actual = fields
        .iter()
        .map(|(field, _)| field.as_str())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(format!(
            "{name} fields must be present exactly once in canonical order"
        ));
    }
    Ok(fields.iter().map(|(_, value)| value).collect())
}

pub(crate) fn parse_ordered_json(bytes: &[u8]) -> Result<OrderedJsonValue, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = OrderedJsonValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OrderedJsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl OrderedJsonValue {
    pub(crate) fn into_json(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::Array(values) => Value::Array(values.into_iter().map(Self::into_json).collect()),
            Self::Object(fields) => Value::Object(
                fields
                    .into_iter()
                    .map(|(name, value)| (name, value.into_json()))
                    .collect::<Map<_, _>>(),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

struct OrderedJsonVisitor;

impl<'de> Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object fields")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(OrderedJsonValue::Number)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(OrderedJsonValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(OrderedJsonValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut fields = Vec::new();
        while let Some((name, value)) = map.next_entry::<String, OrderedJsonValue>()? {
            if !seen.insert(name.clone()) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate JSON field {name}"
                )));
            }
            fields.push((name, value));
        }
        Ok(OrderedJsonValue::Object(fields))
    }
}
