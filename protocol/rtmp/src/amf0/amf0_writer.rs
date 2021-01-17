
use super::Amf0ValueType;
use super::Amf0WriteError;
use super::amf0_markers;
use byteorder::BigEndian;



fn write_any(value: &Amf0ValueType, bytes: &mut Vec<u8>) -> Result<(), Amf0WriteError> {
    match *value {
        Amf0ValueType::Boolean(ref val) => Ok(write_bool(&val, bytes)),
        Amf0ValueType::Null => Ok(write_null(bytes)),
        Amf0ValueType::Number(ref val) => write_number(&val, bytes),
        Amf0ValueType::UTF8String(ref val) => write_string(&val, bytes),
        Amf0ValueType::Object(ref val) => write_object(&val, bytes),
        _ => Ok(())
    }
}

fn write_number(value: &f64, bytes: &mut Vec<u8>) -> Result<(), Amf0WriteError> {
    bytes.push(amf0_markers::NUMBER);
    bytes.write_f64::<BigEndian>(value.clone())?;
    Ok(())
}

fn write_bool(value: &bool, bytes: &mut Vec<u8>) {
    bytes.push(amf0_markers::BOOLEAN);
    bytes.push((value.clone()) as u8);
}

fn write_string(value: &String, bytes: &mut Vec<u8>) -> Result<(), Amf0WriteError> {
    if value.len() > (u16::max_value() as usize) {
        return Err(Amf0WriteError::NormalStringTooLong);
    }

    bytes.push(amf0_markers::STRING);
    bytes.write_u16::<BigEndian>(value.len() as u16)?;
    bytes.extend(value.as_bytes());
    Ok(())
}

fn write_null(bytes: &mut Vec<u8>) {
    bytes.push(amf0_markers::NUMBER);
}

fn write_object_eof(bytes: &mut Vec<u8>) -> Result<(), Amf0WriteError> {
    bytes.write_u24::<BigEndian>(amf0_markers::OBJECT_END)?;
    Ok(())
}

fn write_object(properties: &UnOrderedMap, bytes: &mut Vec<u8>) -> Result<(), Amf0WriteError> {
    bytes.push(amf0_markers::OBJECT_END);

    let len: usize = properties.len();

    for i in 0..len {
        let obj: Amf0Object = properties.get(i);
        bytes.write_u16::<BigEndian>(obj.key.len() as u16)?;
        bytes.extend(obj.key.as_bytes());
        write_any(obj.Value, bytes);
    }

    write_object_eof(bytes);
    Ok(())
}
