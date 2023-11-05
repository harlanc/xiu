pub mod errors;

use streamhub::{
    define::{
        FrameData, Information, InformationSender, NotifyInfo, PublishType, PublisherInfo,
        StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::{ChannelError, ChannelErrorValue},
    statistics::StreamStatistics,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use self::errors::SessionError;

pub struct GB28181ServerSession {}

impl GB28181ServerSession {
    pub fn new(stream: TcpStream, event_producer: StreamHubEventSender) -> Self {
        // let remote_addr = if let Ok(addr) = stream.peer_addr() {
        //     log::info!("server session: {}", addr.to_string());
        //     Some(addr)
        // } else {
        //     None
        // };

        Self {}
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        Ok(())
    }
}
