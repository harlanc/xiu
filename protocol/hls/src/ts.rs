use {
    super::errors::MediaError,
    bytes::BytesMut,
    std::{fs, fs::File, io::Write},
};

pub struct Ts {
    ts_number: u32,
    live_path: String,
}

impl Ts {
    pub fn new(path: String) -> Self {
        fs::create_dir_all(path.clone()).unwrap();

        Self {
            ts_number: 0,
            live_path: path,
        }
    }
    pub fn write(&mut self, data: BytesMut) -> Result<(String, String), MediaError> {
        let ts_file_name = format!("{}.ts", self.ts_number);
        let ts_file_path = format!("{}/{}", self.live_path, ts_file_name);
        self.ts_number += 1;

        let mut ts_file_handler = File::create(ts_file_path.clone())?;
        ts_file_handler.write_all(&data[..])?;

        Ok((ts_file_name, ts_file_path))
    }
    pub fn delete(&mut self, ts_file_name: String) {
        fs::remove_file(ts_file_name).unwrap();
    }
}
