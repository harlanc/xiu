
use crate::amf0::amf0_reader::Amf0Reader;
use crate::amf0::define::Amf0ValueType;
use super::errors::NetConnectionError;

use liverust_lib::netio::{
    errors::IOReadError,
    reader::NetworkReader,
    reader::Reader,
    writer::{IOWriteError, Writer},
};
pub struct NetConnectionReader {
    reader: Reader,
    amf0_reader: Amf0Reader,
}


impl NetConnectionReader{
    fn onconnect(&mut self) -> Result<(), NetConnectionError>{

        Ok(())
    }
}