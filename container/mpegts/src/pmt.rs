use super::pes;
use bytes::BytesMut;

pub struct Pmt {
    pid: u8,
    program_number: u16,
    version_number: u8,       //5 bits
    continuity_counter: u8,   //4i bits
    pcr_pid: u16,             //13 bits
    program_info_length: u16, //12 bits

    program_info: BytesMut,
    provider: [char; 64],
    name: [char; 64],
    stream_count: u8,
    pes: [pes::Pes; 4],
}

impl Pmt{
    //p49
    pub fn write(&mut self) {

    }
}
