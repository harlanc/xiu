use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::{broadcast, mpsc, oneshot};

pub enum DispatchEvent {
    CreateChannel {
        stream_name: String,
        channel: oneshot::Sender<HlsEventProducer>,
    },
}

pub type DispatchEventProducer = mpsc::Sender<DispatchEvent>;
pub type DispatchEventConsumer = mpsc::Receiver<DispatchEvent>;

#[derive(Debug, Clone)]
pub enum HlsEvent {
    Init {},
    HlsSequenceIncr { sequence: u64 },
}

pub type HlsEventProducer = broadcast::Sender<HlsEvent>;
pub type HlsEventConsumer = broadcast::Receiver<HlsEvent>;

pub type StpMap = Arc<RwLock<HashMap<String, (HlsEventProducer, HlsEventConsumer)>>>;

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
                        stp.write()
                            .unwrap()
                            .insert(stream_name.to_owned(), (tx, rx));

                        channel.send(tx2).expect("Failed to send");
                    }
                }
            }
        });

        tx
    }
}
