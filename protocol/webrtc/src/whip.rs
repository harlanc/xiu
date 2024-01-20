use super::errors::WebRTCError;
use super::errors::WebRTCErrorValue;
use bytes::BytesMut;
use std::sync::Arc;
use streamhub::define::{PacketData, PacketDataSender};

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
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::util::Marshal;

pub type Result<T> = std::result::Result<T, WebRTCError>;

pub async fn handle_whip(
    offer: RTCSessionDescription,
    sender: PacketDataSender,
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
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let mut b = vec![0u8; 3000];

            while let Ok((rtp_packet, _)) = track.read(&mut b).await {
                // Update the PayloadType
                //rtp_packet.header.payload_type = c.payload_type;

                // Marshal into original buffer with updated PayloadType

                let n = rtp_packet.marshal_to(&mut b)?;

                match rtp_packet.header.payload_type {
                    //video h264
                    96 => {
                        let video_packet = PacketData::Video {
                            timestamp: rtp_packet.header.timestamp,
                            data: BytesMut::from(&b[..n]),
                        };
                        if let Err(err) = sender_clone.send(video_packet) {
                            log::error!("send video packet error: {}", err);
                        }
                    }
                    //aac
                    97 | 111 => {
                        let audio_packet = PacketData::Audio {
                            timestamp: rtp_packet.header.timestamp,
                            data: BytesMut::from(&b[..n]),
                        };
                        if let Err(err) = sender_clone.send(audio_packet) {
                            log::error!("send audio packet error: {}", err);
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

#[cfg(test)]
mod tests {
    // use ogg::reading::PacketReader;
    // use std::fs::File;
    // use std::io::BufReader;

    // use opus::Decoder;

    use bytes::{BufMut, BytesMut};
    use bytesio::bytes_writer::BytesWriter;
    use fdk_aac::dec::Decoder as AacDecoder;
    use fdk_aac::enc::Encoder as AacEncoder;
    use ogg::reading::PacketReader;
    use opus::Decoder as OpusDecoder;
    use opus::Encoder as OpusEncoder;
    use std::collections::VecDeque;
    use std::fs::File;
    use std::io::{self, Read};
    use std::io::{BufReader, BufWriter, Write};
    use webrtc::media::audio;
    use xflv::demuxer::{FlvAudioTagDemuxer, FlvDemuxer};
    use xflv::flv_tag_header::AudioTagHeader;
    use xflv::muxer::FlvMuxer;
    use xflv::Marshal;

    #[test]
    fn test_flv_2_opus() {
        let mut file = File::open("/Users/hailiang8/xgplayer-demo-360p.flv").unwrap();

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("Failed to read file");

        let mut bytes = BytesMut::from(&buffer[..]);

        let mut flv_demuxer = FlvDemuxer::new(bytes);

        flv_demuxer.read_flv_header();

        let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
        let mut aac_encoder = AacEncoder::new(fdk_aac::enc::EncoderParams {
            bit_rate: fdk_aac::enc::BitRate::VbrMedium,
            sample_rate: 48000,
            transport: fdk_aac::enc::Transport::Raw,
            channels: fdk_aac::enc::ChannelMode::Stereo,
        })
        .unwrap();

        let mut i = 0;

        let mut audio_demuxer = FlvAudioTagDemuxer::new();

        let mut audio_pcm_data_from_decoded_aac: Vec<i16> = Vec::new();

        let mut audio_pcm_data_from_decoded_opus: Vec<i16> = Vec::new();

        let mut opus_encoder =
            OpusEncoder::new(48000, opus::Channels::Stereo, opus::Application::Voip).unwrap();

        let mut opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo).unwrap();
        let mut i = 0;

        let mut file = File::create("/Users/hailiang8/xgplayer-demo-360p-aac.flv").unwrap();

        let mut flv_muxer = FlvMuxer::default();
        flv_muxer.write_flv_header();
        flv_muxer.write_previous_tag_size(0);

        //  let mut buffer = VecDeque::new();

        let mut time: u32 = 0;

        loop {
            match flv_demuxer.read_flv_tag() {
                Ok(data) => {
                    if let Some(flvtag) = data {
                        match flvtag {
                            xflv::define::FlvData::Audio { timestamp, data } => {
                                println!("audio: time:{}", timestamp);
                                let len = data.len() as u32;

                                match audio_demuxer.demux(timestamp, data.clone()) {
                                    Ok(d) => {
                                        // i = i + 1;
                                        // if i > 50 {
                                        //     return;
                                        // }

                                        if !d.has_data {
                                            flv_muxer.write_flv_tag_header(8, len, timestamp);
                                            flv_muxer.write_flv_tag_body(data);
                                            flv_muxer.write_previous_tag_size(len + 11);

                                            let data = flv_muxer.writer.extract_current_bytes();
                                            file.write_all(&data[..]);

                                            continue;
                                        }

                                        println!("aac demux len: {}", d.data.len());
                                        aac_decoder.fill(&d.data);
                                        let mut decode_frame = vec![0_i16; 1024 * 3];

                                        match aac_decoder.decode_frame(&mut decode_frame[..]) {
                                            Ok(()) => {
                                                let len = aac_decoder.decoded_frame_size();

                                                // for i in 0..len {
                                                //     buffer.push_back(decode_frame[i]);
                                                // }

                                                println!("aac decoder ok : {}", len);
                                                audio_pcm_data_from_decoded_aac
                                                    .extend_from_slice(&decode_frame[..len]);

                                                while audio_pcm_data_from_decoded_aac.len()
                                                    >= 960 * 2
                                                {
                                                    let pcm = audio_pcm_data_from_decoded_aac
                                                        .split_off(960 * 2);

                                                    let mut encoded_opus = vec![0; 1500];

                                                    println!(
                                                        "input len: {}",
                                                        audio_pcm_data_from_decoded_aac.len()
                                                    );

                                                    match opus_encoder.encode(
                                                        &audio_pcm_data_from_decoded_aac,
                                                        &mut encoded_opus,
                                                    ) {
                                                        Ok(l) => {
                                                            let samples = opus_decoder
                                                                .get_nb_samples(&encoded_opus)
                                                                .unwrap();
                                                            println!(
                                                                "opus encode ok : {} {} samples:{}",
                                                                l,
                                                                audio_pcm_data_from_decoded_aac
                                                                    .len(),
                                                                samples
                                                            );
                                                            audio_pcm_data_from_decoded_aac = pcm;

                                                            let mut output = vec![0; 5670]; //960 * 2];

                                                            match opus_decoder.decode(
                                                                &encoded_opus[..l],
                                                                &mut output,
                                                                false,
                                                            ) {
                                                                Ok(size) => {
                                                                    println!(
                                                                        "opus decode ok : {}",
                                                                        size
                                                                    );

                                                                    audio_pcm_data_from_decoded_opus.extend_from_slice(&output[..size*2]);

                                                                    while audio_pcm_data_from_decoded_opus.len() >= 2048
                                                {
                                                    let pcm = audio_pcm_data_from_decoded_opus
                                                        .split_off(2048);

                                                    let mut encoded_aac: Vec<u8> = vec![0; 1500];
                                                    match aac_encoder.encode(
                                                        &audio_pcm_data_from_decoded_opus,
                                                        &mut encoded_aac[..],
                                                    ) {
                                                        Ok(info) => {
                                                            println!("aac encode ok : {:?}", info);
                                                            audio_pcm_data_from_decoded_opus = pcm; //audio_pcm_data_from_decoded_opus[info.input_consumed..].to_vec();

                                                            if info.output_size > 0 {
                                                                let audio_tag_header =
                                                                    AudioTagHeader {
                                                                        sound_format: 10,
                                                                        sound_rate: 3,
                                                                        sound_size: 1,
                                                                        sound_type: 1,
                                                                        aac_packet_type: 1,
                                                                    };

                                                                let tag_header_data =
                                                                    audio_tag_header
                                                                        .marshal()
                                                                        .unwrap();

                                                                let mut writer = BytesWriter::new();
                                                                writer.write(&tag_header_data);

                                                                let audio_data = &encoded_aac
                                                                    [..info.output_size];
                                                                writer.write(audio_data);

                                                                let body =
                                                                    writer.extract_current_bytes();

                                                                let len = body.len() as u32;

                                                                flv_muxer.write_flv_tag_header(
                                                                    8, len, time,
                                                                );
                                                                flv_muxer.write_flv_tag_body(body);
                                                                flv_muxer.write_previous_tag_size(
                                                                    len + 11,
                                                                );

                                                                time += 21;

                                                                let data = flv_muxer
                                                                    .writer
                                                                    .extract_current_bytes();
                                                                file.write_all(&data[..]);
                                                            }
                                                            if info.input_consumed > 0
                                                                && info.output_size > 0
                                                            {
                                                            } else {
                                                                break;
                                                            }
                                                        }
                                                        Err(err) => {
                                                            println!("aac encode err : {}", err);
                                                        }
                                                    }
                                                }
                                                                }
                                                                Err(err) => {
                                                                    println!(
                                                                        "opus decode err : {}",
                                                                        err
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Err(err) => {
                                                            println!("opus encode err : {}", err);
                                                        }
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                println!("decoder error: {}", err);
                                                // return;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        println!("demux error: {}", err);
                                    }
                                }
                            }
                            xflv::define::FlvData::Video { timestamp, data } => {
                                println!("video");
                            }
                            xflv::define::FlvData::MetaData { timestamp, data } => {
                                println!("metadata");
                            }
                        }
                    } else {
                        println!("read none");
                    }
                }
                Err(err) => {
                    println!("read error: {}", err);
                    // file.w
                    return;
                }
            }
        }
    }

    #[test]
    fn test_demux_decode_encode_mux_aac() {
        let mut file = File::open("/Users/hailiang8/xgplayer-demo-360p.flv").unwrap();

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("Failed to read file");

        let mut bytes = BytesMut::from(&buffer[..]);

        let mut flv_demuxer = FlvDemuxer::new(bytes);

        flv_demuxer.read_flv_header();

        let mut aac_decoder = AacDecoder::new(fdk_aac::dec::Transport::Adts);
        let mut aac_encoder = AacEncoder::new(fdk_aac::enc::EncoderParams {
            bit_rate: fdk_aac::enc::BitRate::VbrMedium,
            sample_rate: 48000,
            transport: fdk_aac::enc::Transport::Raw,
            channels: fdk_aac::enc::ChannelMode::Stereo,
        })
        .unwrap();

        let mut i = 0;

        let mut audio_demuxer = FlvAudioTagDemuxer::new();

        let mut audio_pcm_data_from_decoded_aac: Vec<i16> = Vec::new();

        let mut audio_pcm_data_from_decoded_opus: Vec<i16> = Vec::new();

        let mut opus_encoder =
            OpusEncoder::new(48000, opus::Channels::Stereo, opus::Application::Voip).unwrap();

        let mut opus_decoder = OpusDecoder::new(48000, opus::Channels::Stereo).unwrap();
        let mut i = 0;

        let mut file = File::create("/Users/hailiang8/xgplayer-demo-360p-aac.flv").unwrap();

        let mut flv_muxer = FlvMuxer::default();
        flv_muxer.write_flv_header();
        flv_muxer.write_previous_tag_size(0);

        //  let mut buffer = VecDeque::new();

        let mut time: u32 = 0;

        loop {
            match flv_demuxer.read_flv_tag() {
                Ok(data) => {
                    if let Some(flvtag) = data {
                        match flvtag {
                            xflv::define::FlvData::Audio { timestamp, data } => {
                                println!("audio: time:{}", timestamp);
                                let len = data.len() as u32;

                                match audio_demuxer.demux(timestamp, data.clone()) {
                                    Ok(d) => {
                                        if !d.has_data {
                                            flv_muxer.write_flv_tag_header(8, len, timestamp);
                                            flv_muxer.write_flv_tag_body(data);
                                            flv_muxer.write_previous_tag_size(len + 11);

                                            let data = flv_muxer.writer.extract_current_bytes();
                                            file.write_all(&data[..]);

                                            continue;
                                        }

                                        println!("aac demux len: {}", d.data.len());
                                        aac_decoder.fill(&d.data);
                                        let mut decode_frame = vec![0_i16; 1024 * 3];

                                        match aac_decoder.decode_frame(&mut decode_frame[..]) {
                                            Ok(()) => {
                                                let len = aac_decoder.decoded_frame_size();

                                                audio_pcm_data_from_decoded_opus
                                                    .extend_from_slice(&decode_frame[..len]);
                                                //                     // audio_pcm_data_from_decoded_opus.extend_from_slice(&output[..size]);

                                                while audio_pcm_data_from_decoded_opus.len() >= 2048
                                                {
                                                    let pcm = audio_pcm_data_from_decoded_opus
                                                        .split_off(2048);

                                                    let mut encoded_aac: Vec<u8> = vec![0; 1500];
                                                    match aac_encoder.encode(
                                                        &audio_pcm_data_from_decoded_opus,
                                                        &mut encoded_aac[..],
                                                    ) {
                                                        Ok(info) => {
                                                            println!("aac encode ok : {:?}", info);
                                                            audio_pcm_data_from_decoded_opus = pcm; //audio_pcm_data_from_decoded_opus[info.input_consumed..].to_vec();

                                                            if info.output_size > 0 {
                                                                let audio_tag_header =
                                                                    AudioTagHeader {
                                                                        sound_format: 10,
                                                                        sound_rate: 3,
                                                                        sound_size: 1,
                                                                        sound_type: 1,
                                                                        aac_packet_type: 1,
                                                                    };

                                                                let tag_header_data =
                                                                    audio_tag_header
                                                                        .marshal()
                                                                        .unwrap();

                                                                let mut writer = BytesWriter::new();
                                                                writer.write(&tag_header_data);

                                                                let audio_data = &encoded_aac
                                                                    [..info.output_size];
                                                                writer.write(audio_data);

                                                                let body =
                                                                    writer.extract_current_bytes();

                                                                let len = body.len() as u32;

                                                                flv_muxer.write_flv_tag_header(
                                                                    8, len, time,
                                                                );
                                                                flv_muxer.write_flv_tag_body(body);
                                                                flv_muxer.write_previous_tag_size(
                                                                    len + 11,
                                                                );

                                                                time += 21;

                                                                let data = flv_muxer
                                                                    .writer
                                                                    .extract_current_bytes();
                                                                file.write_all(&data[..]);
                                                            }
                                                            if info.input_consumed > 0
                                                                && info.output_size > 0
                                                            {
                                                            } else {
                                                                break;
                                                            }
                                                        }
                                                        Err(err) => {
                                                            println!("aac encode err : {}", err);
                                                        }
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                println!("decoder error: {}", err);
                                                // return;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        println!("demux error: {}", err);
                                    }
                                }
                            }
                            xflv::define::FlvData::Video { timestamp, data } => {
                                println!("video");
                            }
                            xflv::define::FlvData::MetaData { timestamp, data } => {
                                println!("metadata");
                            }
                        }
                    } else {
                        println!("read none");
                    }
                }
                Err(err) => {
                    println!("read error: {}", err);
                    // file.w
                    return;
                }
            }
        }
    }
    #[test]
    fn test_vec() {
        let mut decode_frame = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];

        let mut buffer = VecDeque::new();

        for i in 0..decode_frame.len() {
            buffer.push_back(decode_frame[i]);
        }

        let mut pcm = vec![0; 5 * 2];
        for i in 0..5 {
            pcm[i * 2] = buffer.pop_front().unwrap();
            pcm[i * 2 + 1] = buffer.pop_front().unwrap();
        }

        println!("{:?}", pcm);

        let mut audio_pcm_data: Vec<u8> = Vec::new();

        audio_pcm_data.extend_from_slice(&decode_frame[..]);

        println!("{:?}", audio_pcm_data);

        let pcm2 = audio_pcm_data.split_off(10);

        println!("{:?}", pcm2);
        println!("{:?}", audio_pcm_data);
    }
    #[test]
    fn test_rtmp_url_parser2() {
        // 打开待解码的文件
        let mut file = File::open("/Users/hailiang8/output.ogg").unwrap();

        // let mut pck_rdr = PacketReader::new(&mut file);

        // let mut ctr = 0;
        // loop {
        //     let r = pck_rdr.read_packet();
        //     match r {
        //         Ok(Some(p)) => {
        //             println!("some");
        //             //  dump_pck_info(&p, ctr);
        //             // Why do we not check p.last_packet here, and break the loop if false?
        //             // Well, first, this is only an example.
        //             // Second, the codecs may end streams in the middle of the file,
        //             // while still continuing other streams.
        //             // Therefore, don't do a probably too-early break.
        //             // Applications which know the codec may know after which
        //             // ended stream to stop decoding the file and thus not
        //             // encounter an error.
        //         }
        //         // End of stream
        //         Ok(None) => {
        //             println!("none");
        //             break;
        //         }
        //         Err(e) => {
        //             println!("Encountered Error: {:?}", e);
        //             break;
        //         }
        //     }
        //     ctr += 1;
        // }

        // 创建Ogg PacketReader
        let mut packet_reader = PacketReader::new(&mut file);

        // 创建Opus解码器
        let mut decoder = OpusDecoder::new(48000, opus::Channels::Stereo).unwrap();

        // 创建输出文件
        let mut output_file = File::create("decoded.pcm").unwrap();
        println!("test 1");
        // 解码循环
        // let mut state = PacketReaderState::new();
        loop {
            match packet_reader.read_packet() {
                Ok(Some(packet)) => {
                    // 检查是否为Opus数据包
                    //if packet.stream_serial() == 0
                    {
                        // 解码Opus数据包
                        let mut output: [i16; 4096] = [0; 4096];
                        match decoder.decode(&packet.data, &mut output, false) {
                            Ok(len) => {
                                println!("decode len: {}", len);
                            }
                            Err(err) => {
                                println!("decode err: {}", err);
                                return;
                            }
                        }

                        // 将解码后的样本写入输出文件
                        // output_file.write_all(samples)?;

                        // // 检查是否有更多的解码数据
                        // if decoder.is_empty() {
                        //     break;
                        // }
                    }
                }
                Ok(None) => {
                    println!("test 2");
                    break;
                } // 文件结束
                Err(error) => {
                    println!("decode err: {}", error);
                    return;
                }
            }
        }
    }
}
