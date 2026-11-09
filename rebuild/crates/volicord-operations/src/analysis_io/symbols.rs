//! Lossless scalar-slot planning. Field positions affect physical locality only;
//! they never identify entities, interpret facts or change a serialized value.
use super::{DenormalizingReader, Error, PendingValues};
use std::{
    collections::{HashMap, VecDeque},
    io::{BufReader, BufWriter, Read, Write},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct Position {
    owner: u64,
    field: u64,
    category: u8,
}

enum Frame {
    Object { owner: u64, field: u64, key: bool },
    Array { owner: u64 },
}

#[derive(Default)]
pub(super) struct Tracker {
    frames: Vec<Frame>,
}

fn key_tag(bytes: &[u8]) -> u64 {
    // Collisions can only reduce compression: the reference permutation is
    // checked independently and still points to the exact current scalar.
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn category(bytes: &[u8]) -> u8 {
    if bytes.first() == Some(&b'"') && bytes.last() == Some(&b'"') {
        let value = &bytes[1..bytes.len() - 1];
        let tail = value.rsplit(|byte| *byte == b':').next().unwrap_or(value);
        if tail.iter().all(u8::is_ascii_hexdigit) {
            if tail.len() == 32 {
                return 2;
            }
            if tail.len() == 64 {
                return 3;
            }
        }
        1
    } else if bytes
        .first()
        .is_some_and(|b| b.is_ascii_digit() || *b == b'-')
    {
        4
    } else {
        5
    }
}

impl Tracker {
    pub(super) fn punctuation(&mut self, byte: u8) {
        let owner = match self.frames.last() {
            Some(Frame::Object { field, .. }) => *field,
            Some(Frame::Array { owner }) => *owner,
            None => 0,
        };
        match byte {
            b'{' => self.frames.push(Frame::Object {
                owner,
                field: 0,
                key: true,
            }),
            b'[' => self.frames.push(Frame::Array { owner }),
            b'}' | b']' => {
                self.frames.pop();
            }
            b',' => {
                if let Some(Frame::Object { key, .. }) = self.frames.last_mut() {
                    *key = true;
                }
            }
            _ => {}
        }
    }

    pub(super) fn scalar(&mut self, bytes: &[u8]) -> Position {
        match self.frames.last_mut() {
            Some(Frame::Object { owner, field, key }) if *key => {
                let result = Position {
                    owner: *owner,
                    field: 0,
                    category: 0,
                };
                *field = key_tag(bytes);
                *key = false;
                result
            }
            Some(Frame::Object { owner, field, .. }) => Position {
                owner: *owner,
                field: *field,
                category: category(bytes),
            },
            Some(Frame::Array { owner }) => Position {
                owner: *owner,
                field: 0,
                category: category(bytes),
            },
            None => Position {
                owner: 0,
                field: 0,
                category: category(bytes),
            },
        }
    }
}

pub(super) struct Dictionary {
    symbols: Vec<Rc<[u8]>>,
    index: HashMap<Rc<[u8]>, u32>,
    positions: Vec<Option<Position>>,
}

impl Dictionary {
    pub(super) fn from_reader(reader: DenormalizingReader) -> Result<Self, Error> {
        let mut tracker = Tracker::default();
        let mut positions = vec![None; reader.symbols.len()];
        let mut at = reader.reference_at;
        for &byte in &reader.shape[reader.shape_at..] {
            if byte == b'$' {
                let raw = reader
                    .values
                    .get(at..at + 4)
                    .ok_or_else(|| Error::new("Analysis symbol position reference is truncated"))?;
                let index = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
                at += 4;
                let range = reader
                    .symbols
                    .get(index)
                    .ok_or_else(|| Error::new("Analysis symbol position is invalid"))?;
                let position = tracker.scalar(&reader.values[range.clone()]);
                if positions[index].is_none() {
                    positions[index] = Some(position);
                }
            } else {
                tracker.punctuation(byte);
            }
        }
        if at != reader.values.len() {
            return Err(Error::new(
                "Analysis symbol positions have trailing references",
            ));
        }
        let mut symbols = Vec::with_capacity(reader.symbols.len());
        let mut index = HashMap::with_capacity(reader.symbols.len());
        for range in reader.symbols {
            let raw = &reader.values[range];
            if !matches!(
                raw.first(),
                Some(b'"' | b'-' | b'0'..=b'9' | b'n' | b't' | b'f')
            ) {
                return Err(Error::new("Analysis base dictionary contains a non-scalar"));
            }
            serde_json::from_slice::<serde::de::IgnoredAny>(raw)
                .map_err(|error| Error::with_source("Analysis base scalar is corrupt", error))?;
            let symbol = Rc::<[u8]>::from(raw);
            let ordinal = u32::try_from(symbols.len())
                .map_err(|error| Error::with_source("Analysis dictionary is too large", error))?;
            if index.insert(Rc::clone(&symbol), ordinal).is_some() {
                return Err(Error::new("Analysis base contains duplicate symbols"));
            }
            symbols.push(symbol);
        }
        Ok(Self {
            symbols,
            index,
            positions,
        })
    }

    pub(super) fn apply(mut self, current: PendingValues) -> Result<PendingValues, Error> {
        let mut reserved = vec![false; self.symbols.len()];
        for symbol in &current.symbols {
            if let Some(&index) = self.index.get(symbol.as_ref()) {
                reserved[index as usize] = true;
            }
        }
        let mut holes = HashMap::<Position, VecDeque<u32>>::new();
        for (index, position) in self.positions.iter().enumerate() {
            if !reserved[index] {
                if let Some(position) = position {
                    holes.entry(*position).or_default().push_back(index as u32);
                }
            }
        }
        let mut mapping = Vec::with_capacity(current.symbols.len());
        for (index, symbol) in current.symbols.into_iter().enumerate() {
            let slot = if let Some(&slot) = self.index.get(symbol.as_ref()) {
                slot
            } else if let Some(slot) = current
                .positions
                .get(index)
                .copied()
                .flatten()
                .and_then(|position| holes.get_mut(&position))
                .and_then(VecDeque::pop_front)
            {
                self.symbols[slot as usize] = symbol;
                slot
            } else {
                let slot = u32::try_from(self.symbols.len()).map_err(|error| {
                    Error::with_source("Analysis dictionary is too large", error)
                })?;
                self.symbols.push(symbol);
                slot
            };
            mapping.push(slot);
        }
        drop((self.index, self.positions, holes, reserved));
        let packed_references = if let Some(packed) = current.packed_references {
            let mut reader = BufReader::with_capacity(
                64 * 1024,
                zstd::stream::read::Decoder::new(packed.as_slice()).map_err(|error| {
                    Error::with_source("cannot open Analysis references for slot planning", error)
                })?,
            );
            let mut writer = BufWriter::with_capacity(
                64 * 1024,
                zstd::stream::write::Encoder::new(Vec::new(), 1).map_err(|error| {
                    Error::with_source("cannot encode planned Analysis references", error)
                })?,
            );
            let mut remaining = current
                .scalar_count
                .checked_mul(4)
                .ok_or_else(|| Error::new("Analysis reference length is unsupported"))?;
            let mut chunk = [0; 64 * 1024];
            while remaining != 0 {
                let length = remaining.min(chunk.len() as u64) as usize;
                reader.read_exact(&mut chunk[..length]).map_err(|error| {
                    Error::with_source("Analysis references are truncated", error)
                })?;
                for raw in chunk[..length].chunks_exact_mut(4) {
                    let index = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
                    let slot = mapping
                        .get(index)
                        .ok_or_else(|| Error::new("Analysis reference is invalid"))?;
                    raw.copy_from_slice(&slot.to_le_bytes());
                }
                writer.write_all(&chunk[..length]).map_err(|error| {
                    Error::with_source("cannot write planned references", error)
                })?;
                remaining -= length as u64;
            }
            let mut tail = [0];
            if reader
                .read(&mut tail)
                .map_err(|error| Error::with_source("cannot finish Analysis references", error))?
                != 0
            {
                return Err(Error::new("Analysis references have trailing bytes"));
            }
            Some(
                writer
                    .into_inner()
                    .map_err(|error| {
                        Error::with_source("cannot flush planned references", error.into_error())
                    })?
                    .finish()
                    .map_err(|error| {
                        Error::with_source("cannot finish planned references", error)
                    })?,
            )
        } else {
            None
        };
        Ok(PendingValues {
            symbols: self.symbols,
            positions: Vec::new(),
            packed_references,
            scalar_count: current.scalar_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_io::NormalizingWriter;
    use serde_json::json;

    #[test]
    fn new_file_hash_does_not_displace_observation_identity_slots(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let old_id = "a".repeat(64);
        let new_id = "b".repeat(64);
        let file_hash = "f".repeat(64);
        let added_hash = "c".repeat(64);
        let base = json!({"files":[{"content_sha256":file_hash,"path":"old.py"}],
            "nodes":[{"identity":old_id,"name":"한글\\quoted","target":old_id}],"removed":"earlier-only"});
        let current = json!({"files":[{"content_sha256":file_hash,"path":"old.py"},
            {"content_sha256":added_hash,"path":"new.py"}],
            "nodes":[{"identity":new_id,"name":"한글\\quoted","target":new_id}]});
        let mut writer = NormalizingWriter::default();
        serde_json::to_writer(&mut writer, &base)?;
        let normalized = writer.finish()?;
        let reader =
            DenormalizingReader::new(normalized.shape, normalized.values, normalized.scalar_count)?;
        let raw_id = serde_json::to_vec(&old_id)?;
        let expected_slot = reader
            .symbols
            .iter()
            .position(|range| reader.values[range.clone()] == raw_id)
            .ok_or("old slot")?;
        let dictionary = Dictionary::from_reader(reader)?;
        let mut writer = NormalizingWriter::default();
        // Exercise scalar and punctuation tracking across arbitrary UTF-8 writes.
        for chunk in serde_json::to_vec(&current)?.chunks(1) {
            writer.write_all(chunk)?;
        }
        let parts = writer.finish_parts()?;
        let planned = dictionary.apply(parts.values)?.finish()?;
        let mut reader = DenormalizingReader::new(parts.shape, planned, parts.scalar_count)?;
        let raw_id = serde_json::to_vec(&new_id)?;
        assert_eq!(
            reader
                .symbols
                .iter()
                .position(|range| reader.values[range.clone()] == raw_id),
            Some(expected_slot)
        );
        let mut decoded = Vec::new();
        reader.read_to_end(&mut decoded)?;
        assert_eq!(decoded, serde_json::to_vec(&current)?);
        Ok(())
    }
}
