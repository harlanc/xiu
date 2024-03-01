use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use super::utils;
use super::utils::OnFrameFn;
use super::utils::OnRtpPacketFn;
use super::utils::OnRtpPacketFn2;
use super::utils::TPacker;
use super::utils::TRtpReceiverForRtcp;
use super::utils::TUnPacker;
use super::utils::TVideoPacker;
use super::utils::Unmarshal;
use super::RtpHeader;
use super::RtpPacket;
use async_trait::async_trait;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use bytesio::bytes_reader::BytesReader;
use bytesio::bytesio::TNetIO;
use std::sync::Arc;
use streamhub::define::FrameData;
use tokio::sync::Mutex;

pub struct RtpH264Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpH264Packer {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        mtu: usize,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpH264Packer {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                version: 2,
                ..Default::default()
            },
            mtu,
            io,
            on_packet_handler: None,
            on_packet_for_rtcp_handler: None,
        }
    }

    pub async fn pack_fu_a(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        let byte_1st = nalu_reader.read_u8()?;

        let fu_indicator: u8 = (byte_1st & 0xE0) | define::FU_A;
        let mut fu_header: u8 = (byte_1st & 0x1F) | define::FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len: usize;

        while left_nalu_bytes > 0 {
            if left_nalu_bytes + define::RTP_FIXED_HEADER_LEN <= self.mtu - 2 {
                fu_header = (byte_1st & 0x1F) | define::FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - define::RTP_FIXED_HEADER_LEN - 2;
            }

            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u8(fu_indicator);
            packet.payload.put_u8(fu_header);

            if fu_header & define::FU_START > 0 {
                fu_header &= 0x7F
            }

            packet.payload.put(fu_payload);
            packet.header.marker = if fu_header & define::FU_END > 0 { 1 } else { 0 };

            if let Some(f) = &self.on_packet_for_rtcp_handler {
                f(packet.clone()).await;
            }

            if let Some(f) = &self.on_packet_handler {
                // log::info!("seq number: {}", packet.header.seq_number);
                f(self.io.clone(), packet).await?;
            }

            left_nalu_bytes = nalu_reader.len();
            self.header.seq_number += 1;
        }

        Ok(())
    }
    pub async fn pack_single(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = 1;
        packet.payload.put(nalu);

        // let packet_bytesmut = packet.marshal()?;
        self.header.seq_number += 1;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(packet.clone()).await;
        }

        if let Some(f) = &self.on_packet_handler {
            return f(self.io.clone(), packet).await;
        }

        Ok(())
    }
}

#[async_trait]
impl TPacker for RtpH264Packer {
    //pack annexb h264 data
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp; // ((timestamp as u64 * self.clock_rate as u64) / 1000) as u32;
        utils::split_annexb_and_process(nalus, self).await?;
        Ok(())
    }

    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpH264Packer {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[async_trait]
impl TVideoPacker for RtpH264Packer {
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        if nalu.len() + define::RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single(nalu).await?;
        } else {
            self.pack_fu_a(nalu).await?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct RtpH264UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

#[async_trait]
impl TUnPacker for RtpH264UnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        self.timestamp = rtp_packet.header.timestamp;
        self.sequence_number = rtp_packet.header.seq_number;

        if let Some(packet_type) = rtp_packet.payload.first() {
            match *packet_type & 0x1F {
                1..=23 => {
                    return self.unpack_single(rtp_packet.payload.clone(), *packet_type);
                }
                define::STAP_A | define::STAP_B => {
                    return self.unpack_stap(rtp_packet.payload.clone(), *packet_type);
                }
                define::MTAP_16 | define::MTAP_24 => {
                    return self.unpack_mtap(rtp_packet.payload.clone(), *packet_type);
                }
                define::FU_A | define::FU_B => {
                    return self.unpack_fu(rtp_packet.payload.clone(), *packet_type);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl RtpH264UnPacker {
    pub fn new() -> Self {
        RtpH264UnPacker {
            ..Default::default()
        }
    }

    fn unpack_single(
        &mut self,
        payload: BytesMut,
        _t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        if let Some(f) = &self.on_frame_handler {
            let mut annexb_payload = BytesMut::new();
            annexb_payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            annexb_payload.put(payload);

            f(FrameData::Video {
                timestamp: self.timestamp,
                data: annexb_payload,
            })?;
        }
        Ok(())
    }

    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               |
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-A

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |               DON             |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-B

    // FU indicator
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |F|NRI|  Type   |
    // +---------------+

    // FU header
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |S|E|R|  Type   |
    // +---------------+
    fn unpack_fu(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let fu_indicator = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;

        if t == define::FU_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        if utils::is_fu_start(fu_header) {
            self.fu_buffer
                .put_u8((fu_indicator & 0xE0) | (fu_header & 0x1F))
        }

        self.fu_buffer.put(payload_reader.extract_remaining_bytes());

        if utils::is_fu_end(fu_header) {
            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            payload.put(self.fu_buffer.clone());
            self.fu_buffer.clear();
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }

        Ok(())
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-A NAL HDR |         NALU 1 Size           | NALU 1 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 1 Data                           |
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 2 Data                           |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-A
    //   containing two single-time aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-B NAL HDR | DON                           | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | NALU 1 Size   | NALU 1 HDR    | NALU 1 Data                   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                       NALU 2 Data                             |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-B
    //   containing two single-time aggregation units

    fn unpack_stap(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        //STAP-A / STAP-B HDR
        payload_reader.read_u8()?;

        if t == define::STAP_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        while !payload_reader.is_empty() {
            let length = payload_reader.read_u16::<BigEndian>()? as usize;
            let nalu = payload_reader.read_bytes(length)?;

            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            payload.put(nalu);
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }
        Ok(())
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP16 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offset        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 HDR   |  NALU 1 DATA                                  |
    // +-+-+-+-+-+-+-+-+                                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset        |  NALU 2 HDR   |  NALU 2 DATA  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP16 containing two multi-time
    //   aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP24 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offs          |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |NALU 1 TS offs |  NALU 1 HDR   |  NALU 1 DATA                  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset                        |  NALU 2 HDR   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 2 DATA                                                  |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP24 containing two multi-time
    //   aggregation units

    fn unpack_mtap(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        //read NAL HDR
        payload_reader.read_u8()?;
        //read decoding_order_number_base
        payload_reader.read_u16::<BigEndian>()?;

        while !payload_reader.is_empty() {
            //read nalu size
            let nalu_size = payload_reader.read_u16::<BigEndian>()? as usize;
            // read dond
            payload_reader.read_u8()?;
            // read TS offs
            let (ts, ts_bytes) = if t == define::MTAP_16 {
                (payload_reader.read_u16::<BigEndian>()? as u32, 2_usize)
            } else if t == define::MTAP_24 {
                (payload_reader.read_u24::<BigEndian>()?, 3_usize)
            } else {
                log::warn!("should not be here!");
                (0, 0)
            };
            assert!(ts != 0);
            let nalu = payload_reader.read_bytes(nalu_size - ts_bytes - 1)?;

            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            payload.put(nalu);
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }

        Ok(())
    }
}

impl TRtpReceiverForRtcp for RtpH264UnPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}
