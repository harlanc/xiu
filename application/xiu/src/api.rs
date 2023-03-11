use {
    anyhow::Result,
    axum::{routing::get, Router},
    rtmp::{channels::define, channels::define::ChannelEventProducer},
    {
        tokio,
        tokio::sync::{mpsc, oneshot},
    },
};
#[derive(Clone)]
struct ApiService {
    channel_event_producer: ChannelEventProducer,
}

impl ApiService {
    async fn get_stream_status(&self) -> Result<String> {
        let (data_sender, mut data_receiver) = mpsc::unbounded_channel();
        let (size_sender, size_receiver) = oneshot::channel();
        let channel_event = define::ChannelEvent::Api {
            data_sender,
            size_sender,
        };
        if let Err(err) = self.channel_event_producer.send(channel_event) {
            log::error!("send api event error: {}", err);
        }
        let mut data = Vec::new();
        match size_receiver.await {
            Ok(size) => {
                if size == 0 {
                    return Ok(String::from("no stream data"));
                }
                loop {
                    if let Some(stream_statistics) = data_receiver.recv().await {
                        data.push(stream_statistics);
                    }

                    if data.len() == size {
                        break;
                    }
                }
            }
            Err(err) => {
                log::error!("start_api_service recv size error: {}", err);
            }
        }

        if let Ok(data) = serde_json::to_string(&data) {
            return Ok(data);
        }

        Ok(String::from(""))
    }
}

pub async fn run(producer: ChannelEventProducer, port: usize) {
    let api = ApiService {
        channel_event_producer: producer,
    };

    //https://stackoverflow.com/questions/73251151/how-to-call-struct-method-from-axum-server-route
    let status = move || async move {
        match api.get_stream_status().await {
            Ok(response) => response,
            Err(_) => "error".to_owned(),
        }
    };

    let app = Router::new().route("/get_stream_status", get(status));
    log::info!("Http api server listening on http://:{}", port);
    axum::Server::bind(&([127, 0, 0, 1], port as u16).into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
