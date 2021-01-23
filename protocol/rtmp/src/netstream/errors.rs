

use failure::{Backtrace, Fail};
use std::fmt;
use std::io;
use crate::amf0::error::{Amf0WriteError, Amf0WriteErrorValue};


pub struct NetStreamError {
    pub value: NetStreamErrorValue,
}


pub enum NetStreamErrorValue {
 

    Amf0WriteError(Amf0WriteError),

    InvalidMaxChunkSize { chunk_size: usize },


}




impl From<Amf0WriteError> for NetStreamError {
    fn from(error: Amf0WriteError) -> Self {
        NetStreamError {
            value: NetStreamErrorValue::Amf0WriteError(error),
        }
    }
}
