use failure::Fail;

pub struct LogError {
    pub value: LogErrorValue,
}

#[derive(Debug, Fail)]
pub enum LogErrorValue {
    #[fail(display = "write file error")]
    IOError(std::io::Error),
}
