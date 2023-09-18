use super::errors::WebRTCError;
use super::errors::WebRTCErrorValue;
use bytes::BytesMut;
use std::sync::Arc;
use streamhub::define::PacketDataReceiver;
use streamhub::define::{PacketData, PacketDataSender};
use tokio::net::UdpSocket;
use tokio::time::Duration;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_H264, MIME_TYPE_OPUS};
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
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::util::{Conn, Marshal};

pub type Result<T> = std::result::Result<T, WebRTCError>;

pub async fn handle_whep(
    offer: RTCSessionDescription,
    mut receiver: PacketDataReceiver,
) -> Result<(RTCSessionDescription, Arc<RTCPeerConnection>)> {
    // Everything below is the WebRTC-rs API! Thanks for using it ❤️.

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

    // Create Track that we send video back to browser on
    let video_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_H264.to_owned(),
            ..Default::default()
        },
        "video".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    // Create Track that we send video back to browser on
    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            ..Default::default()
        },
        "audio".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    // Add this newly created track to the PeerConnection
    let rtp_sender = peer_connection
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    let rtp_sender_audio = peer_connection
        .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    // Read incoming RTCP packets
    // Before these packets are returned they are processed by interceptors. For things
    // like NACK this needs to be called.
    tokio::spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        Result::<()>::Ok(())
    });

    // Set the handler for ICE connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection.on_ice_connection_state_change(Box::new(
        move |connection_state: RTCIceConnectionState| {
            println!("Connection State has changed {connection_state}");
            if connection_state == RTCIceConnectionState::Failed {
                // let _ = done_tx1.try_send(());
            }
            Box::pin(async {})
        },
    ));

    // Set the handler for Peer connection state
    // This will notify you when the peer has connected/disconnected
    peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            println!("Peer Connection has gone to failed exiting: Done forwarding");
            // let _ = done_tx2.try_send(());
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
    // if let Some(local_desc) = peer_connection.local_description().await {
    //     let json_str = serde_json::to_string(&local_desc)?;
    //     let b64 = signal::encode(&json_str);
    //     println!("{b64}");
    // } else {
    //     println!("generate local_description failed!");
    // }

    // // Open a UDP Listener for RTP Packets on port 5004
    // let listener = UdpSocket::bind("127.0.0.1:5004").await?;

    // let done_tx3 = done_tx.clone();
    // Read RTP packets forever and send them to the WebRTC Client
    tokio::spawn(async move {
        // let mut inbound_rtp_packet = vec![0u8; 1600]; // UDP MTU
        // while let Ok((n, _)) = listener.recv_from(&mut inbound_rtp_packet).await {
        //     if let Err(err) = video_track.write(&inbound_rtp_packet[..n]).await {
        //         if Error::ErrClosedPipe == err {
        //             // The peerConnection has been closed.
        //         } else {
        //             println!("video_track write err: {err}");
        //         }
        //         let _ = done_tx3.try_send(());
        //         return;
        //     }
        // }

        loop {
            if let Some(data) = receiver.recv().await {
                match data {
                    PacketData::Video { timestamp, data } => {
                        video_track.write(&data[..]);
                    }

                    PacketData::Audio { timestamp, data } => {
                        audio_track.write(&data[..]);
                    }
                }
            } else {
            }
        }
    });

    // Output the answer in base64 so we can paste it in browser
    if let Some(local_desc) = peer_connection.local_description().await {
        Ok((local_desc, peer_connection))
    } else {
        Err(WebRTCError {
            value: WebRTCErrorValue::CanNotGetLocalDescription,
        })
    }

    // println!("Press ctrl-c to stop");
    // tokio::select! {
    //     _ = done_rx.recv() => {
    //         println!("received done signal!");
    //     }
    //     _ = tokio::signal::ctrl_c() => {
    //         println!();
    //     }
    // };

    // peer_connection.close().await?;

    // Ok(())
}
