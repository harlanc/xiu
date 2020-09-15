use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;
use std::io::Cursor;
use std::io::Read;

mod amf0_markers {
    pub const NUMBER_MARKER: u8 = 0x00;
    pub const BOOLEAN: u8 = 0x01;
    pub const STRING: u8 = 0x02;
    pub const OBJECT: u8 = 0x03;
    pub const NULL: u8 = 0x05;
    pub const ECMA_ARRAY: u8 = 0x08;
    pub const OBJECT_END: u8 = 0x09;
    pub const LONG_STRING: u8 = 0x0c;
}

enum Amf0Value {
    Number(f64),
    Boolean(bool),
    UTF8String(String),
    Object(HashMap<String, Amf0Value>),
    Null,
}

struct Object {
    key: String,
    Value: Amf0Value,
}

fn read_number<R: Read>(bytes: &mut R) -> Result<Amf0Value> {
    let number = bytes.read_f64::<BigEndian>()?;
    let value = Amf0Value::Number(number);
    Ok(value)
}

fn read_null() -> Result<Amf0Value> {
    Ok(Amf0Value::Null)
}

fn read_bool<R: Read>(bytes: &mut R) -> Result<Amf0Value> {
    let value = bytes.read_u8()?;

    match value {
        1 => Ok(Amf0Value::Boolean(true)),
        _ => Ok(Amf0Value::Boolean(false)),
    }
}
fn read_string<R: >
