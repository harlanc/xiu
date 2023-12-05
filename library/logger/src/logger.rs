use {
    super::target::FileTarget,
    anyhow::Result,
    chrono::prelude::*,
    env_logger::{Builder, Env, Target},
    job_scheduler_ng::{Job, JobScheduler},
    std::{
        env, fs,
        fs::{File, OpenOptions},
        path::Path,
        str::FromStr,
        sync::{
            mpsc::{channel, Receiver, Sender},
            Arc, Mutex,
        },
        thread,
        time::Duration,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum Rotate {
    Day,
    Hour,
    Minute,
}

impl FromStr for Rotate {
    type Err = ();
    fn from_str(input: &str) -> Result<Rotate, Self::Err> {
        match input {
            "day" => Ok(Rotate::Day),
            "hour" => Ok(Rotate::Hour),
            "minute" => Ok(Rotate::Minute),
            _ => Err(()),
        }
    }
}

fn get_log_file_name(rotate: Rotate) -> String {
    let local_time: DateTime<Local> = Local::now();
    match rotate {
        Rotate::Day => {
            format!(
                "{}{:02}{:02}0000",
                local_time.year(),
                local_time.month(),
                local_time.day(),
            )
        }
        Rotate::Hour => {
            format!(
                "{}{:02}{:02}{:02}00",
                local_time.year(),
                local_time.month(),
                local_time.day(),
                local_time.hour(),
            )
        }
        Rotate::Minute => {
            format!(
                "{}{:02}{:02}{:02}{:02}",
                local_time.year(),
                local_time.month(),
                local_time.day(),
                local_time.hour(),
                local_time.minute()
            )
        }
    }
}

pub fn gen_log_file(rotate: Rotate, path: String) -> Result<File> {
    let file_name = get_log_file_name(rotate);
    let full_path = format!("{path}/{file_name}.log");
    // println!("file_name: {}", full_path);
    if !Path::new(&full_path).exists() {
        //println!("create file : {}", full_path);
        Ok(File::create(full_path)?)
    } else {
        //println!("open file : {}", full_path);
        let file = OpenOptions::new().append(true).open(full_path)?;
        Ok(file)
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
            println!("time number: {cur_number}");

            match gen_log_file(rotate.to_owned(), path.to_owned()) {
                Ok(file) => {
                    let mut state = file_handler.lock().expect("Could not lock mutex");
                    *state = file;
                }
                Err(err) => {
                    println!("gen_log_file err : {err}");
                }
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
#[derive(Default)]
pub struct Logger {
    close_sender: Option<Sender<bool>>,
}

impl Logger {
    pub fn new(level: &String, rotate: Option<Rotate>, path: Option<String>) -> Result<Logger> {
        if rotate.is_none() || path.is_none() {
            env::set_var("RUST_LOG", level);
            env_logger::init();
            return Ok(Self {
                ..Default::default()
            });
        }

        let env = Env::default()
            .filter_or("MY_LOG_LEVEL", level)
            // Normally using a pipe as a target would mean a value of false, but this forces it to be true.
            .write_style_or("MY_LOG_STYLE", "always");

        let path_val = path.unwrap();
        let rotate_val = rotate.unwrap();

        if let Err(err) = fs::create_dir_all(path_val.clone()) {
            println!("cannot create folder: {path_val}, err: {err}");
        }
        let file = gen_log_file(rotate_val.clone(), path_val.clone())?;
        let target = FileTarget::new(file)?;

        let handler = target.cur_file_handler.clone();
        let (send, receiver) = channel::<bool>();

        gen_log_file_thread_run(handler, rotate_val, path_val, receiver);

        Builder::from_env(env)
            .target(Target::Pipe(Box::new(target)))
            .init();

        Ok(Self {
            close_sender: Some(send),
        })
    }
    pub fn stop(&self) {
        if let Some(sender) = &self.close_sender {
            if let Err(err) = sender.send(true) {
                println!("Logger close err :{err}");
            }
        }
    }
}
#[cfg(test)]
mod tests {

    use super::Logger;
    use super::Rotate;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::Duration;

    #[test]
    fn test_log() {
        let logger = Logger::new(
            &String::from("info"),
            Some(Rotate::Minute),
            Some(String::from("./logs")),
        )
        .unwrap();

        let mut recorder = 0;

        loop {
            std::thread::sleep(Duration::from_millis(500));
            log::trace!("some trace log");
            log::debug!("some debug log");
            log::info!("some information log");
            log::warn!("some warning log");
            log::error!("some error log");
            recorder += 1;
            if recorder > 10 {
                logger.stop();
                break;
            }
        }
    }
    #[test]
    fn test_write_file() {
        match OpenOptions::new().append(true).open("abc.txt") {
            Ok(mut file) => {
                if let Err(err) = file.write_all(&[b'h', b'e', b'l', b'l', b'o', b'o']) {
                    println!("file write_all: {err}");
                }
            }
            Err(err) => {
                println!("file create: {err}");
            }
        }
    }
}
