use crate::notify::Notifier;
use reqwest::Client;
use async_trait::async_trait;
use crate::define::{PublisherInfo, StreamHubEventMessage, StreamHubEventSender, StreamHubEvent};

macro_rules! serialize_event {
    ($message:expr) => {{
        let event_serialize_str = match serde_json::to_string(&$message) {
            Ok(data) => {
                log::info!("event data: {}", data);
                data
            }
            Err(_) => String::from("empty body"),
        };
        event_serialize_str
    }};
}


pub struct HttpNotifier {
    request_client: Client,
    on_publish_url: Option<String>,
    on_unpublish_url: Option<String>,
    on_play_url: Option<String>,
    on_stop_url: Option<String>,
    on_hls_url: Option<String>,
    event_producer: StreamHubEventSender,
}

impl HttpNotifier {
    pub fn new(
        on_publish_url: Option<String>,
        on_unpublish_url: Option<String>,
        on_play_url: Option<String>,
        on_stop_url: Option<String>,
        on_hls_url: Option<String>,
        event_producer: StreamHubEventSender,

    ) -> Self {
        Self {
            request_client: reqwest::Client::new(),
            on_publish_url,
            on_unpublish_url,
            on_play_url,
            on_stop_url,
            on_hls_url,
            event_producer,
        }
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn on_publish_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_publish_url) = &self.on_publish_url {
            match self
                .request_client
                .post(on_publish_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_publish error: {}", err);
                }
                Ok(response) => {
                    if response.status() != 200 {
                        self.kick_off_client(event).await;
                    }
                    log::info!("on_publish success: {:?}", response);
                }
            }
        }
    }

    async fn on_unpublish_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_unpublish_url) = &self.on_unpublish_url {
            match self
                .request_client
                .post(on_unpublish_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_unpublish error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_unpublish success: {:?}", response);
                }
            }
        }
    }

    async fn on_play_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_play_url) = &self.on_play_url {
            match self
                .request_client
                .post(on_play_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_play error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_play success: {:?}", response);
                }
            }
        }
    }

    async fn on_stop_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_stop_url) = &self.on_stop_url {
            match self
                .request_client
                .post(on_stop_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_stop error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_stop success: {:?}", response);
                }
            }
        }
    }

    async fn on_hls_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_hls_url) = &self.on_hls_url {
            match self
                .request_client
                .post(on_hls_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_hls error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_hls success: {:?}", response);
                }
            }
        }
    }

    async fn kick_off_client(&self, event: &StreamHubEventMessage) {
        if let StreamHubEventMessage::Publish { identifier, info } = event {
            let hub_event = StreamHubEvent::ApiKickClient { id: info.id.clone() };
            if let Err(err) = self.event_producer.send(hub_event) {
                log::error!("send notify kick_off_client event error: {}", err);
            }
            log::info!("kick from hook: {:?}", identifier);
        }
    }
}
