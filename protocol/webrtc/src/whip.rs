use super::errors::WebRTCError;
use super::errors::WebRTCErrorValue;
use bytes::BytesMut;
use std::sync::Arc;
use streamhub::define::{PacketData, PacketDataSender};
use tokio::net::UdpSocket;
use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS, MIME_TYPE_VP8};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::util::{Conn, Marshal};

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
        .add_transceiver_from_kind(RTPCodecType::Audio, None)
        .await?;
    peer_connection
        .add_transceiver_from_kind(RTPCodecType::Video, None)
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

                // Write
                // if let Err(err) = c.conn.send(&b[..n]).await {
                //     // For this particular example, third party applications usually timeout after a short
                //     // amount of time during which the user doesn't have enough time to provide the answer
                //     // to the browser.
                //     // That's why, for this particular example, the user first needs to provide the answer
                //     // to the browser then open the third party application. Therefore we must not kill
                //     // the forward on "connection refused" errors
                //     //if opError, ok := err.(*net.OpError); ok && opError.Err.Error() == "write: connection refused" {
                //     //    continue
                //     //}
                //     //panic(err)
                //     if err.to_string().contains("Connection refused") {
                //         continue;
                //     } else {
                //         println!("conn send err: {err}");
                //         break;
                //     }
                // }
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

    // let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected
    let pc_clone = peer_connection.clone();
    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        log::info!("Peer Connection State has changed: {s}");
        let pc_clone_2 = pc_clone.clone();
        Box::pin(async move {
            if s == RTCPeerConnectionState::Failed {
                // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                println!("Peer Connection has gone to failed exiting: Done forwarding");
                // let _ = done_tx.try_send(());
                pc_clone_2.close().await;
            }
        })
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
