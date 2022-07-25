use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use hyper::{service::Service, Body, Request, Response, StatusCode};
use tokio::{fs::File, sync::oneshot};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::{
    hls_event_manager::{HlsEvent, M3u8Event, StpMap},
    m3u8::M3u8PlaylistResponse,
};

static NOTFOUND: &[u8] = b"Not Found";

pub struct HlsHandler {
    stp_map: StpMap,
}

impl Service<Request<Body>> for HlsHandler {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // create a response in a future.
        // let fut = async {
        //     Ok(resp)
        // };

        // // Return the response as an immediate future
        // Box::pin(fut)

        let path = req.uri().path();
        let directives = req
            .uri()
            .query()
            .map(|v| {
                url::form_urlencoded::parse(v.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_else(HashMap::new);

        let mut file_path: String = String::from("");

        if path.ends_with(".m3u8") {
            let msn = directives.get("_HLS_msn");
            let part = directives.get("_HLS_part");

            if part.is_some() && msn.is_none() {
                // Client sent an invalid request: https://datatracker.ietf.org/doc/html/draft-pantos-hls-rfc8216bis#section-6.2.5.2
                return Box::pin(async { Ok(bad_request()) });
            }

            //http://127.0.0.1/app_name/stream_name/stream_name.m3u8
            let m3u8_index = path.find(".m3u8").unwrap();

            if m3u8_index > 0 {
                let (left, _) = path.split_at(m3u8_index);
                let rv: Vec<_> = left.split("/").collect();

                let app_name = String::from(rv[1]);
                let stream_name = String::from(rv[2]);

                println!("msn: {:?}", msn);
                if let Some(msn_s) = msn {
                    // Client wants us to hold the request until media segment number msn is generated

                    let msn_ur: Result<u64, _> = msn_s.parse();

                    if msn_ur.is_err() {
                        return Box::pin(async { Ok(bad_request()) });
                    }

                    let msn_u = msn_ur.unwrap();

                    // TODO: unsafe
                    let hm_read = self.stp_map.read().unwrap();

                    let stream_event_channel = hm_read.get(&stream_name);

                    if stream_event_channel.is_none() {
                        return Box::pin(async { Ok(not_found()) });
                    } else if let Some((tx, _rx, m3u8_prod)) = stream_event_channel {
                        let mut rc = tx.clone().subscribe();
                        let mut mp = m3u8_prod.clone();

                        let fp = format!("./{}/{}/{}.m3u8", app_name, stream_name, stream_name);
                        return Box::pin(async move {
                            let (resp_tx, resp_rx) = oneshot::channel();

                            let q = M3u8Event::RequestPlaylist { channel: resp_tx };

                            mp.send(q).await;

                            let M3u8PlaylistResponse { sequence_no: seq } = resp_rx.await.unwrap();

                            if seq > msn_u {
                                // sequence already exists
                                return simple_file_send(fp.as_str()).await;
                            }

                            // if msn_u > seq + 2 {
                            //     // sequence too far in future
                            //     return Ok(bad_request());
                            // }

                            loop {
                                let m = rc.recv().await;

                                if let Ok(HlsEvent::HlsSequenceIncr { sequence: seq }) = m {
                                    if seq != msn_u {
                                        continue;
                                    };

                                    break;
                                } else {
                                    continue;
                                }
                            }

                            simple_file_send(fp.as_str()).await
                        });
                    }
                }

                file_path = format!("./{}/{}/{}.m3u8", app_name, stream_name, stream_name);
            }
        } else if path.ends_with(".ts") {
            //http://127.0.0.1/app_name/stream_name/ts_name.m3u8
            let ts_index = path.find(".ts").unwrap();

            if ts_index > 0 {
                let (left, _) = path.split_at(ts_index);

                let rv: Vec<_> = left.split("/").collect();

                let app_name = String::from(rv[1]);
                let stream_name = String::from(rv[2]);
                let ts_name = String::from(rv[3]);

                file_path = format!("./{}/{}/{}.ts", app_name, stream_name, ts_name);
            }
        }
        let f = async move { simple_file_send(file_path.as_str()).await };

        Box::pin(f)
    }
}

pub struct MakeHlsHandler {
    pub stp_map: StpMap,
}

impl<T> Service<T> for MakeHlsHandler {
    type Response = HlsHandler;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let stp_map = self.stp_map.clone();
        let fut = async move { Ok(HlsHandler { stp_map }) };
        Box::pin(fut)
    }
}

/// HTTP status code 400
fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(NOTFOUND.into())
        .unwrap()
}

/// HTTP status code 404
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

async fn simple_file_send(filename: &str) -> Result<Response<Body>, hyper::Error> {
    // Serve a file by asynchronously reading it by chunks using tokio-util crate.

    if let Ok(file) = File::open(filename).await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        let r = Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .header("Access-Control-Allow-Headers", "*")
            .body(body);
        return Ok(r.unwrap());
    }

    Ok(not_found())
}
