use {
    super::errors::NetConnectionError, crate::amf0::amf0_reader::Amf0Reader,
    netio::bytes_reader::BytesReader,
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
