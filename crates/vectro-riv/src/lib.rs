//! Reader for the `.riv` binary format (major version 7).
//!
//! Layout, per Rive's published format docs and MIT runtime:
//! `RIVE` fingerprint, varuint major, varuint minor, varuint file id,
//! a table of contents (property keys, then 2-bit field types), then
//! objects: varuint type key, (varuint property key, value)*, 0.

use std::collections::HashMap;
use std::fmt;

pub const SUPPORTED_MAJOR: u64 = 7;
const KEYS_JSON: &str = include_str!("../../../data/rive-keys.json");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldType {
    Uint,
    String,
    Double,
    Color,
    Bool,
    Int,
    Bytes,
}

impl FieldType {
    fn from_registry(name: &str) -> Option<Self> {
        Some(match name {
            "uint" | "id" => Self::Uint,
            "int" => Self::Int,
            "string" => Self::String,
            "double" => Self::Double,
            "color" => Self::Color,
            "bool" => Self::Bool,
            "bytes" => Self::Bytes,
            _ => return None,
        })
    }

    /// The 2-bit backing type the table of contents records.
    fn from_toc(bits: u32) -> Self {
        match bits {
            1 => Self::String,
            2 => Self::Double,
            3 => Self::Color,
            _ => Self::Uint,
        }
    }

