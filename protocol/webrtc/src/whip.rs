use crate::opus2aac::Opus2AacTranscoder;

use super::errors::WebRTCError;
use super::errors::WebRTCErrorValue;
use bytes::BytesMut;
use std::sync::Arc;
use streamhub::define::VideoCodecType;
use streamhub::define::{FrameData, PacketData};
use tokio::sync::mpsc::UnboundedSender;
use webrtc::rtp::codecs::opus::OpusPacket;

use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp::codecs::h264::H264Packet;
use webrtc::sdp::util::Codec;

use super::rtp_queue::RtpQueue;
use webrtc::rtp::packetizer::Depacketizer;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::util::Marshal;
use xflv::mpeg4_aac::Mpeg4Aac;

pub type Result<T> = std::result::Result<T, WebRTCError>;

// mod nal_unit_type {
//     pub const SPS: u8 = 0x07; //0x67
//     pub const PPS: u8 = 0x08; //0x68
//     pub const IDR_FRAME: u8 = 0x05; //0x65
//     pub const NO_IDR_FRAME: u8 = 0x01; //0x41 B/P frame
// }

mod nal_payload_type {
    pub const H264: u8 = 96;
    pub const OPUS: u8 = 111;
}

pub(crate) fn parse_rtpmap(rtpmap: &str) -> Result<Codec> {
    // a=rtpmap:<payload type> <encoding name>/<clock rate>[/<encoding parameters>]
    let split: Vec<&str> = rtpmap.split_whitespace().collect();
    if split.len() != 2 {
        return Err(WebRTCError {
            value: WebRTCErrorValue::MissingWhitespace,
        });
    }

    let pt_split: Vec<&str> = split[0].split(':').collect();
    if pt_split.len() != 2 {
        return Err(WebRTCError {
            value: WebRTCErrorValue::MissingColon,
        });
    }
    let payload_type = pt_split[1].parse::<u8>()?;

    let split: Vec<&str> = split[1].split('/').collect();
    let name = split[0].to_string();
    let parts = split.len();
    let clock_rate = if parts > 1 {
        split[1].parse::<u32>()?
    } else {
        0
    };
    let encoding_parameters = if parts > 2 {
        split[2].to_string()
    } else {
        "".to_string()
    };

    Ok(Codec {
        payload_type,
        name,
        clock_rate,
        encoding_parameters,
        ..Default::default()
    })
}

