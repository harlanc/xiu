use std::collections::HashMap;

use super::amf0_markers;
use super::Amf0ReadError;
use super::Amf0ValueType;
use byteorder::BigEndian;

// use super::define::UnOrderedMap;

use super::errors::Amf0ReadErrorValue;
use netio::bytes_reader::BytesReader;

pub struct Amf0Reader {
    reader: BytesReader,
}

impl Amf0Reader {
    pub fn new(reader: BytesReader) -> Self {
        Self { reader: reader }
    }
    pub fn read_all(&mut self) -> Result<Vec<Amf0ValueType>, Amf0ReadError> {
        let mut results = vec![];

        loop {
            let result = self.read_any()?;

            match result {
                Amf0ValueType::END => {
                    break;
                }
                _ => {
                    results.push(result);
                }
            }
        }

        Ok(results)
    }
    pub fn read_any(&mut self) -> Result<Amf0ValueType, Amf0ReadError> {
        let markers = self.reader.read_u8()?;

        if markers == amf0_markers::OBJECT_END {
            return Ok(Amf0ValueType::END);
        }

        match markers {
            amf0_markers::NUMBER => self.read_number(),
            amf0_markers::BOOLEAN => self.read_bool(),
            amf0_markers::STRING => self.read_string(),
            amf0_markers::OBJECT => self.read_object(),
            amf0_markers::NULL => self.read_null(),
            amf0_markers::ECMA_ARRAY => self.read_ecma_array(),
            amf0_markers::LONG_STRING => self.read_long_string(),
            _ => Err(Amf0ReadError {
                value: Amf0ReadErrorValue::UnknownMarker { marker: markers },
            }),
        }
    }
    pub fn read_with_type(&mut self, specified_marker: u8) -> Result<Amf0ValueType, Amf0ReadError> {
        let marker = self.reader.advance_u8()?;

        if marker != specified_marker {
            return Err(Amf0ReadError {
                value: Amf0ReadErrorValue::WrongType,
            });
        }

        return self.read_any();
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

    use super::amf0_markers;
    use super::Amf0Reader;
    use super::Amf0ValueType;
    use bytes::BytesMut;
    use netio::bytes_reader::BytesReader;
    use std::collections::HashMap;

    #[test]
    fn test_amf_reader() {
        let data: [u8; 177] = [
            2, 0, 7, 99, 111, 110, 110, 101, 99, 116, 0, 63, 240, 0, 0, 0, 0, 0, 0, //body
            3, 0, 3, 97, 112, 112, 2, 0, 6, 104, 97, 114, 108, 97, 110, 0, 4, 116, 121, 112, 101,
            2, 0, 10, 110, 111, 110, 112, 114, 105, 118, 97, 116, 101, 0, 8, 102, 108, 97, 115,
            104, 86, 101, 114, 2, 0, 31, 70, 77, 76, 69, 47, 51, 46, 48, 32, 40, 99, 111, 109, 112,
            97, 116, 105, 98, 108, 101, 59, 32, 70, 77, 83, 99, 47, 49, 46, 48, 41, 0, 6, 115, 119,
            102, 85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108,
            104, 111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 5, 116, 99,
            85, 114, 108, 2, 0, 28, 114, 116, 109, 112, 58, 47, 47, 108, 111, 99, 97, 108, 104,
            111, 115, 116, 58, 49, 57, 51, 53, 47, 104, 97, 114, 108, 97, 110, 0, 0, 9,
        ];

        let mut bytes_reader = BytesReader::new(BytesMut::new());
        bytes_reader.extend_from_slice(&data);
        let mut amf_reader = Amf0Reader::new(bytes_reader);

        let command_name = amf_reader.read_with_type(amf0_markers::STRING).unwrap();
        assert_eq!(
            command_name,
            Amf0ValueType::UTF8String(String::from("connect"))
        );

        let transaction_id = amf_reader.read_with_type(amf0_markers::NUMBER).unwrap();
        assert_eq!(transaction_id, Amf0ValueType::Number(1.0));

        let command_obj_raw = amf_reader.read_with_type(amf0_markers::OBJECT).unwrap();
        let mut properties = HashMap::new();
        properties.insert(
            String::from("app"),
            Amf0ValueType::UTF8String(String::from("harlan")),
        );
        properties.insert(
            String::from("type"),
            Amf0ValueType::UTF8String(String::from("nonprivate")),
        );
        properties.insert(
            String::from("flashVer"),
            Amf0ValueType::UTF8String(String::from("FMLE/3.0 (compatible; FMSc/1.0)")),
        );
        properties.insert(
            String::from("swfUrl"),
            Amf0ValueType::UTF8String(String::from("rtmp://localhost:1935/harlan")),
        );
        properties.insert(
            String::from("tcUrl"),
            Amf0ValueType::UTF8String(String::from("rtmp://localhost:1935/harlan")),
        );
        assert_eq!(command_obj_raw, Amf0ValueType::Object(properties));

        let bytes = amf_reader.reader.get_remaining_bytes();

        format!("{}","we");

         let others = amf_reader.read_all().unwrap();
    }
}
