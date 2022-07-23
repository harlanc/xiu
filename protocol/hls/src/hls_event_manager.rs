use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
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

pub struct HlsEventManager {
	stream_to_producer: Arc<Mutex<HashMap<String, HlsEventConsumer>>>,
}

impl HlsEventManager {
	pub fn new() -> HlsEventManager {
		return HlsEventManager {
			stream_to_producer: Arc::new(Mutex::new(HashMap::new())),
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
						stp.lock().unwrap().insert(stream_name.to_owned(), rx);

						channel.send(tx).expect("Failed to send");
					}
				}
			}
		});

		tx
	}
}
