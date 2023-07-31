pub const ANNEXB_NALU_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

pub type RtpNalType = u8;
//H264
pub const STAP_A: RtpNalType = 24;
pub const STAP_B: RtpNalType = 25;
pub const MTAP_16: RtpNalType = 26;
pub const MTAP_24: RtpNalType = 27;
pub const FU_A: RtpNalType = 28;
pub const FU_B: RtpNalType = 29;
// H265
pub const AP: RtpNalType = 48; //Aggregation Packets
pub const FU: RtpNalType = 49; //Fragmentation Units
pub const PACI: RtpNalType = 50;

pub const FU_START: u8 = 0x80;
pub const FU_END: u8 = 0x40;

pub const RTP_FIXED_HEADER_LEN: usize = 12;
