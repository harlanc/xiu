use crate::amf0::amf0_markers;
use crate::amf0::amf0_reader::Amf0Reader;

use crate::amf0::define::Amf0ValueType;

pub struct SetPeerBandwidthProperties {
    window_size: u32,
    limit_type: u8,
}

impl SetPeerBandwidthProperties {
    pub fn new(window_size: u32, limit_type: u8) -> Self {
        Self {
            window_size: window_size,
            limit_type: limit_type,
        }
    }
}

pub enum Rtmp_Messages {
    AMF0_COMMAND {
        msg_stream_id: u32,
        command_name: Amf0ValueType,
        transaction_id: Amf0ValueType,
        command_object: Amf0ValueType,
        others: Vec<Amf0ValueType>,
    },
    SET_CHUNK_SIZE {
        chunk_size: u32,
    },
    ABORT_MESSAGE {
        chunk_stream_id: u32,
    },
    ACKNOWLEDGEMENT {
        sequence_number: u32,
    },
    WINDOW_ACKNOWLEDGEMENT_SIZE {
        size: u32,
    },
    SET_PEER_BANDWIDTH {
        properties: SetPeerBandwidthProperties,
    },
}
