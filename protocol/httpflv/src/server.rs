use {
    super::httpflv::HttpFlv,
    futures::channel::mpsc::unbounded,
    hyper::{
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server, StatusCode,
    },
    rtmp::channels::define::ChannelEventProducer,
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
static NOTFOUND: &[u8] = b"Not Found";

async fn handle_connection(
    req: Request<Body>,
    event_producer: ChannelEventProducer, // event_producer: ChannelEventProducer
) -> Result<Response<Body>> {
    let path = req.uri().path();

    match path.find(".flv") {
        Some(index) if index > 0 => {
            let (left, _) = path.split_at(index);
            let rv: Vec<_> = left.split("/").collect();

            let app_name = String::from(rv[1]);
            let stream_name = String::from(rv[2]);

            let (http_response_data_producer, http_response_data_consumer) = unbounded();

            let mut flv_hanlder = HttpFlv::new(
                app_name,
                stream_name,
                event_producer,
                http_response_data_producer,
            );

            tokio::spawn(async move {
                if let Err(err) = flv_hanlder.run().await {
                    log::error!("flv handler run error {}\n", err);
                }
            });

            let mut resp = Response::new(Body::wrap_stream(http_response_data_consumer));
            resp.headers_mut()
                .insert("Access-Control-Allow-Origin", "*".parse().unwrap());

            Ok(resp)
        }

        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()),
    }
}

pub async fn run(event_producer: ChannelEventProducer, port: u32) -> Result<()> {
    let listen_address = format!("0.0.0.0:{}", port);
    let sock_addr = listen_address.parse().unwrap();

    let new_service = make_service_fn(move |_| {
        let flv_copy = event_producer.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                handle_connection(req, flv_copy.clone())
            }))
        }
    });

    let server = Server::bind(&sock_addr).serve(new_service);

    log::info!("Httpflv server listening on http://{}", sock_addr);

    server.await?;

    Ok(())
}
