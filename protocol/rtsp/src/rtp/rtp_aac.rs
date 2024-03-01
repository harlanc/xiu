use super::errors::PackerError;
use super::errors::UnPackerError;
use super::utils::OnFrameFn;
use super::utils::OnRtpPacketFn;
use super::utils::OnRtpPacketFn2;
use super::utils::TPacker;

use super::utils::TRtpReceiverForRtcp;
use super::utils::TUnPacker;
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

// pub type OnPacketFn = fn(BytesMut) -> Result<(), PackerError>;

pub struct RtpAacPacker {
    header: RtpHeader,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpAacPacker {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpAacPacker {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            io,
            on_packet_handler: None,
            on_packet_for_rtcp_handler: None,
        }
    }
}
#[async_trait]
impl TPacker for RtpAacPacker {
    async fn pack(&mut self, data: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp;

        let data_len = data.len();
        let mut packet = RtpPacket::new(self.header.clone());
        packet.payload.put_u16(16);
        packet.payload.put_u8((data_len >> 5) as u8);
        packet.payload.put_u8(((data_len & 0x1F) << 3) as u8);
        packet.payload.put(data);

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(packet.clone()).await;
        }

        if let Some(f) = &self.on_packet_handler {
            f(self.io.clone(), packet).await?;
        }

        self.header.seq_number += 1;

        Ok(())
    }

    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[derive(Default)]
pub struct RtpAacUnPacker {
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

// +---------+-----------+-----------+---------------+
// | RTP     | AU Header | Auxiliary | Access Unit   |
// | Header  | Section   | Section   | Data Section  |
// +---------+-----------+-----------+---------------+
// 	<----------RTP Packet Payload----------->
//
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+
// |AU-headers-length|AU-header|AU-header|      |AU-header|padding|
// |                 |   (1)   |   (2)   |      |   (n)   | bits  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+

// Au-headers-length 2 bytes

impl RtpAacUnPacker {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[async_trait]
impl TUnPacker for RtpAacUnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        let mut reader_payload = BytesReader::new(rtp_packet.payload);

        let au_headers_length = (reader_payload.read_u16::<BigEndian>()? + 7) / 8;
        let au_header_length = 2;
        let aus_number = au_headers_length / au_header_length;

        let mut au_lengths = Vec::new();
        for _ in 0..aus_number {
            let au_length = (((reader_payload.read_u8()? as u16) << 8)
                | ((reader_payload.read_u8()? as u16) & 0xF8)) as usize;
            au_lengths.push(au_length / 8);
        }

        log::debug!(
            "send audio : au_headers_length :{}, aus_number: {}, au_lengths: {:?}",
            au_headers_length,
            aus_number,
            au_lengths,
        );

        for (i, item) in au_lengths.iter().enumerate() {
            let au_data = reader_payload.read_bytes(*item)?;
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Audio {
                    timestamp: rtp_packet.header.timestamp + i as u32 * 1024,
                    data: au_data,
                })?;
            }
        }

        Ok(())
    }
    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacUnPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}
