use super::rtsp_channel::RtcpChannel;
use super::rtsp_channel::RtpChannel;
use super::rtsp_codec::RtspCodecInfo;
use super::rtsp_transport::RtspTransport;
use crate::rtp::errors::UnPackerError;
use crate::rtsp_channel::TRtpFunc;
use bytes::BytesMut;
use bytesio::bytes_reader::BytesReader;
use bytesio::bytesio::TNetIO;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    pub track_type: TrackType,

    pub transport: RtspTransport,
    pub uri: String,
    pub media_control: String,

    pub rtp_channel: Arc<Mutex<RtpChannel>>,
    pub rtcp_channel: Arc<Mutex<RtcpChannel>>,
}

impl RtspTrack {
    pub fn new(track_type: TrackType, codec_info: RtspCodecInfo, media_control: String) -> Self {
        let rtp_channel = RtpChannel::new(codec_info);

        RtspTrack {
            track_type,
            media_control,
            transport: RtspTransport::default(),
            uri: String::default(),
            rtp_channel: Arc::new(Mutex::new(rtp_channel)),
            rtcp_channel: Arc::new(Mutex::default()),
        }
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
                        if let Err(err) = rtp_channel_in.on_packet(&mut reader) {
                            log::error!("rtp_receive_loop on_packet error: {}", err);
                        }
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
    pub async fn rtcp_receive_loop(&mut self, rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        let rtcp_channel_out = self.rtcp_channel.clone();

        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            let mut rtcp_channel_in = rtcp_channel_out.lock().await;

            loop {
                let data = match rtcp_io.lock().await.read().await {
                    Ok(data) => data,
                    Err(err) => {
                        log::error!("read error: {:?}", err);
                        break;
                    }
                };
                reader.extend_from_slice(&data[..]);
                rtcp_channel_in.on_rtcp(&mut reader, rtcp_io.clone()).await;
            }
        });
    }

    pub async fn set_transport(&mut self, transport: RtspTransport) {
        if let Some(interleaveds) = transport.interleaved {
            self.rtcp_channel
                .lock()
                .await
                .set_channel_identifier(interleaveds[1]);
        } else {
            log::info!("it is a udp transport!!!");
        }

        self.transport = transport;
    }

    pub async fn on_rtp(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        self.rtp_channel.lock().await.on_packet(reader)
    }

    pub async fn on_rtcp(
        &mut self,
        reader: &mut BytesReader,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) {
        self.rtcp_channel.lock().await.on_rtcp(reader, io).await;
    }

    pub async fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        self.rtp_channel.lock().await.create_packer(io);
    }
}
