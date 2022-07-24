use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::m3u8::M3u8PlaylistResponse;

pub enum DispatchEvent {
    CreateChannel {
        stream_name: String,
        channel: oneshot::Sender<(HlsEventProducer, M3u8Consumer)>,
    },
}

pub type DispatchEventProducer = mpsc::Sender<DispatchEvent>;
pub type DispatchEventConsumer = mpsc::Receiver<DispatchEvent>;

pub enum M3u8Event {
    RequestPlaylist {
        channel: oneshot::Sender<M3u8PlaylistResponse>,
    },
}

pub type M3u8Producer = mpsc::Sender<M3u8Event>;
pub type M3u8Consumer = mpsc::Receiver<M3u8Event>;

#[derive(Debug, Clone)]
pub enum HlsEvent {
    Init {},
    HlsSequenceIncr { sequence: u64 },
}

pub type HlsEventProducer = broadcast::Sender<HlsEvent>;
pub type HlsEventConsumer = broadcast::Receiver<HlsEvent>;

pub type StpMap = Arc<RwLock<HashMap<String, (HlsEventProducer, HlsEventConsumer, M3u8Producer)>>>;

pub struct HlsEventManager {
    pub stream_to_producer: StpMap,
}

impl HlsEventManager {
    pub fn new() -> HlsEventManager {
        return HlsEventManager {
            stream_to_producer: Arc::new(RwLock::new(HashMap::new())),
        };
    }

    pub fn setup_dispatch_channel(&self) -> DispatchEventProducer {
        let (tx, mut rx) = mpsc::channel(1);

        let stp = self.stream_to_producer.clone();

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                use DispatchEvent::*;
                match cmd {
                    CreateChannel {
                        stream_name,
                        channel,
                    } => {
                        let (tx, rx) = broadcast::channel(2);
                        let tx2 = tx.clone();

                        let (m3u8_tx, m3u8_rx) = mpsc::channel(1);

                        stp.write()
                            .unwrap()
                            .insert(stream_name.to_owned(), (tx, rx, m3u8_tx));

                        channel.send((tx2, m3u8_rx)).expect("Failed to send");
                    }
                }
            }
        });

        tx
    }
}
