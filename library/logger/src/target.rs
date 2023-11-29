use {
    anyhow::Result,
    std::sync::{Arc, Mutex},
    std::{fs::File, io},
};

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
            Ok(rv) => Ok(rv),
            Err(err) => {
                println!("write err {err}");
                Ok(0)
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        println!("flush");
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

        println!("{newdate}");
        println!("{cur_number}");
    }



    // #[test]
    // fn test_job_scheduler_ng() {
    //     let mut sched = JobScheduler::new();

    //     sched.add(Job::new("0 0 * * * *".parse().unwrap(), || {
    //         let dt: DateTime<Local> = Local::now();
    //         let cur_number = format!(
    //             "{}-{:02}-{:02} {:02}:{:02}:00",
    //             dt.year(),
    //             dt.month(),
    //             dt.day(),
    //             dt.hour(),
    //             dt.minute()
    //         );
    //         println!("time number: {cur_number}");
    //     }));

    //     loop {
    //         sched.tick();
    //         std::thread::sleep(Duration::from_millis(500));
    //     }
    // }
}
