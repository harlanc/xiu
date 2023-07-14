use crate::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtp::rtcp::RTCP_SR;
use crate::rtsp_channel::TRtpFunc;
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
use super::rtsp_channel::RtcpChannel;
use super::rtsp_channel::RtpChannel;
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

pub trait Track {
    fn create_packer(&mut self, writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>);
    fn create_unpacker(&mut self);
}
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum TrackType {
    #[default]
    Audio,
    Video,
    Application,
}

// A track can be a audio/video track, the A/V media data is transmitted
// over RTP, and the control data is transmitted over RTCP.
// The rtp/rtcp can be over TCP or UDP :
// 1. Over the TCP: It shares one TCP channel with the RTSP signaling data, and
// the entire session uses only one TCP connection（RTSP signaling data/audio
// RTP/audio RTCP/video RTP/video RTCP）
// 2. Over the UDP: It will establish 4 UDP channles for A/V RTP/RTCP data.
// 2.1 A RTP channel for audio media data transmitting.
// 2.2 A RTCP channel for audio control data transmitting
// 2.3 A RTP channel for video media data transmitting.
// 2.4 A RTCP channel for video control data transmitting
pub struct RtspTrack {
    track_type: TrackType,

    pub transport: RtspTransport,
    pub uri: String,
    pub media_control: String,

    pub rtp_channel: Arc<Mutex<RtpChannel>>,
    rtcp_channel: Arc<Mutex<RtcpChannel>>,
}

impl RtspTrack {
    pub fn new(
        track_type: TrackType,
        codec_info: RtspCodecInfo,
        media_control: String,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        let rtp_channel = RtpChannel::new(codec_info);

        let rtsp_track = RtspTrack {
            track_type,
            media_control,
            transport: RtspTransport::default(),
            uri: String::default(),
            rtp_channel: Arc::new(Mutex::new(rtp_channel)),
            rtcp_channel: Arc::new(Mutex::new(RtcpChannel::default())),
        };

        rtsp_track
    }

    pub async fn rtp_receive_loop(&mut self, mut rtp_io: Box<dyn TNetIO + Send + Sync>) {
        let rtp_channel_out = self.rtp_channel.clone();
        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            let mut rtp_channel_in = rtp_channel_out.lock().await;
            loop {
                match rtp_io.read().await {
                    Ok(data) => {
                        reader.extend_from_slice(&data[..]);
                        rtp_channel_in.on_rtp(&mut reader);
                    }
                    Err(err) => {
                        log::error!("read error: {:?}", err);
                        break;
                    }
                }
            }
        });
    }
    //send and receive rtcp data in a UDP channel
    pub async fn rtcp_run_loop(&mut self, mut rtcp_io: Box<dyn TNetIO + Send + Sync>) {
        let rtcp_channel_out = self.rtcp_channel.clone();
        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            let mut rtcp_channel_in = rtcp_channel_out.lock().await;
            loop {
                match rtcp_io.read().await {
                    Ok(data) => {
                        reader.extend_from_slice(&data[..]);
                        rtcp_channel_in.on_rtcp(&mut reader);
                    }
                    Err(err) => {
                        log::error!("read error: {:?}", err);
                        break;
                    }
                }
            }
        });
    }

    pub fn set_transport(&mut self, transport: RtspTransport) {
        self.transport = transport;
    }

    pub async fn on_rtp(&mut self, reader: &mut BytesReader) {
        self.rtp_channel.lock().await.on_rtp(reader);
    }

    pub async fn on_rtcp(&mut self, reader: &mut BytesReader) {
        self.rtcp_channel.lock().await.on_rtcp(reader);
    }

    pub async fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        self.rtp_channel.lock().await.create_packer(io);
    }
}
