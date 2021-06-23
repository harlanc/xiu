use super::pmt;
pub struct Pat {
    transport_stream_id: u16,
    version_number: u8,     //5bits
    continuity_counter: u8, //s4 bits

    pmt_count: u8,

    pmt: [pmt::Pmt; 4],
}
