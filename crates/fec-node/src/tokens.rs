//! Typed records (`fec_parser::covers::Cover`, `itemizations::Itemization`) as
//! a flat token stream, decoded into plain objects by `js/tokens.ts`.
//!
//! Building JS objects from Rust costs a Node-API call per value; a batch of
//! tokens is four buffers and one string, and the JS side rebuilds the objects
//! with one generated factory per struct type (plans/nodejs/05-typed-records.md §1).
//!
//! The serializer is generic over serde: no per-struct code. A value is:
//!
//! | tag | meaning | payload |
//! |---|---|---|
//! | `NULL` | `None`, unit, or a non-finite number (as `serde_json` writes it) | — |
//! | `STR` | string, date (`"YYYY-MM-DD"`), unit variant | next `ends` entry |
//! | `NUM` | any integer or float | next `nums` entry |
//! | `TRUE`/`FALSE` | bool | — |
//! | `STRUCT` | struct | struct id (next `nums` entry), then each field's value |
//! | `SEQ` | sequence | length (next `nums` entry), then each item |
//!
//! Struct ids are keyed by `(name, keys)`, not the name alone: a struct
//! serialized as an internally tagged enum variant gets a leading `family`
//! key, so the same Rust type can arrive with two key lists. A struct's keys
//! are only known once it has been serialized, so its id is written as a
//! placeholder and patched in `end()`. Ids are stable for the life of a
//! [`StructTable`]; each batch lists the ids it introduced in `new_structs`
//! as `"id\x1fName\x1fkey1\x1fkey2…"`.

use napi::bindgen_prelude::{Float64Array, Uint32Array, Uint8Array};
use napi_derive::napi;
use serde::ser::{self, Serialize};

pub const NULL: u8 = 0;
pub const STR: u8 = 1;
pub const NUM: u8 = 2;
pub const TRUE: u8 = 3;
pub const FALSE: u8 = 4;
pub const STRUCT: u8 = 5;
pub const SEQ: u8 = 6;

/// One batch of typed values, one top-level value per row.
#[napi(object)]
pub struct TokenBatch {
    /// Top-level values in this batch.
    pub rows: u32,
    pub tags: Uint8Array,
    pub nums: Float64Array,
    /// Every string value, concatenated.
    pub text: String,
    /// UTF-16 end offset of each string value in `text`.
    pub ends: Uint32Array,
    /// Struct types first seen in this batch: `"id\x1fName\x1fkey1\x1fkey2…"`.
    pub new_structs: Vec<String>,
}

/// Struct ids, keyed by `(name, keys)`, for the life of a reader.
///
/// Looked up once per serialized struct, so it's a flat list compared by
/// pointer first: serde passes the same `&'static str`s every time.
#[derive(Default)]
pub struct StructTable {
    by_name: Vec<(&'static str, Vec<Variant>)>,
    next: u32,
}

/// One key list a struct name was seen with, and its id.
type Variant = (Vec<&'static str>, u32);

fn same(a: &str, b: &str) -> bool {
    (a.as_ptr() == b.as_ptr() && a.len() == b.len()) || a == b
}

impl StructTable {
    /// The id for `(name, keys)`, and whether it is new.
    fn intern(&mut self, name: &'static str, keys: &[&'static str]) -> (u32, bool) {
        let i = match self.by_name.iter().position(|(n, _)| same(n, name)) {
            Some(i) => i,
            None => {
                self.by_name.push((name, Vec::new()));
                self.by_name.len() - 1
            }
        };
        let variants = &mut self.by_name[i].1;
        let found = variants
            .iter()
            .find(|(k, _)| k.len() == keys.len() && k.iter().zip(keys).all(|(a, b)| same(a, b)));
        if let Some((_, id)) = found {
            return (*id, false);
        }
        let id = self.next;
        self.next += 1;
        variants.push((keys.to_vec(), id));
        (id, true)
    }
}

/// Accumulates one batch of tokens.
#[derive(Default)]
pub struct TokenWriter {
    tags: Vec<u8>,
    nums: Vec<f64>,
    text: String,
    ends: Vec<u32>,
    off: u32,
    rows: u32,
    new_structs: Vec<String>,
    /// Reused key buffers, one per level of struct nesting.
    key_pool: Vec<Vec<&'static str>>,
}

impl TokenWriter {
    /// Append one top-level value (`None` → `NULL`).
    pub fn push<T: Serialize>(
        &mut self,
        table: &mut StructTable,
        value: Option<&T>,
    ) -> Result<(), SerError> {
        self.rows += 1;
        match value {
            Some(v) => v.serialize(&mut TokenSer { w: self, table }),
            None => {
                self.tags.push(NULL);
                Ok(())
            }
        }
    }

