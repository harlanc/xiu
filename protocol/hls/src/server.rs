use std::ops::Index;

use bytes::BytesMut;
// use super::errors::ServerError;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

use futures_util::{stream, StreamExt};
use networkio::bytes_writer::BytesWriter;
use std::io;

use futures::{channel::mpsc::unbounded, task::SpawnExt, SinkExt, Stream}; // 0.3.1, features = ["thread-pool"]

use {
    networkio::networkio::NetworkIO,
    std::{sync::Arc, time::Duration},
};

use tokio::fs::File;

use tokio_util::codec::{BytesCodec, FramedRead};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
static NOTFOUND: &[u8] = b"Not Found";

async fn handle_connection(req: Request<Body>) -> Result<Response<Body>> {
    let path = req.uri().path();

    let mut file_path: String = String::from("");

    if path.ends_with(".m3u8") {
        //http://127.0.0.1/app_name/stream_name/stream_name.m3u8
        let m3u8_index = path.find(".m3u8").unwrap();

        if m3u8_index > 0 {
            println!("{}: {}", m3u8_index, path);

            let (left, _) = path.split_at(m3u8_index);
            println!("11{}: {}", m3u8_index, left);
            let rv: Vec<_> = left.split("/").collect();
            for s in rv.clone() {
                println!("22{}: {}", m3u8_index, s);
            }

            let app_name = String::from(rv[1]);
            let stream_name = String::from(rv[2]);

            file_path = format!("./{}/{}/{}.m3u8", app_name, stream_name, stream_name);
        }
    } else if path.ends_with(".ts") {
        //http://127.0.0.1/app_name/stream_name/ts_name.m3u8
        let ts_index = path.find(".ts").unwrap();

        if ts_index > 0 {
            println!("{}: {}", ts_index, path);

            let (left, _) = path.split_at(ts_index);
            println!("11{}: {}", ts_index, left);
            let rv: Vec<_> = left.split("/").collect();
            for s in rv.clone() {
                println!("22{}: {}", ts_index, s);
            }

            let app_name = String::from(rv[1]);
            let stream_name = String::from(rv[2]);
            let ts_name = String::from(rv[3]);

            file_path = format!("./{}/{}/{}.ts", app_name, stream_name, ts_name);
        }
    }

    return simple_file_send(file_path.as_str()).await;
}

/// HTTP status code 404
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

async fn simple_file_send(filename: &str) -> Result<Response<Body>> {
    // Serve a file by asynchronously reading it by chunks using tokio-util crate.

    if let Ok(file) = File::open(filename).await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        return Ok(Response::new(body));
    }

    Ok(not_found())
}

pub async fn run() -> Result<()> {
    let addr = "0.0.0.0:8080".parse().unwrap();

    let new_service = make_service_fn(move |_| async {
        Ok::<_, GenericError>(service_fn(move |req| handle_connection(req)))
    });

    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    server.await?;

    Ok(())
}
