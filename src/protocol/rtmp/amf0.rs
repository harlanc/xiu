use byteorder::BigEndian;
use bytes::Bytes;
use std::io::Cursor;
use std::io::Read;

mod amf0_markers {
    pub const NUMBER: u8 = 0x00;
    pub const BOOLEAN: u8 = 0x01;
    pub const STRING: u8 = 0x02;
    pub const OBJECT: u8 = 0x03;
    pub const NULL: u8 = 0x05;
    pub const ECMA_ARRAY: u8 = 0x08;
    pub const OBJECT_END: u8 = 0x09;
    pub const LONG_STRING: u8 = 0x0c;
}

enum Amf0ValueType {
    Number(f64),
    Boolean(bool),
    UTF8String(String),
    Object(UnOrderedMap),
    Null,
    EcmaArray(UnOrderedMap),
    LongUTF8String(String),
}

struct Object {
    key: String,
    Value: Amf0ValueType,
}

struct UnOrderedMap {
    properties: Vec<Object>,
}

impl UnOrderedMap {
    pub fn new() -> UnOrderedMap {
        UnOrderedMap {
            properties: Vec::new(),
        }
    }
    fn insert(self, key: String, val: Amf0ValueType) -> Option(Amf0ValueType) {
        for i in self.properties {
            if i.key == key {
                let tmpVal = i.Value;
                i.Value = val;
                return Option(tmpVal);
            }
        }

        let obj = Object {
            key: key,
            Value: val,
        };
        self.properties.push(obj);

        Option(None)
    }
    fn get(self, key: String) -> Option(Amf0ValueType) {
        for i in self.properties {
            if i.key == key {
                return Option(i.key);
            }
        }
        Option(None)
    }
}

fn read_any<R: Read>(bytes: &mut Vec<u8>) -> Result<Amf0ValueType, Amf0ReadError> {
    let mut buffer: [u8; 1] = [0];
    let bytes_num = bytes.read(&mut buffer)?;

    if bytes_num == 0 {
        return Ok(None);
    }

    if buffer[0] == amf0_markers::OBJECT_END {
        return Ok(None);
    }

    match buffer[0] {
        amf0_markers::NUMBER => read_number(bytes).map(Some),
        amf0_markers::BOOLEAN => read_bool(bytes).map(Some),
        amf0_markers::UTF8String => read_string(bytes).map(Some),
        amf0_markers::OBJECT_END => read_object(bytes).map(Some),
        amf0_markers::NULL => read_null().map(Some),
        amf0_markers::ECMA_ARRAY => read_ecma_array(bytes).map(Some),
        amf0_markers::LONG_STRING => read_long_string(bytes).map(Some),
        _ => Err(Amf0ReadError::UnknownMarker { marker: buffer[0] }),
    }
}

fn read_number<R: Read>(bytes: &mut Vec<u8>) -> Result<Amf0ValueType, Amf0ReadError> {
    let number = bytes.read_f64::<BigEndian>()?;
    let value = Amf0ValueType::Number(number);
    Ok(value)
}

fn read_bool<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let value = bytes.read_u8()?;

    match value {
        1 => Ok(Amf0ValueType::Boolean(true)),
        _ => Ok(Amf0ValueType::Boolean(false)),
    }
}

fn read_raw_string<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let l = bytes.read_u16::<BigEndian>()?;
    let mut buffer: Vec<u8> = vec![0_u8; l as usize];
    bytes.read(&mut buffer);

    let val = String::from_utf8(buffer)?;
    Ok(val)
}

fn read_string<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let raw_string = read_raw_string(bytes)?;
    Ok(Amf0ValueType::UTF8String(raw_string))
}

fn read_null() -> Result<Amf0ValueType, Amf0ReadError> {
    Ok(Amf0ValueType::Null)
}

fn is_read_object_eof<R: Read>(bytes: &mut R) -> Result<bool, Amf0ReadError> {
    let marker = bytes.read_u24::<BigEndian>()?;
    if marker == 0x09 {
        Ok(true)
    }
    bytes.write_u24::<BigEndian>(marker)?;
    Ok(false)
}

fn read_object<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let mut properties = UnOrderedMap::new();

    loop {
        let is_eof = is_read_object_eof(bytes)?;

        if is_eof {
            break;
        }

        let key = read_raw_string(bytes)?;
        let val = read_any(bytes)?;

        properties.insert(key, val);
    }

    Ok(Amf0ValueType::Object(properties))
}

fn read_ecma_array<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let len = bytes.read_u32::<BigEndian>()?;

    let mut properties = UnOrderedMap::new();

    for i in 0..len {
        let key = read_raw_string(bytes)?;
        let val = read_any(bytes)?;
        properties.insert(key, val);
    }

    is_read_object_eof(bytes)?;

    Ok(Amf0ValueType::Object(properties))
}

fn read_long_string<R: Read>(bytes: &mut R) -> Result<Amf0ValueType, Amf0ReadError> {
    let l = bytes.read_u32::<BigEndian>()?;
    let mut buffer: Vec<u8> = vec![0_u8; l as usize];
    bytes.read(&mut buffer);

    let val = String::from_utf8(buffer)?;
    Ok(Amf0ValueType::LongUTF8String(val))
}

#[cfg(test)]
mod tests {

    #[test]

    fn test_byte_order() {
        use byteorder::{BigEndian, ByteOrder};

        let phi = 1.6180339887;
        let mut buf = [0; 8];
        BigEndian::write_f64(&mut buf, phi);
        assert_eq!(phi, BigEndian::read_f64(&buf));
        println!("tsetstt")
    }
}
