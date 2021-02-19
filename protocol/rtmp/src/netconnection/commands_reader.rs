use super::errors::NetConnectionError;
use crate::amf0::amf0_reader::Amf0Reader;
use crate::amf0::define::Amf0ValueType;

use liverust_lib::netio::{
    bytes_errors::BytesWriteError,netio::NetworkIO, bytes_reader::BytesReader, bytes_writer::BytesWriter,
};
pub struct NetConnectionReader {
    reader: BytesReader,
    amf0_reader: Amf0Reader,
}

impl NetConnectionReader {
    fn onconnect(&mut self) -> Result<(), NetConnectionError> {
        Ok(())
    }
}
