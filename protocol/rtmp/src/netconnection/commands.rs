

use crate::amf0::amf0_writer::Amf0Writer;
use liverust_lib::netio::writer::Writer;
pub struct NetConnection {
    writer: Writer,
    amf0_writer: Amf0Writer,
}

impl NetConnection {

}