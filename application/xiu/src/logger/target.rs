use {
    chrono::prelude::*,
    std::{fs, fs::File, io, path::Path},
};

use anyhow::Result;
use job_scheduler::{Job, JobScheduler};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use std::io::Error;
use std::io::ErrorKind;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::thread;

pub struct FileTarget {
    pub cur_file_handler: Arc<Mutex<File>>,
}

impl FileTarget {
    pub fn new(file: File) -> Result<Self> {
        Ok(Self {
            cur_file_handler: Arc::new(Mutex::new(file)),
        })
    }
}

impl io::Write for FileTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.cur_file_handler.lock().unwrap().write(buf) {
            Ok(rv) => {
                return Ok(rv);
            }
            Err(err) => {
                print!("write err {}", err);
                return Ok(0);
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file_handler = self.cur_file_handler.lock().unwrap();
        file_handler.flush()
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
        println!("{}", cur_number);
    }

    // use crate::logger::target;
    // use env_logger::{Builder, Env, Target};
    // use std::env;
    // // use crate::logger::logger::
    // use crate::logger::target::gen_log_file;
    // use crate::logger::target::gen_log_file_thread_run;
    // use std::sync::mpsc::channel;
    // use std::{fs, fs::File, io, path::Path};

    // #[test]
    // fn test_log() {
    //     let env = Env::default()
    //         .filter_or("MY_LOG_LEVEL", "trace")
    //         // Normally using a pipe as a target would mean a value of false, but this forces it to be true.
    //         .write_style_or("MY_LOG_STYLE", "always");

    //     let p = env::current_dir().unwrap();
    //     println!("cur path: {}", p.into_os_string().into_string().unwrap());

    //     let path = String::from("./logs");
    //     let rotate = target::Rotate::Minute;

    //     if let Err(err) = fs::create_dir_all(path.clone()) {
    //         println!("cannot create folder: {}, err: {}", path, err);
    //     }
    //     let file = gen_log_file(rotate.clone(), path.clone()).unwrap();

    //     let target = target::FileTarget::new(file).unwrap();

    //     let handler = target.cur_file_handler.clone();
    //     let (send, receive) = channel::<bool>();

    //     gen_log_file_thread_run(
    //         handler,
    //         target::Rotate::Minute,
    //         String::from("./logs"),
    //         receive,
    //     );

    //     Builder::from_env(env)
    //         // The Sender of the channel is given to the logger
    //         // A wrapper is needed, because the `Sender` itself doesn't implement `std::io::Write`.
    //         .target(Target::Pipe(Box::new(target)))
    //         .init();

    //     let mut recorder = 0;

    //     loop {
    //         std::thread::sleep(Duration::from_millis(500));
    //         log::trace!("some trace log");
    //         log::debug!("some debug log");
    //         log::info!("some information log");
    //         log::warn!("some warning log");
    //         log::error!("some error log");
    //         recorder += 1;
    //         if recorder > 180 {
    //             send.send(true);
    //             break;
    //         }
    //     }
    // }

    use job_scheduler::{Job, JobScheduler};
    use std::time::Duration;
    #[test]
    fn test_job_scheduler() {
        let mut sched = JobScheduler::new();

        sched.add(Job::new("0 0 * * * *".parse().unwrap(), || {
            let dt: DateTime<Local> = Local::now();

            let cur_number = format!(
                "{}-{:02}-{:02} {:02}:{:02}:00",
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute()
            );
            println!("time number: {}", cur_number);
        }));

        loop {
            sched.tick();

            std::thread::sleep(Duration::from_millis(500));
        }
    }
}
