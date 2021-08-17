use {
    chrono::prelude::*,
    std::{fs, fs::File, io, path::Path},
};

pub enum Rotate {
    Day,
    Hour,
    Minute,
}

pub struct FileTarget {
    rotate: Rotate,
    path: String,
    cur_file_handler: Option<File>,
}

impl FileTarget {
    pub fn new(rotate: Rotate, path: String) -> Self {
        fs::create_dir_all(path.clone()).unwrap();
        Self {
            rotate,
            path,
            cur_file_handler: None,
        }
    }
    fn get_log_file_name(&mut self) -> String {
        let local_time: DateTime<Local> = Local::now();
        let file_name: String;
        match self.rotate {
            Rotate::Day => {
                file_name = format!(
                    "{}-{:02}-{:02} 00:00:00",
                    local_time.year(),
                    local_time.month(),
                    local_time.day(),
                );
            }
            Rotate::Hour => {
                file_name = format!(
                    "{}-{:02}-{:02} {:02}:00:00",
                    local_time.year(),
                    local_time.month(),
                    local_time.day(),
                    local_time.hour(),
                );
            }
            Rotate::Minute => {
                file_name = format!(
                    "{}-{:02}-{:02} {:02}:{:02}:00",
                    local_time.year(),
                    local_time.month(),
                    local_time.day(),
                    local_time.hour(),
                    local_time.minute()
                );
            }
        }
        return file_name;
    }
}

impl io::Write for FileTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let file_name = self.get_log_file_name();
        let full_path = format!("{}/{}", self.path, file_name);

        if !Path::new(&full_path).exists() {
            self.cur_file_handler = Some(File::create(full_path).unwrap());
        }

        if let Some(file_handler) = &mut self.cur_file_handler {
            match file_handler.write(buf) {
                Ok(rv) => {
                    return Ok(rv);
                }
                Err(err) => {
                    print!("write err {}", err);
                    return Ok(0);
                }
            }
        }
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use chrono::prelude::*;

    #[test]

    fn test_chrono() {
        // Convert the timestamp string into an i64
        let timestamp = "1524820690".parse::<i64>().unwrap();

        // Create a NaiveDateTime from the timestamp
        let naive = NaiveDateTime::from_timestamp(timestamp, 0);

        // Create a normal DateTime from the NaiveDateTime
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

        // Format the datetime how you want
        let newdate = datetime.format("%Y-%m-%d %H:%M:%S");

        // Print the newly formatted date and time

        let dt: DateTime<Local> = Local::now();

        let cur_number = format!(
            "{}-{:02}-{:02} {:02}:{:02}:00",
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute()
        );

        println!("{}\n", newdate);
        print!("{}\n", cur_number);
    }

    use crate::logger::logger;
    use env_logger::{Builder, Env, Target};

    #[test]
    fn test_log() {
        let env = Env::default()
            .filter_or("MY_LOG_LEVEL", "trace")
            // Normally using a pipe as a target would mean a value of false, but this forces it to be true.
            .write_style_or("MY_LOG_STYLE", "always");

        // Create the channel for the log messages

        Builder::from_env(env)
            // The Sender of the channel is given to the logger
            // A wrapper is needed, because the `Sender` itself doesn't implement `std::io::Write`.
            .target(Target::Pipe(Box::new(logger::FileTarget {
                rotate: logger::Rotate::Minute,
                path: String::from("./logs"),
                cur_file_handler: None,
            })))
            .init();

        log::trace!("some trace log");
        log::debug!("some debug log");
        log::info!("some information log");
        log::warn!("some warning log");
        log::error!("some error log");
    }
}
