use super::amf0_markers;
use super::Amf0ReadError;
use super::{Amf0ValueType, Amf0WriteError};
use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};

use super::define::UnOrderedMap;

use super::error::Amf0ReadErrorValue;
use liverust_lib::netio::{
    reader::{ Reader},
    writer::{Writer},
};

fn read_any(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
    let markers = bytes.read_u8()?;

    if markers == amf0_markers::OBJECT_END {
        return Ok(Amf0ValueType::END);
    }

    match markers {
        amf0_markers::NUMBER => read_number(bytes),
        amf0_markers::BOOLEAN => read_bool(bytes),
        amf0_markers::STRING => read_string(bytes),
        amf0_markers::OBJECT_END => read_object(bytes),
        amf0_markers::NULL => read_null(),
        amf0_markers::ECMA_ARRAY => read_ecma_array(bytes),
        amf0_markers::LONG_STRING => read_long_string(bytes),
        _ => Err(Amf0ReadError {
            value: Amf0ReadErrorValue::UnknownMarker { marker: markers },
        }),
    }
}

fn read_number(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
    let number = bytes.read_f64::<BigEndian>()?;
    let value = Amf0ValueType::Number(number);
    Ok(value)
}

fn read_bool(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
    let value = bytes.read_u8()?;

    match value {
        1 => Ok(Amf0ValueType::Boolean(true)),
        _ => Ok(Amf0ValueType::Boolean(false)),
    }
}

fn read_raw_string(bytes: &mut Reader) -> Result<String, Amf0ReadError> {
    let l = bytes.read_u16::<BigEndian>()?;
    let mut buffer: Vec<u8> = vec![0_u8; l as usize];
    let bytes = bytes.read_bytes(l as usize)?;

    let val = String::from_utf8(bytes.to_vec())?;
  
    Ok(val)
}

fn read_string(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
    let raw_string = read_raw_string(bytes)?;
    Ok(Amf0ValueType::UTF8String(raw_string))
}

fn read_null() -> Result<Amf0ValueType, Amf0ReadError> {
    Ok(Amf0ValueType::Null)
}

fn is_read_object_eof(bytes: &mut Reader) -> Result<bool, Amf0ReadError> {
    let marker = bytes.advance_u24::<BigEndian>()?;
    if marker == 0x09 {
        return Ok(true);
    }
    Ok(false)
}

fn read_object(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
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

fn read_ecma_array(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
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

fn read_long_string(bytes: &mut Reader) -> Result<Amf0ValueType, Amf0ReadError> {
    let l = bytes.read_u32::<BigEndian>()?;

    let buff = bytes.read_bytes(l as usize)?;

    let val = String::from_utf8(buff.to_vec())?;
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
