use std::ops::Index;

use bytes::BytesMut;
// use super::errors::ServerError;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
use super::define::HttpResponseDataConsumer;
use super::define::HttpResponseDataProducer;
use super::httpflv::HttpFlv;
use futures_util::{stream, StreamExt};
use networkio::bytes_writer::BytesWriter;
use std::io;

use futures::{channel::mpsc::unbounded, task::SpawnExt, SinkExt, Stream}; // 0.3.1, features = ["thread-pool"]

use {
    crate::rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventProducer,
    },
    networkio::networkio::NetworkIO,
    std::{sync::Arc, time::Duration},
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
            println!("{}: {}", index, path);
            let (left, _) = path.split_at(index);
            println!("11{}: {}", index, left);
            let rv: Vec<_> = left.split("/").collect();
            for s in rv.clone() {
                println!("22{}: {}", index, s);
            }

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
                    print!("pull client error {}\n", err);
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

pub async fn run(event_producer: ChannelEventProducer) -> Result<()> {
    let addr = "0.0.0.0:13370".parse().unwrap();

    let new_service = make_service_fn(move |_| {
        let flv_copy = event_producer.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                handle_connection(req, flv_copy.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    server.await?;

    Ok(())
}
