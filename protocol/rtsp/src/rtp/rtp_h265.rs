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

pub struct RtpH265Packer {
    header: RtpHeader,
    mtu: usize,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

impl RtpH265Packer {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        mtu: usize,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpH265Packer {
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

    pub async fn pack_fu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        /* NALU header
        0               1
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
        |F|    Type   |  LayerId  | TID |
        +-------------+-----------------+

        Forbidden zero(F) : 1 bit
        NAL unit type(Type) : 6 bits
        NUH layer ID(LayerId) : 6 bits
        NUH temporal ID plus 1 (TID) : 3 bits
        */
        let nalu_header_1st_byte = nalu_reader.read_u8()?;
        let nalu_header_2nd_byte = nalu_reader.read_u8()?;

        /* The PayloadHdr needs replace Type with the FU type value(49) */
        let payload_hdr: u16 = ((nalu_header_1st_byte as u16 & 0x81) | ((define::FU as u16) << 1))
            << 8
            | nalu_header_2nd_byte as u16;
        /* FU header
        +---------------+
        |0|1|2|3|4|5|6|7|
        +-+-+-+-+-+-+-+-+
        |S|E|   FuType  |
        +---------------+
        */
        /*set FuType from NALU header's Type */
        let mut fu_header = (nalu_header_1st_byte >> 1) & 0x3F | define::FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len: usize;

        while left_nalu_bytes > 0 {
            /* 3 = PayloadHdr(2 bytes) + FU header(1 byte) */
            if left_nalu_bytes + define::RTP_FIXED_HEADER_LEN <= self.mtu - 3 {
                fu_header = (nalu_header_1st_byte & 0x1F) | define::FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - define::RTP_FIXED_HEADER_LEN - 3;
            }

            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u16(payload_hdr);
            packet.payload.put_u8(fu_header);
            packet.payload.put(fu_payload);
            packet.header.marker = if fu_header & define::FU_END > 0 { 1 } else { 0 };

            if fu_header & define::FU_START > 0 {
                fu_header &= 0x7F
            }

            if let Some(f) = &self.on_packet_for_rtcp_handler {
                f(packet.clone()).await;
            }

            if let Some(f) = &self.on_packet_handler {
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
impl TPacker for RtpH265Packer {
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp;
        utils::split_annexb_and_process(nalus, self).await?;
        Ok(())
    }
    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpH265Packer {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[async_trait]
impl TVideoPacker for RtpH265Packer {
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        if nalu.len() + define::RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single(nalu).await?;
        } else {
            self.pack_fu(nalu).await?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct RtpH265UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    using_donl_field: bool,
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

#[async_trait]
impl TUnPacker for RtpH265UnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        self.timestamp = rtp_packet.header.timestamp;
        self.sequence_number = rtp_packet.header.seq_number;

        if let Some(packet_type) = rtp_packet.payload.first() {
            match *packet_type >> 1 & 0x3F {
                define::FU => {
                    return self.unpack_fu(rtp_packet.payload.clone());
                }
                define::AP => {
                    return self.unpack_ap(rtp_packet.payload);
                }
                define::PACI => return Ok(()),

                _ => {
                    return self.unpack_single(rtp_packet.payload.clone());
                }
            }
        }

        Ok(())
    }

    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl RtpH265UnPacker {
    pub fn new() -> Self {
        RtpH265UnPacker::default()
    }

    fn unpack_single(&mut self, payload: BytesMut) -> Result<(), UnPackerError> {
        let mut annexb_payload = BytesMut::new();
        annexb_payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
        annexb_payload.put(payload);

        if let Some(f) = &self.on_frame_handler {
            f(FrameData::Video {
                timestamp: self.timestamp,
                data: annexb_payload,
            })?;
        }
        Ok(())
    }

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                          RTP Header                           |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |      PayloadHdr (Type=48)     |           NALU 1 DONL         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |           NALU 1 Size         |            NALU 1 HDR         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |                         NALU 1 Data . . .                     |
    |                                                               |
    +     . . .     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |               |  NALU 2 DOND  |            NALU 2 Size        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |          NALU 2 HDR           |                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+            NALU 2 Data        |
    |                                                               |
    |         . . .                 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    */

    fn unpack_ap(&mut self, rtp_payload: BytesMut) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        /*read PayloadHdr*/
        payload_reader.read_bytes(2)?;

        while !payload_reader.is_empty() {
            if self.using_donl_field {
                /*read DONL*/
                payload_reader.read_bytes(2)?;
            }
            /*read NALU Size*/
            let nalu_len = payload_reader.read_u16::<BigEndian>()? as usize;
            /*read NALU HDR + Data */
            let nalu = payload_reader.read_bytes(nalu_len)?;

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

    /*
    0               1
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |F|    Type   |  LayerId  | TID |
    +-------------+-----------------+

    Forbidden zero(F) : 1 bit
    NAL unit type(Type) : 6 bits
    NUH layer ID(LayerId) : 6 bits
    NUH temporal ID plus 1 (TID) : 3 bits
    */

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     PayloadHdr (Type=49)      |    FU header  |  DONL (cond)  |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    |  DONL (cond)  |                                               |
    |-+-+-+-+-+-+-+-+                                               |
    |                           FU payload                          |
    |                                                               |
    |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /* FU header */
    +---------------+
    |0|1|2|3|4|5|6|7|
    +-+-+-+-+-+-+-+-+
    |S|E|   FuType  |
    +---------------+
    */
    fn unpack_fu(&mut self, rtp_payload: BytesMut) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let payload_header_1st_byte = payload_reader.read_u8()?;
        let payload_header_2nd_byte = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;
        if self.using_donl_field {
            payload_reader.read_bytes(2)?;
        }

        if utils::is_fu_start(fu_header) {
            /*set NAL UNIT type 2 bytes */
            //replace Type of PayloadHdr with the FuType of FU header
            let nal_1st_byte = (payload_header_1st_byte & 0x81) | ((fu_header & 0x3F) << 1);
            self.fu_buffer.put_u8(nal_1st_byte);
            self.fu_buffer.put_u8(payload_header_2nd_byte);
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
}

impl TRtpReceiverForRtcp for RtpH265UnPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}