pub async fn handle_whip(
    offer: RTCSessionDescription,
    frame_sender: Option<UnboundedSender<FrameData>>,
    packet_sender: Option<UnboundedSender<PacketData>>,
) -> Result<(RTCSessionDescription, Arc<RTCPeerConnection>)> {
    // Create a MediaEngine object to configure the supported codec
    let mut m = MediaEngine::default();

    m.register_default_codecs()?;

    // Create a InterceptorRegistry. This is the user configurable RTP/RTCP Pipeline.
    // This provides NACKs, RTCP Reports and other features. If you use `webrtc.NewPeerConnection`
    // this is enabled by default. If you are manually managing You MUST create a InterceptorRegistry
    // for each PeerConnection.
    let mut registry = Registry::new();

    // Use the default set of Interceptors
    registry = register_default_interceptors(registry, &mut m)?;

    // Create the API object with the MediaEngine
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    // Prepare the configuration
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Create a new RTCPeerConnection
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    // Allow us to receive 1 audio track, and 1 video track
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Audio,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                send_encodings: Vec::new(),
            }),
        )
        .await?;
    peer_connection
        .add_transceiver_from_kind(
            RTPCodecType::Video,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                send_encodings: Vec::new(),
            }),
        )
        .await?;

    let offer_in = offer.clone();
    // Set a handler for when a new remote track starts, this handler will forward data to
    // our UDP listeners.
    // In your application this is where you would handle/process audio/video
    let pc = Arc::downgrade(&peer_connection);
    peer_connection.on_track(Box::new(move |track, _, _| {
        // Send a PLI on an interval so that the publisher is pushing a keyframe every rtcpPLIInterval
        let media_ssrc = track.ssrc();
        let pc2 = pc.clone();
        tokio::spawn(async move {
            let mut result = Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(3));
                tokio::pin!(timeout);

                tokio::select! {
                    _ = timeout.as_mut() =>{
                        if let Some(pc) = pc2.upgrade(){
                            result = pc.write_rtcp(&[Box::new(PictureLossIndication{
                                sender_ssrc: 0,
                                media_ssrc,
                            })]).await.map_err(Into::into);
                        }else{
                            break;
                        }
                    }
                };
            }
        });
        let packet_sender_clone = packet_sender.clone().unwrap();
        let frame_sender_clone = frame_sender.clone().unwrap();
        let offer_clone = offer_in.clone();
        tokio::spawn(async move {
            let mut b = vec![0u8; 3000];
            let mut h264_packet = H264Packet::default();
            let mut opus_packet = OpusPacket;

            let mut video_codec = Codec::default();
            let mut audio_codec = Codec::default();
            let mut vcodec: VideoCodecType = VideoCodecType::H264;
            let mut opus2aac_transcoder = Opus2AacTranscoder::new(
                48000,
                opus::Channels::Stereo,
                48000,
                fdk_aac::enc::ChannelMode::Stereo,
            )
            .unwrap();

            //111 OPUS/48000/2
            //96 H264/90000
            if let Ok(session_description) = offer_clone.unmarshal() {
                for m in session_description.media_descriptions {
                    for a in &m.attributes {
                        let attr = a.to_string();
                        if attr.starts_with("rtpmap:") {
                            if let Ok(codec) = parse_rtpmap(&attr) {
                                log::info!("codec: {}", codec);
                                match codec.name.as_str() {
                                    "H264" => {
                                        video_codec = codec;
                                    }
                                    "H265" => {
                                        video_codec = codec;
                                        vcodec = VideoCodecType::H265;
                                    }
                                    "OPUS" => {
                                        audio_codec = codec;
                                        let channels =
                                            match audio_codec.encoding_parameters.as_str() {
                                                "1" => opus::Channels::Mono,
                                                "2" => opus::Channels::Stereo,
                                                _ => opus::Channels::Stereo,
                                            };

                                        opus2aac_transcoder = Opus2AacTranscoder::new(
                                            audio_codec.clock_rate,
                                            channels,
                                            audio_codec.clock_rate,
                                            fdk_aac::enc::ChannelMode::Stereo,
                                        )
                                        .unwrap();
                                    }
                                    _ => {
                                        log::warn!("not supported codec: {}", codec);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let media_info = FrameData::MediaInfo {
                media_info: streamhub::define::MediaInfo {
                    audio_clock_rate: audio_codec.clock_rate,
                    video_clock_rate: video_codec.clock_rate,
                    vcodec,
                },
            };

            if let Err(err) = frame_sender_clone.send(media_info) {
                log::error!("send media info error: {}", err);
            } else {
                log::info!("send media info suceess: {:?} {}", audio_codec, video_codec);
            }

            let _sps_sent: bool = false;
            let _pps_sent: bool = false;
            let mut aac_asc_sent: bool = false;

            let mut rtp_queue = RtpQueue::new(100);

            while let Ok((rtp_packet, _)) = track.read(&mut b).await {
                let n = rtp_packet.marshal_to(&mut b)?;

                match rtp_packet.header.payload_type {
                    //video h264
                    nal_payload_type::H264 => {
                        let video_packet = PacketData::Video {
                            timestamp: rtp_packet.header.timestamp,
                            data: BytesMut::from(&b[..n]),
                        };
                        if let Err(err) = packet_sender_clone.send(video_packet) {
                            log::error!("send video packet error: {}", err);
                        }

                        rtp_queue.write_queue(rtp_packet);

                        while let Some(rtp_packet_ordered) = rtp_queue.read_queue() {
                            match h264_packet.depacketize(&rtp_packet_ordered.payload) {
                                Ok(rv) => {
                                    if !rv.is_empty() {
                                        let byte_array = rv.to_vec();
                                        let nal_type = byte_array[4] & 0x1F;

                                        if nal_type != 0x0C {
                                            let video_frame = FrameData::Video {
                                                timestamp: rtp_packet_ordered.header.timestamp,
                                                data: BytesMut::from(&byte_array[..]),
                                            };

                                            if let Err(err) = frame_sender_clone.send(video_frame) {
                                                log::error!("send video frame error: {}", err);
                                            } else {
                                                // log::info!("send video frame suceess: {}", nal_type);
                                            }
                                        }
                                    }
                                }
                                Err(_err) => {
                                    // log::error!("The h264 packet payload err:{}", err);
                                    // let hex_string = hex::encode(b.to_vec());
                                    // log::error!(
                                    //     "The h264 packet payload err string :{}",
                                    //     hex_string
                                    // );
                                }
                            }
                        }
                    }
                    //aac 111(opus)
                    nal_payload_type::OPUS => {
                        let audio_packet = PacketData::Audio {
                            timestamp: rtp_packet.header.timestamp,
                            data: BytesMut::from(&b[..n]),
                        };
                        if let Err(err) = packet_sender_clone.send(audio_packet) {
                            log::error!("send audio packet error: {}", err);
                        }

                        if !aac_asc_sent {
                            if let Ok(aac) = Mpeg4Aac::new(2, 48000, 2) {
                                if let Ok(asc) = aac.gen_audio_specific_config() {
                                    let audio_frame = FrameData::Audio {
                                        timestamp: 0,
                                        data: asc,
                                    };
                                    if let Err(err) = frame_sender_clone.send(audio_frame) {
                                        log::error!("send audio frame error: {}", err);
                                    }
                                }
                            }
                            aac_asc_sent = true;
                        }

                        match opus_packet.depacketize(&rtp_packet.payload) {
                            Ok(rv) => {
                                if !rv.is_empty() {
                                    // log::info!("audio timestamp: {}", rtp_packet.header.timestamp);
                                    let byte_array = rv.to_vec();
                                    match opus2aac_transcoder.transcode(&byte_array) {
                                        Ok(data) => {
                                            for data_val in data {
                                                let audio_frame = FrameData::Audio {
                                                    timestamp: rtp_packet.header.timestamp,
                                                    data: BytesMut::from(&data_val[..]),
                                                };

                                                if let Err(err) =
                                                    frame_sender_clone.send(audio_frame)
                                                {
                                                    log::error!("send audio frame error: {}", err);
                                                } else {
                                                    // log::info!("send aidop frame suceess");
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            log::error!("opus2aac transcode error: {:?}", err);
                                        }
                                    }
                                }
                            }
                            Err(_err) => {}
                        }
                    }
                    _ => {}
                }
            }

            Result::<()>::Ok(())
        });

        Box::pin(async {})
    }));

    // Set the handler for ICE connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection.on_ice_connection_state_change(Box::new(
        move |connection_state: RTCIceConnectionState| {
            log::info!("Connection State has changed {connection_state}");
            if connection_state == RTCIceConnectionState::Connected {
                log::info!("Ctrl+C the remote client to stop the demo");
            }
            Box::pin(async {})
        },
    ));

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected

    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        log::info!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            println!("Peer Connection has gone to failed exiting: Done forwarding");
        }

        Box::pin(async {})
    }));

    // Set the remote SessionDescription
    peer_connection.set_remote_description(offer).await?;

    // Create an answer
    let answer = peer_connection.create_answer(None).await?;

    // Create channel that is blocked until ICE Gathering is complete
    let mut gather_complete = peer_connection.gathering_complete_promise().await;

    // Sets the LocalDescription, and starts our UDP listeners
    peer_connection.set_local_description(answer).await?;

    // Block until ICE Gathering is complete, disabling trickle ICE
    // we do this because we only can exchange one signaling message
    // in a production application you should exchange ICE Candidates via OnICECandidate
    let _ = gather_complete.recv().await;

    // Output the answer in base64 so we can paste it in browser
    if let Some(local_desc) = peer_connection.local_description().await {
        Ok((local_desc, peer_connection))
    } else {
        Err(WebRTCError {
            value: WebRTCErrorValue::CanNotGetLocalDescription,
        })
    }
}