    fn toc_bits(self) -> u32 {
        match self {
            Self::String | Self::Bytes => 1,
            Self::Double => 2,
            Self::Color => 3,
            Self::Uint | Self::Int | Self::Bool => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Uint(u64),
    Int(i64),
    Double(f32),
    Color(u32),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Uint(v) => write!(f, "{v}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Double(v) => write!(f, "{v}"),
            Value::Color(v) => write!(f, "#{v:08x}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "{v:?}"),
            Value::Bytes(v) => write!(f, "<{} bytes>", v.len()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: u64,
    pub name: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub type_key: u64,
    pub type_name: Option<String>,
    pub properties: Vec<Property>,
}

#[derive(Debug)]
pub struct RivFile {
    pub major: u64,
    pub minor: u64,
    pub file_id: u64,
    /// Properties newer than the baseline, in file order, with their backing type.
    pub toc: Vec<(u64, FieldType)>,
    pub objects: Vec<Object>,
}

#[derive(Debug)]
pub enum Error {
    BadFingerprint,
    UnsupportedMajor(u64),
    Truncated(usize),
    UnknownProperty { key: u64, offset: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadFingerprint => write!(f, "not a .riv file (missing RIVE fingerprint)"),
            Error::UnsupportedMajor(v) => write!(f, "format {v} is not supported (need {SUPPORTED_MAJOR})"),
            Error::Truncated(at) => write!(f, "file ends early at byte {at}"),
            Error::UnknownProperty { key, offset } => {
                write!(f, "property {key} at byte {offset} is in neither the schema nor the table of contents")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Type and property names from Rive's generated definitions.
pub struct Schema {
    types: HashMap<u64, String>,
    /// type name -> (parent type name, property name -> key)
    chain: HashMap<String, (String, HashMap<String, u64>)>,
    props: HashMap<u64, (String, Option<FieldType>)>,
}

impl Schema {
    pub fn bundled() -> Self {
        let json: serde_json::Value = serde_json::from_str(KEYS_JSON).expect("bundled key table is valid JSON");
        let types = json["types"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v["name"].as_str().unwrap().to_string()))
            .collect();
        let mut chain: HashMap<String, (String, HashMap<String, u64>)> = HashMap::new();
        for (k, v) in json["properties"].as_object().unwrap() {
            let owner = v["owner"].as_str().unwrap().to_string();
            chain.entry(owner).or_default().1.insert(v["name"].as_str().unwrap().to_string(), k.parse().unwrap());
        }
        for v in json["types"].as_object().unwrap().values() {
            let entry = chain.entry(v["name"].as_str().unwrap().to_string()).or_default();
            entry.0 = v["parent"].as_str().unwrap().to_string();
        }
        let props = json["properties"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                let ty = v["type"].as_str().and_then(FieldType::from_registry);
                (k.parse().unwrap(), (v["name"].as_str().unwrap().to_string(), ty))
            })
            .collect();
        Schema { types, chain, props }
    }

    pub fn type_name(&self, key: u64) -> Option<&str> {
        self.types.get(&key).map(String::as_str)
    }

    pub fn type_key(&self, name: &str) -> Option<u64> {
        self.types.iter().find(|(_, n)| n.as_str() == name).map(|(k, _)| *k)
    }

    /// The key of `property` as seen on `type_name`, searching up its parent types.
    pub fn property_key(&self, type_name: &str, property: &str) -> Option<u64> {
        let mut current = type_name;
        for _ in 0..32 {
            let (parent, props) = self.chain.get(current)?;
            if let Some(k) = props.get(property) {
                return Some(*k);
            }
            if parent.is_empty() || parent == current {
                return None;
            }
            current = parent;
        }
        None
    }

    pub fn property_type(&self, key: u64) -> Option<FieldType> {
        self.props.get(&key).and_then(|(_, t)| *t)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.bytes.get(self.pos).ok_or(Error::Truncated(self.pos))?;
        self.pos += 1;
        Ok(b)
    }

    fn varuint(&mut self) -> Result<u64, Error> {
        let (mut value, mut shift) = (0u64, 0);
        loop {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift > 63 {
                return Err(Error::Truncated(self.pos));
            }
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or(Error::Truncated(self.pos))?;
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn value(&mut self, ty: FieldType) -> Result<Value, Error> {
        Ok(match ty {
            FieldType::Uint => Value::Uint(self.varuint()?),
            FieldType::Int => {
                let z = self.varuint()?;
                Value::Int(((z >> 1) as i64) ^ -((z & 1) as i64))
            }
            FieldType::Double => Value::Double(f32::from_bits(self.u32()?)),
            FieldType::Color => Value::Color(self.u32()?),
            FieldType::Bool => Value::Bool(self.byte()? == 1),
            FieldType::String => {
                let n = self.varuint()? as usize;
                Value::String(String::from_utf8_lossy(self.take(n)?).into_owned())
            }
            FieldType::Bytes => {
                let n = self.varuint()? as usize;
                Value::Bytes(self.take(n)?.to_vec())
            }
        })
    }
}

pub fn read(bytes: &[u8], schema: &Schema) -> Result<RivFile, Error> {
    let mut r = Reader { bytes, pos: 0 };
    if r.take(4).map_err(|_| Error::BadFingerprint)? != b"RIVE" {
        return Err(Error::BadFingerprint);
    }
    let major = r.varuint()?;
    if major != SUPPORTED_MAJOR {
        return Err(Error::UnsupportedMajor(major));
    }
    let minor = r.varuint()?;
    let file_id = r.varuint()?;

    let mut keys = Vec::new();
    loop {
        let k = r.varuint()?;
        if k == 0 {
            break;
        }
        keys.push(k);
    }
    let mut toc = Vec::with_capacity(keys.len());
    let (mut word, mut bit) = (0u32, 8);
    for k in keys {
        if bit == 8 {
            word = r.u32()?;
            bit = 0;
        }
        toc.push((k, FieldType::from_toc((word >> bit) & 3)));
        bit += 2;
    }

    let mut objects = Vec::new();
    while r.pos < bytes.len() {
        let type_key = r.varuint()?;
        let mut properties = Vec::new();
        loop {
            let offset = r.pos;
            let key = r.varuint()?;
            if key == 0 {
                break;
            }
            let known = schema.props.get(&key);
            // The schema knows bool/int/bytes apart; the ToC only knows the backing type.
            let ty = known
                .and_then(|(_, t)| *t)
                .or_else(|| toc.iter().find(|(k, _)| *k == key).map(|(_, t)| *t))
                .ok_or(Error::UnknownProperty { key, offset })?;
            let value = r.value(ty)?;
            properties.push(Property { key, name: known.map(|(n, _)| n.clone()), value });
        }
        objects.push(Object { type_key, type_name: schema.type_name(type_key).map(str::to_string), properties });
    }

    Ok(RivFile { major, minor, file_id, toc, objects })
}

fn put_varuint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn put_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Uint(v) => put_varuint(out, *v),
        Value::Int(v) => put_varuint(out, ((v << 1) ^ (v >> 63)) as u64),
        Value::Double(v) => out.extend_from_slice(&v.to_bits().to_le_bytes()),
        Value::Color(v) => out.extend_from_slice(&v.to_le_bytes()),
        Value::Bool(v) => out.push(u8::from(*v)),
        Value::String(v) => {
            put_varuint(out, v.len() as u64);
            out.extend_from_slice(v.as_bytes());
        }
        Value::Bytes(v) => {
            put_varuint(out, v.len() as u64);
            out.extend_from_slice(v);
        }
    }
}

/// Serialise a file. Reading a file and writing it back gives the same bytes.
pub fn write(file: &RivFile) -> Vec<u8> {
    let mut out = b"RIVE".to_vec();
    put_varuint(&mut out, file.major);
    put_varuint(&mut out, file.minor);
    put_varuint(&mut out, file.file_id);
    for (k, _) in &file.toc {
        put_varuint(&mut out, *k);
    }
    put_varuint(&mut out, 0);
    // Rive packs four 2-bit types per u32 word and leaves the upper 24 bits zero.
    for chunk in file.toc.chunks(4) {
        let word = chunk.iter().enumerate().fold(0u32, |w, (i, (_, t))| w | (t.toc_bits() << (i * 2)));
        out.extend_from_slice(&word.to_le_bytes());
    }
    for o in &file.objects {
        put_varuint(&mut out, o.type_key);
        for p in &o.properties {
            put_varuint(&mut out, p.key);
            put_value(&mut out, &p.value);
        }
        put_varuint(&mut out, 0);
    }
    out
}

/// Builds a file object by object, looking up keys by name through each type's chain
/// and listing every property it uses in the table of contents.
pub struct Builder<'s> {
    schema: &'s Schema,
    objects: Vec<Object>,
    used: Vec<u64>,
}

impl<'s> Builder<'s> {
    pub fn new(schema: &'s Schema) -> Self {
        Builder { schema, objects: Vec::new(), used: Vec::new() }
    }

    /// Append an object; returns its position in the file.
    pub fn add(&mut self, type_name: &str, props: &[(&str, Value)]) -> usize {
        let type_key = self.schema.type_key(type_name).unwrap_or_else(|| panic!("unknown type {type_name}"));
        let properties = props
            .iter()
            .map(|(name, value)| {
                let key = self
                    .schema
                    .property_key(type_name, name)
                    .unwrap_or_else(|| panic!("{type_name} has no property {name}"));
                if !self.used.contains(&key) {
                    self.used.push(key);
                }
                Property { key, name: Some(name.to_string()), value: value.clone() }
            })
            .collect();
        self.objects.push(Object { type_key, type_name: Some(type_name.into()), properties });
        self.objects.len() - 1
    }

    pub fn schema(&self) -> &Schema {
        self.schema
    }

    /// Change a property of an object already added (by the position `add` returned), for
    /// references that are only known once later objects have been placed.
    pub fn set(&mut self, at: usize, name: &str, value: Value) {
        let o = &mut self.objects[at];
        let type_name = o.type_name.clone().expect("objects added here carry their type name");
        let key = self
            .schema
            .property_key(&type_name, name)
            .unwrap_or_else(|| panic!("{type_name} has no property {name}"));
        if !self.used.contains(&key) {
            self.used.push(key);
        }
        match o.properties.iter_mut().find(|p| p.key == key) {
            Some(p) => p.value = value,
            None => o.properties.push(Property { key, name: Some(name.to_string()), value }),
        }
    }

    pub fn finish(self) -> RivFile {
        let toc = self.used.iter().map(|k| (*k, self.schema.property_type(*k).unwrap_or(FieldType::Uint))).collect();
        RivFile { major: SUPPORTED_MAJOR, minor: 0, file_id: 1, toc, objects: self.objects }
    }
}
