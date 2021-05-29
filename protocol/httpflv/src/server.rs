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
use tokio::sync::mpsc;
use tokio_util::codec::{BytesCodec, FramedRead};

use tokio_stream::wrappers::UnboundedReceiverStream;

use futures::{task::SpawnExt, SinkExt, Stream}; // 0.3.1, features = ["thread-pool"]

use {
    crate::rtmp::channels::define::{
        ChannelData, ChannelDataConsumer, ChannelDataProducer, ChannelEvent, ChannelEventProducer,
    },
    networkio::networkio::NetworkIO,
    std::{sync::Arc, time::Duration},
    // tokio::{
    //     sync::{mpsc, oneshot, Mutex},
    //     time::sleep,
    // },
};

//pub static mut event_producer : ChannelEventProducer ;//

type GenericError = Box<dyn std::error::Error + Send + Sync>;

type Result<T> = std::result::Result<T, GenericError>;

static INDEX: &[u8] = b"<a href=\"test.html\">test.html</a>";
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static NOTFOUND: &[u8] = b"Not Found";
static OK: &[u8] = b"OK";
static POST_DATA: &str = r#"{"original": "data"}"#;
static URL: &str = "http://127.0.0.1:1337/json_api";

async fn api_get_response() -> Result<Response<Body>> {
    let data = vec!["foo", "bar"];
    let res = match serde_json::to_string(&data) {
        Ok(json) => Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(INTERNAL_SERVER_ERROR.into())
            .unwrap(),
    };
    Ok(res)
}

fn stream(rv: HttpResponseDataConsumer) -> impl Stream<Item = io::Result<BytesMut>> {
    rv
}

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

            let app_name = String::from(rv[0]);
            let stream_name = String::from(rv[1]);

            let (http_response_data_producer, http_response_data_consumer) =
                mpsc::unbounded_channel();

            let mut flv_hanlder = HttpFlv::new(
                app_name,
                stream_name,
                event_producer,
                http_response_data_producer,
            );

            flv_hanlder.run();

            // Ok(Response::builder()
            //     .status(StatusCode::OK)
            //     .body(OK.into())
            //     .unwrap())

            let stream = UnboundedReceiverStream::new(http_response_data_consumer);

            let resp = Response::new(Body::wrap_stream(stream));

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

    // let shared_router = Arc::new(router);
    // let new_service = make_service_fn(move |_| {
    //     let app_state = AppState {
    //         state_thing: some_state.clone(),
    //     };

    //     let router_capture = shared_router.clone();
    //     async {
    //         Ok::<_, Error>(service_fn(move |req| {
    //             route(router_capture.clone(), req, app_state.clone())
    //         }))
    //     }
    // });

    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    server.await?;

    // let addr = "0.0.0.0:8080".parse().expect("address creation works");
    // let server = Server::bind(&addr).serve(new_service);
    // println!("Listening on http://{}", addr);
    // let _ = server.await;

    Ok(())
}

// pub struct HttpFlvServer {}

// impl HttpFlvServer {
//     async fn handle_connection(& mut self, req: Request<Body>) -> Result<Response<Body>> {
//         let path = req.uri().path();

//         match path.find(".flv") {
//             Some(index) if index > 0 => {
//                 println!("{}: {}", index, path);
//                 let (left, _) = path.split_at(index);
//                 println!("11{}: {}", index, left);
//                 let mut rv = left.split("/");
//                 for s in rv {
//                     println!("22{}: {}", index, s);
//                 }
//                 Ok(Response::builder()
//                     .status(StatusCode::OK)
//                     .body(OK.into())
//                     .unwrap())
//             }

//             _ => Ok(Response::builder()
//                 .status(StatusCode::NOT_FOUND)
//                 .body(NOTFOUND.into())
//                 .unwrap()),
//         }
//     }

//     pub async fn run(&'static mut self) -> Result<()> {
//         let addr = "0.0.0.0:13370".parse().unwrap();

//         let new_service = make_service_fn(move |_| async {
//             Ok::<_, GenericError>(service_fn(move |req| self.handle_connection(req)))
//         });

//         let server = Server::bind(&addr).serve(new_service);
//         println!("Listening on http://{}", addr);
//         server.await?;

//         Ok(())
//     }
// }
