use {
    super::errors::NetConnectionError, crate::amf0::amf0_reader::Amf0Reader,
    bytesio::bytes_reader::BytesReader,
};

#[allow(dead_code)]
pub struct NetConnectionReader {
    reader: BytesReader,
    amf0_reader: Amf0Reader,
}

impl NetConnectionReader {
    #[allow(dead_code)]
    fn onconnect(&mut self) -> Result<(), NetConnectionError> {
        Ok(())
    }
}
