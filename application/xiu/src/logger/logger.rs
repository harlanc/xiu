use super::target::FileTarget;

use anyhow::Result;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::{fs, fs::File, io, path::Path};

use crate::logger::target;
use env_logger::{Builder, Env, Target};
use std::env;

use chrono::prelude::*;

use job_scheduler::{Job, JobScheduler};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use std::thread;

#[derive(Clone)]
pub enum Rotate {
    Day,
    Hour,
    Minute,
}

fn get_log_file_name(rotate: Rotate) -> String {
    let local_time: DateTime<Local> = Local::now();
    let file_name: String;
    match rotate {
        Rotate::Day => {
            file_name = format!(
                "{}{:02}{:02}0000",
                local_time.year(),
                local_time.month(),
                local_time.day(),
            );
        }
        Rotate::Hour => {
            file_name = format!(
                "{}{:02}{:02}{:02}00",
                local_time.year(),
                local_time.month(),
                local_time.day(),
                local_time.hour(),
            );
        }
        Rotate::Minute => {
            file_name = format!(
                "{}{:02}{:02}{:02}{:02}",
                local_time.year(),
                local_time.month(),
                local_time.day(),
                local_time.hour(),
                local_time.minute()
            );
        }
    }
    file_name
}

pub fn gen_log_file(rotate: Rotate, path: String) -> Result<File> {
    let file_name = get_log_file_name(rotate);
    let full_path = format!("{}/{}.log", path, file_name);
    println!("file_name: {}", full_path);

    if !Path::new(&full_path).exists() {
        Ok(File::create(full_path)?)
    } else {
        Ok(File::open(full_path)?)
    }
}

pub fn gen_log_file_thread_run(
    file_handler: Arc<Mutex<File>>,
    rotate: Rotate,
    path: String,
    r: Receiver<bool>,
) {
    thread::spawn(move || {
        let mut sched = JobScheduler::new();

        let scheduler_rule = match rotate {
            Rotate::Minute => "0 * * * * *",
            Rotate::Hour => "0 0 * * * *",
            Rotate::Day => "0 0 0 * * *",
        };

        sched.add(Job::new(scheduler_rule.parse().unwrap(), || {
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

            if let Ok(file) = gen_log_file(rotate.to_owned(), path.to_owned()) {
                let mut state = file_handler.lock().expect("Could not lock mutex");
                *state = file;
            }
        }));
        let duration = Duration::from_millis(500);
        loop {
            sched.tick();
            if r.recv_timeout(duration).is_ok() {
                return;
            }
        }
    });
}

pub struct Logger {
    close_sender: Sender<bool>,
}

impl Logger {
    pub fn new(rotate: Rotate, path: String) -> Result<Logger> {
        if let Err(err) = fs::create_dir_all(path.clone()) {
            println!("cannot create folder: {}, err: {}", path, err);
        }
        let file = gen_log_file(rotate.clone(), path.clone())?;
        let target = FileTarget::new(file)?;

        let handler = target.cur_file_handler.clone();
        let (send, receiver) = channel::<bool>();

        gen_log_file_thread_run(handler, rotate, path, receiver);

        let env = Env::default()
            .filter_or("MY_LOG_LEVEL", "trace")
            // Normally using a pipe as a target would mean a value of false, but this forces it to be true.
            .write_style_or("MY_LOG_STYLE", "always");

        Builder::from_env(env)
            // The Sender of the channel is given to the logger
            // A wrapper is needed, because the `Sender` itself doesn't implement `std::io::Write`.
            .target(Target::Pipe(Box::new(target)))
            .init();

        Ok(Self { close_sender: send })
    }
    pub fn close(&self) {
        if let Err(err) = self.close_sender.send(true) {
            println!("Logger close err :{}", err);
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::logger::target;
    use env_logger::{Builder, Env, Target};
    use std::env;
    // use crate::logger::logger::
    use super::Rotate;
    use crate::logger::logger::Logger;

    use std::sync::mpsc::channel;
    use std::time::Duration;

    #[test]
    fn test_log() {
        let logger = Logger::new(Rotate::Minute, String::from("./logs")).unwrap();

        let mut recorder = 0;

        loop {
            std::thread::sleep(Duration::from_millis(500));
            log::trace!("some trace log");
            log::debug!("some debug log");
            log::info!("some information log");
            log::warn!("some warning log");
            log::error!("some error log");
            recorder += 1;
            if recorder > 180 {
                logger.close();
                break;
            }
        }
    }
}
