pub mod errors;
pub mod rtsp2rtmp;

use streamhub::{
    define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
    stream::StreamIdentifier,
};

use self::errors::RtmpRemuxerError;

//Receive publish event from stream hub and
//remux from other protocols to rtmp
pub struct RtmpRemuxer {
    receiver: BroadcastEventReceiver,
    event_producer: StreamHubEventSender,
}

impl RtmpRemuxer {
    pub async fn run(&mut self) -> Result<(), RtmpRemuxerError> {
        loop {
            let val = self.receiver.recv().await?;
            match val {
                BroadcastEvent::Publish { identifier } => {
                    if let StreamIdentifier::Rtsp { stream_path } = identifier {}
                    // if let StreamIdentifier::Rtmp {
                    //     app_name,
                    //     stream_name,
                    // } = identifier
                    // {}
                }
                _ => {
                    log::trace!("other infos...");
                }
            }
        }
    }
}
