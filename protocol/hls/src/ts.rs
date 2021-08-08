use super::errors::MediaError;
use bytes::BytesMut;
use networkio::bytes_writer::BytesWriter;
use std::{fs::File, io::Write};
pub struct Ts {
    ts_number: u32,
}

impl Ts {
    pub fn new() -> Self {
        Self { ts_number: 0 }
    }
    pub fn write(&mut self, data: BytesMut) -> Result<String, MediaError> {
        let ts_file_name = format!("{}.ts", self.ts_number);
        self.ts_number += 1;

        let mut ts_file_handler = File::create(ts_file_name.clone())?;
        ts_file_handler.write_all(&data[..])?;

        Ok(ts_file_name)
    }
}
