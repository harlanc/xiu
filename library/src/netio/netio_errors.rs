
use std::io;
use tokio::time::Elapsed;
pub enum NetIOErrorValue {
    NotEnoughBytes,
    EmptyStream,
    IOError(io::Error),
    TimeoutError(Elapsed),
    NoneReturn,
}
pub struct NetIOError {
    pub value: NetIOErrorValue,
}

impl From<NetIOErrorValue> for NetIOError {
    fn from(val: NetIOErrorValue) -> Self {
        NetIOError { value: val }
    }
}

impl From<io::Error> for NetIOError {
    fn from(error: io::Error) -> Self {
        NetIOError {
            value: NetIOErrorValue::IOError(error),
        }
    }
}

impl From<Elapsed> for NetIOError {
    fn from(error: Elapsed) -> Self {
        NetIOError {
            value: NetIOErrorValue::TimeoutError(error),
        }
    }
}