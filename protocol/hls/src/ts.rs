use {
    super::errors::MediaError,
    bytes::BytesMut,
    std::{fs, fs::File, io::Write},
};

pub struct Ts {
    ts_number: u32,
    ts_directory: String,
}

impl Ts {
    pub fn new(app_name: String, stream_name: String) -> Self {
        let exe_directory = if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.pop();
            exe_path.to_string_lossy().to_string()
        } else {
            log::error!("cannot get current exe path, using /app");
            "/app".to_string()
        };

        let ts_directory = format!("{exe_directory}/{app_name}/{stream_name}");
        fs::create_dir_all(ts_directory.clone()).unwrap();

        log::info!("ts folder: {ts_directory}");

        Self {
            ts_number: 0,
            ts_directory,
        }
    }
    pub fn write(&mut self, data: BytesMut) -> Result<(String, String), MediaError> {
        let ts_file_name = format!("{}.ts", self.ts_number);
        let ts_file_path = format!("{}/{}", self.ts_directory, ts_file_name);
        self.ts_number += 1;

        let mut ts_file_handler = File::create(ts_file_path.clone())?;
        ts_file_handler.write_all(&data[..])?;

        Ok((ts_file_name, ts_file_path))
    }
    pub fn delete(&mut self, ts_file_name: String) {
        fs::remove_file(ts_file_name).unwrap();
    }
}
