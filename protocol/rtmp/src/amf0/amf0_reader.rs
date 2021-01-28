use std::collections::HashMap;

use super::amf0_markers;
use super::Amf0ReadError;
use super::{Amf0ValueType};
use byteorder::{BigEndian};

// use super::define::UnOrderedMap;

use super::error::Amf0ReadErrorValue;
use liverust_lib::netio::{reader::Reader};

pub struct Amf0Reader {
    reader: Reader,
}

impl Amf0Reader {
    pub fn read_any(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let markers = self.reader.read_u8()?;

        if markers == amf0_markers::OBJECT_END {
            return Ok(Amf0ValueType::END);
        }

        match markers {
            amf0_markers::NUMBER => self.read_number(),
            amf0_markers::BOOLEAN => self.read_bool(),
            amf0_markers::STRING => self.read_string(),
            amf0_markers::OBJECT_END => self.read_object(),
            amf0_markers::NULL => self.read_null(),
            amf0_markers::ECMA_ARRAY => self.read_ecma_array(),
            amf0_markers::LONG_STRING => self.read_long_string(),
            _ => Err(Amf0ReadError {
                value: Amf0ReadErrorValue::UnknownMarker { marker: markers },
            }),
        }
    }

    pub fn read_number(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let number = self.reader.read_f64::<BigEndian>()?;
        let value = Amf0ValueType::Number(number);
        Ok(value)
    }

    pub fn read_bool(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let value = self.reader.read_u8()?;

        match value {
            1 => Ok(Amf0ValueType::Boolean(true)),
            _ => Ok(Amf0ValueType::Boolean(false)),
        }
    }

    pub fn read_raw_string(&mut self) -> Result<String, Amf0ReadError> {
        let l = self.reader.read_u16::<BigEndian>()?;
        // let mut buffer: Vec<u8> = vec![0_u8; l as usize];
        let bytes = self.reader.read_bytes(l as usize)?;

        let val = String::from_utf8(bytes.to_vec())?;

        Ok(val)
    }

    pub fn read_string(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let raw_string = self.read_raw_string()?;
        Ok(Amf0ValueType::UTF8String(raw_string))
    }

    pub fn read_null(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        Ok(Amf0ValueType::Null)
    }

    pub fn is_read_object_eof(&mut self) -> Result<bool, Amf0ReadError> {
        let marker = self.reader.advance_u24::<BigEndian>()?;
        if marker == 0x09 {
            return Ok(true);
        }
        Ok(false)
    }

    pub fn read_object(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let mut properties = HashMap::new();

        loop {
            let is_eof = self.is_read_object_eof()?;

            if is_eof {
                break;
            }

            let key = self.read_raw_string()?;
            let val = self.read_any()?;

            properties.insert(key, val);
        }

        Ok(Amf0ValueType::Object(properties))
    }

    pub fn read_ecma_array(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let len = self.reader.read_u32::<BigEndian>()?;

        let mut properties = HashMap::new();

        for _ in 0..len {
            let key = self.read_raw_string()?;
            let val = self.read_any()?;
            properties.insert(key, val);
        }

        self.is_read_object_eof()?;

        Ok(Amf0ValueType::Object(properties))
    }

    pub fn read_long_string(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let l = self.reader.read_u32::<BigEndian>()?;

        let buff = self.reader.read_bytes(l as usize)?;

        let val = String::from_utf8(buff.to_vec())?;
        Ok(Amf0ValueType::LongUTF8String(val))
    }
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
