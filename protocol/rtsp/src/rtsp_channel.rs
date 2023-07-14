use crate::rtp::errors::PackerError;
use crate::rtp::errors::UnPackerError;
use crate::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtp::rtcp::RTCP_SR;
use crate::rtp::utils::OnFrameFn;
use crate::rtp::utils::OnPacketFn;
use crate::rtsp_transport::ProtocolType;

use super::rtp::rtp_aac::RtpAacPacker;
use super::rtp::rtp_h264::RtpH264Packer;
use super::rtp::rtp_h265::RtpH265Packer;

use super::rtp::rtp_aac::RtpAacUnPacker;
use super::rtp::rtp_h264::RtpH264UnPacker;
use super::rtp::rtp_h265::RtpH265UnPacker;

use super::rtp::rtcp::rtcp_context::RtcpContext;
use super::rtp::rtcp::rtcp_sr::RtcpSenderReport;
use super::rtp::utils::TPacker;
use super::rtp::utils::TUnPacker;
use super::rtsp_codec::RtspCodecId;
use super::rtsp_codec::RtspCodecInfo;
use super::rtsp_transport::RtspTransport;
use crate::rtp::utils::Marshal;
use crate::rtp::utils::Unmarshal;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytes_writer::AsyncBytesWriter;
use bytesio::bytesio::TNetIO;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

pub trait TRtpFunc {
    fn create_packer(&mut self, writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>);
    fn create_unpacker(&mut self);
}

pub struct RtpChannel {
    codec_info: RtspCodecInfo,
    // pub rtp_packer: Option<Box<dyn TPacker>>,
    // The rtp packer will be used in a separate thread when
    // received rtp data using a separate UDP channel,
    // so here we add the Arc and Mutex
    pub rtp_packer: Option<Box<dyn TPacker>>,
    pub rtp_unpacker: Option<Box<dyn TUnPacker>>,
    ssrc: u32,
    init_sequence: u16,
}

#[derive(Default)]
pub struct RtcpChannel {
    recv_ctx: RtcpContext,
    send_ctx: RtcpContext,
    on_packet_handler: Option<OnPacketFn>,
}

impl RtpChannel {
    pub fn new(codec_info: RtspCodecInfo) -> Self {
        let ssrc: u32 = rand::thread_rng().gen();
        let mut rtp_channel = RtpChannel {
            codec_info,
            ssrc,
            rtp_packer: None,
            rtp_unpacker: None,
            init_sequence: 0,
        };
        rtp_channel.create_unpacker();
        rtp_channel
    }

    pub fn on_rtp(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.unpack(reader)?;
        }
        Ok(())
    }

    pub async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        if let Some(packer) = &mut self.rtp_packer {
            return packer.pack(nalus, timestamp).await;
        }
        Ok(())
    }

    pub fn on_frame_handler(&mut self, f: OnFrameFn) {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.on_frame_handler(f);
        }
    }

    pub fn on_packet_handler(&mut self, f: OnPacketFn) {
        if let Some(packer) = &mut self.rtp_packer {
            packer.on_packet_handler(f);
        }
    }
}

impl TRtpFunc for RtpChannel {
    fn create_unpacker(&mut self) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_unpacker = Some(Box::new(RtpH264UnPacker::default()));
            }
            RtspCodecId::H265 => {
                self.rtp_unpacker = Some(Box::new(RtpH265UnPacker::default()));
            }
            RtspCodecId::AAC => {
                self.rtp_unpacker = Some(Box::new(RtpAacUnPacker::default()));
            }
            RtspCodecId::G711A => {}
        }
    }
    fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_packer = Some(Box::new(RtpH264Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::H265 => {
                self.rtp_packer = Some(Box::new(RtpH265Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::AAC => {
                self.rtp_packer = Some(Box::new(RtpAacPacker::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::G711A => {}
        }
    }
}

impl RtcpChannel {
    pub fn on_rtcp(&mut self, reader: &mut BytesReader) {
        let mut reader_clone = BytesReader::new(reader.get_remaining_bytes());
        if let Ok(rtcp_header) = RtcpHeader::unmarshal(&mut reader_clone) {
            match rtcp_header.payload_type {
                RTCP_SR => {
                    if let Ok(sr) = RtcpSenderReport::unmarshal(reader) {
                        self.recv_ctx.received_sr(&sr);
                        self.send_rtcp_receier_report();
                    }
                }
                _ => {}
            }
        }
    }
    pub async fn send_rtcp_receier_report(&mut self) {
        let rr = self.recv_ctx.generate_rr();
        if let Ok(packet_bytesmut) = rr.marshal() {
            // if let Some(f) = &self.on_packet_handler {
            //     // log::info!("seq number: {}", packet.header.seq_number);
            //     f(self.io.clone(), packet_bytesmut).await?;
            // }
        }
    }

    pub fn on_packet_handler(&mut self, f: OnPacketFn) {
        self.on_packet_handler = Some(f);
    }
}
