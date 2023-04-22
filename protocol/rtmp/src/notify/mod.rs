use reqwest::Client;

pub struct Notifier {
    request_client: Client,
    on_publish_url: Option<String>,
    on_unpublish_url: Option<String>,
    on_play_url: Option<String>,
    on_stop_url: Option<String>,
}

impl Notifier {
    pub fn new(
        on_publish_url: Option<String>,
        on_unpublish_url: Option<String>,
        on_play_url: Option<String>,
        on_stop_url: Option<String>,
    ) -> Self {
        Self {
            request_client: reqwest::Client::new(),
            on_publish_url,
            on_unpublish_url,
            on_play_url,
            on_stop_url,
        }
    }
    pub async fn on_publish_notify(&self, body: String) {
        if let Some(on_publish_url) = &self.on_publish_url {
            match self
                .request_client
                .post(on_publish_url)
                .body(body)
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_publish error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_publish success: {:?}", response);
                }
            }
        }
    }

    pub async fn on_unpublish_notify(&self, body: String) {
        if let Some(on_unpublish_url) = &self.on_unpublish_url {
            match self
                .request_client
                .post(on_unpublish_url)
                .body(body)
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

    pub async fn on_play_notify(&self, body: String) {
        if let Some(on_play_url) = &self.on_play_url {
            match self
                .request_client
                .post(on_play_url)
                .body(body)
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

    pub async fn on_stop_notify(&self, body: String) {
        if let Some(on_stop_url) = &self.on_stop_url {
            match self
                .request_client
                .post(on_stop_url)
                .body(body)
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
}