    /// Hand the batch over, leaving the writer empty (key buffers kept).
    pub fn take(&mut self) -> TokenBatch {
        let batch = TokenBatch {
            rows: self.rows,
            tags: std::mem::take(&mut self.tags).into(),
            nums: std::mem::take(&mut self.nums).into(),
            text: std::mem::take(&mut self.text),
            ends: std::mem::take(&mut self.ends).into(),
            new_structs: std::mem::take(&mut self.new_structs),
        };
        self.off = 0;
        self.rows = 0;
        batch
    }

    fn str(&mut self, v: &str) {
        self.tags.push(STR);
        self.off += utf16_len(v);
        self.ends.push(self.off);
        self.text.push_str(v);
    }

    fn num(&mut self, v: f64) {
        if v.is_finite() {
            self.tags.push(NUM);
            self.nums.push(v);
        } else {
            // serde_json writes NaN and ±inf as null; match it.
            self.tags.push(NULL);
        }
    }
}

/// UTF-16 length of `s`: what JS's `String.prototype.slice` counts in.
pub fn utf16_len(s: &str) -> u32 {
    if s.is_ascii() {
        s.len() as u32
    } else {
        s.encode_utf16().count() as u32
    }
}

#[derive(Debug)]
pub struct SerError(String);

impl std::fmt::Display for SerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token serializer: {}", self.0)
    }
}

impl std::error::Error for SerError {}

impl ser::Error for SerError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        SerError(msg.to_string())
    }
}

struct TokenSer<'w, 't> {
    w: &'w mut TokenWriter,
    table: &'t mut StructTable,
}

/// A struct being serialized: collects its keys, then patches its id.
struct StructSer<'s, 'w, 't> {
    s: &'s mut TokenSer<'w, 't>,
    name: &'static str,
    /// Index of the id placeholder in `nums`.
    id_at: usize,
    keys: Vec<&'static str>,
}

impl ser::SerializeStruct for StructSer<'_, '_, '_> {
    type Ok = ();
    type Error = SerError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        v: &T,
    ) -> Result<(), SerError> {
        self.keys.push(key);
        v.serialize(&mut *self.s)
    }

    fn end(self) -> Result<(), SerError> {
        let (id, new) = self.s.table.intern(self.name, &self.keys);
        self.s.w.nums[self.id_at] = id as f64;
        if new {
            let mut d = format!("{id}\x1f{}", self.name);
            for k in &self.keys {
                d.push('\x1f');
                d.push_str(k);
            }
            self.s.w.new_structs.push(d);
        }
        let mut keys = self.keys;
        keys.clear();
        self.s.w.key_pool.push(keys);
        Ok(())
    }
}

impl ser::SerializeSeq for &mut TokenSer<'_, '_> {
    type Ok = ();
    type Error = SerError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), SerError> {
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<(), SerError> {
        Ok(())
    }
}

/// Serde shapes the typed structs don't use: error by name, so a new shape
/// fails loudly instead of decoding wrong.
macro_rules! unsupported {
    ($($name:ident($($arg:ty),*) -> $ret:ty;)*) => {$(
        fn $name(self, $(_: $arg),*) -> Result<$ret, SerError> {
            Err(SerError(concat!(stringify!($name), " is not supported").into()))
        }
    )*};
}

impl<'s, 'w, 't> ser::Serializer for &'s mut TokenSer<'w, 't> {
    type Ok = ();
    type Error = SerError;
    type SerializeSeq = Self;
    type SerializeTuple = ser::Impossible<(), SerError>;
    type SerializeTupleStruct = ser::Impossible<(), SerError>;
    type SerializeTupleVariant = ser::Impossible<(), SerError>;
    type SerializeMap = ser::Impossible<(), SerError>;
    type SerializeStruct = StructSer<'s, 'w, 't>;
    type SerializeStructVariant = ser::Impossible<(), SerError>;

