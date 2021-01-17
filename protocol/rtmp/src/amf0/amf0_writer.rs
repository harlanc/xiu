use super::amf0_markers;
use super::Amf0ValueType;
use super::Amf0WriteError;
use byteorder::BigEndian;

use super::error::Amf0WriteErrorValue;
use super::define::UnOrderedMap;


use liverust_lib::netio::{reader::Reader, writer::Writer};

fn write_any(value: &Amf0ValueType, writer: &mut Writer) -> Result<(), Amf0WriteError> {
    match *value {
        Amf0ValueType::Boolean(ref val) => write_bool(&val, writer),
        Amf0ValueType::Null => write_null(writer),
        Amf0ValueType::Number(ref val) => write_number(&val, writer),
        Amf0ValueType::UTF8String(ref val) => write_string(&val, writer),
        Amf0ValueType::Object(ref val) => write_object(&val, writer),
        _ => Ok(()),
    }
}

fn write_number(value: &f64, writer: &mut Writer) -> Result<(), Amf0WriteError> {
    writer.write_u8(amf0_markers::NUMBER)?;
    writer.write_f64::<BigEndian>(value.clone())?;
    Ok(())
}

fn write_bool(value: &bool, writer: &mut Writer) -> Result<(), Amf0WriteError> {
    writer.write_u8(amf0_markers::BOOLEAN)?;
    writer.write_u8(value.clone() as u8)?;
    Ok(())
}

fn write_string(value: &String, writer: &mut Writer) -> Result<(), Amf0WriteError> {
    if value.len() > (u16::max_value() as usize) {
        return Err(Amf0WriteError {
            value: Amf0WriteErrorValue::NormalStringTooLong,
        });
    }

    writer.write_u8(amf0_markers::STRING)?;
    writer.write_u16::<BigEndian>(value.len() as u16)?;
    writer.write(value.as_bytes())?;

    Ok(())
}

fn write_null(writer: &mut Writer) -> Result<(), Amf0WriteError> {
    writer.write_u8(amf0_markers::NULL)?;
    Ok(())
}

fn write_object_eof(writer: &mut Writer) -> Result<(), Amf0WriteError> {
    writer.write_u24::<BigEndian>(amf0_markers::OBJECT_END as u32)?;
    Ok(())
}

fn write_object(properties: &UnOrderedMap, writer: &mut Writer) -> Result<(), Amf0WriteError> {
 
    writer.write_u8(amf0_markers::OBJECT_END)?;

    let len: usize = properties.len();

    for i in 0..len {
        let obj = properties.get(i);
        writer.write_u16::<BigEndian>(obj.key.len() as u16)?;
        writer.write(obj.key.as_bytes())?;

        write_any(&obj.value, writer)?;
    }

    write_object_eof(writer);
    Ok(())
}
