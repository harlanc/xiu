use crate::amf0::amf0_markers;
use crate::amf0::amf0_reader::Amf0Reader;

use crate::amf0::define::Amf0ValueType;

pub enum Rtmp_Messages {
    AMF0_COMMAND {
        msg_stream_id: u32,
        command_name: Amf0ValueType,
        transaction_id: Amf0ValueType,
        command_object: Amf0ValueType,
        others: Vec<Amf0ValueType>,
    },
}