    fn serialize_bool(self, v: bool) -> Result<(), SerError> {
        self.w.tags.push(if v { TRUE } else { FALSE });
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<(), SerError> {
        self.w.num(v as f64);
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<(), SerError> {
        self.w.num(v as f64);
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), SerError> {
        self.w.num(v.into());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<(), SerError> {
        self.w.num(v);
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<(), SerError> {
        self.w.str(v.encode_utf8(&mut [0; 4]));
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<(), SerError> {
        self.w.str(v);
        Ok(())
    }
    fn serialize_none(self) -> Result<(), SerError> {
        self.w.tags.push(NULL);
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<(), SerError> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), SerError> {
        self.w.tags.push(NULL);
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), SerError> {
        self.w.tags.push(NULL);
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), SerError> {
        self.w.str(variant);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<(), SerError> {
        v.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, SerError> {
        let len = len.ok_or_else(|| SerError("a sequence without a length".into()))?;
        self.w.tags.push(SEQ);
        self.w.nums.push(len as f64);
        Ok(self)
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<StructSer<'s, 'w, 't>, SerError> {
        self.w.tags.push(STRUCT);
        let id_at = self.w.nums.len();
        self.w.nums.push(0.0); // patched in `end()`
        let mut keys = self.w.key_pool.pop().unwrap_or_default();
        keys.reserve(len);
        Ok(StructSer {
            s: self,
            name,
            id_at,
            keys,
        })
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), SerError> {
        Err(SerError(
            "serialize_newtype_variant is not supported".into(),
        ))
    }
    unsupported! {
        serialize_bytes(&[u8]) -> ();
        serialize_tuple(usize) -> Self::SerializeTuple;
        serialize_tuple_struct(&'static str, usize) -> Self::SerializeTupleStruct;
        serialize_tuple_variant(&'static str, u32, &'static str, usize) -> Self::SerializeTupleVariant;
        serialize_map(Option<usize>) -> Self::SerializeMap;
        serialize_struct_variant(&'static str, u32, &'static str, usize) -> Self::SerializeStructVariant;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fec_parser::covers::{Address, PersonName};
    use fec_parser::itemizations::{CandidateRef, Entity, Itemization};
    use serde_json::{json, Map, Value};
    use std::collections::HashMap;

    /// A Rust decoder for the token stream, for comparing with serde_json.
    struct Decoder<'a> {
        w: &'a TokenWriter,
        structs: HashMap<u32, Vec<String>>,
        tag: usize,
        num: usize,
        s: usize,
        text16: Vec<u16>,
    }

    impl<'a> Decoder<'a> {
        fn new(w: &'a TokenWriter) -> Self {
            let structs = w
                .new_structs
                .iter()
                .map(|d| {
                    let mut parts = d.split('\x1f');
                    let id = parts.next().unwrap().parse().unwrap();
                    let _name = parts.next().unwrap();
                    (id, parts.map(str::to_owned).collect())
                })
                .collect();
            Decoder {
                w,
                structs,
                tag: 0,
                num: 0,
                s: 0,
                text16: w.text.encode_utf16().collect(),
            }
        }

        fn num(&mut self) -> f64 {
            self.num += 1;
            self.w.nums[self.num - 1]
        }

        fn value(&mut self) -> Value {
            self.tag += 1;
            match self.w.tags[self.tag - 1] {
                NULL => Value::Null,
                TRUE => Value::Bool(true),
                FALSE => Value::Bool(false),
                NUM => json!(self.num()),
                STR => {
                    let start = if self.s == 0 {
                        0
                    } else {
                        self.w.ends[self.s - 1]
                    };
                    let end = self.w.ends[self.s];
                    self.s += 1;
                    Value::String(
                        String::from_utf16(&self.text16[start as usize..end as usize]).unwrap(),
                    )
                }
                SEQ => {
                    let n = self.num() as usize;
                    Value::Array((0..n).map(|_| self.value()).collect())
                }
                STRUCT => {
                    let id = self.num() as u32;
                    let keys = self.structs[&id].clone();
                    let mut m = Map::new();
                    for k in keys {
                        let v = self.value();
                        m.insert(k, v);
                    }
                    Value::Object(m)
                }
                t => panic!("bad tag {t}"),
            }
        }
    }

    /// serde_json's value with integral floats normalized (it writes `0.0`,
    /// the decoder can't tell `0` from `0.0`).
    fn expected<T: Serialize>(v: &T) -> Value {
        fn norm(v: Value) -> Value {
            match v {
                Value::Number(n) => json!(n.as_f64().unwrap()),
                Value::Array(a) => Value::Array(a.into_iter().map(norm).collect()),
                Value::Object(m) => {
                    Value::Object(m.into_iter().map(|(k, v)| (k, norm(v))).collect())
                }
                v => v,
            }
        }
        norm(serde_json::to_value(v).unwrap())
    }

    fn name(last: &str) -> PersonName {
        PersonName {
            first_name: "ÉMILE".into(),
            last_name: last.into(),
            middle_name: None,
            prefix: None,
            suffix: Some("😀 JR".into()),
        }
    }

    #[test]
    fn struct_ids_are_keyed_by_name_and_keys() {
        let mut table = StructTable::default();
        assert_eq!(table.intern("A", &["x"]), (0, true));
        assert_eq!(table.intern("A", &["family", "x"]), (1, true));
        assert_eq!(table.intern("A", &["x"]), (0, false));
        assert_eq!(table.intern("B", &["x"]), (2, true));
    }

    #[test]
    fn decodes_like_serde_json() {
        let entity = Entity {
            entity_type: Some("IND".into()),
            organization_name: None,
            name: name("O'BRIEN"),
            address: Address {
                street_1: Some("1 MAIN ST".into()),
                street_2: None,
                city: Some("AUSTIN".into()),
                state: Some("TX".into()),
                zip_code: Some("78701".into()),
            },
        };
        let candidate = CandidateRef {
            fec_id: None,
            name: name(""),
            office: Some("H".into()),
            state: None,
            district: None,
        };
        let values = [
            serde_json::to_value(&entity).unwrap(),
            serde_json::to_value(&candidate).unwrap(),
        ];
        let mut table = StructTable::default();
        let mut w = TokenWriter::default();
        w.push(&mut table, Some(&entity)).unwrap();
        w.push(&mut table, Some(&candidate)).unwrap();
        w.push::<Entity>(&mut table, None).unwrap();
        w.push(&mut table, Some(&vec![1.5f64, f64::NAN, f64::INFINITY]))
            .unwrap();
        w.push(&mut table, Some(&Vec::<String>::new())).unwrap();
        let mut d = Decoder::new(&w);
        assert_eq!(d.value(), expected(&values[0]));
        assert_eq!(d.value(), expected(&values[1]));
        assert_eq!(d.value(), Value::Null);
        assert_eq!(d.value(), json!([1.5, null, null]));
        assert_eq!(d.value(), json!([]));
        assert_eq!(w.rows, 5);
        assert_eq!(d.tag, w.tags.len());
        assert_eq!(d.num, w.nums.len());
    }

    #[test]
    fn itemizations_carry_the_family_key_first() {
        let line = "SA11AI\x1cC00000001\x1cT1\x1c\x1c\x1cIND\x1c\x1cSMITH\x1cJANE\x1c\x1c\x1c\x1c\
                    1 MAIN\x1c\x1cAUSTIN\x1cTX\x1c78701\x1cP2026\x1c\x1c20260115\x1c100.50\x1c\
                    100.50\x1c\x1c\x1cACME\x1cENGINEER";
        let record = csv::StringRecord::from(line.split('\x1c').collect::<Vec<_>>());
        let it = Itemization::from_record(&record, "8.4", None).expect("typed");
        let mut table = StructTable::default();
        let mut w = TokenWriter::default();
        w.push(&mut table, Some(&it)).unwrap();
        w.push(&mut table, Some(&it)).unwrap();
        assert!(w.new_structs[w.new_structs.len() - 1].starts_with(&format!(
            "{}\x1fScheduleA\x1ffamily\x1fform_type",
            w.new_structs.len() - 1
        )));
        let mut d = Decoder::new(&w);
        let want = expected(&it);
        assert_eq!(d.value(), want);
        assert_eq!(d.value(), want);
    }
}
