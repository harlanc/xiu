use super::errors::NetConnectionError;
use crate::amf0::amf0_reader::Amf0Reader;
use crate::amf0::define::Amf0ValueType;

use liverust_lib::netio::{
    errors::IOWriteError, reader::NetworkReader, reader::Reader, writer::Writer,
};
pub struct NetConnectionReader {
    reader: Reader,
    amf0_reader: Amf0Reader,
}

impl NetConnectionReader {
    fn onconnect(&mut self) -> Result<(), NetConnectionError> {
        Ok(())
    }
}
