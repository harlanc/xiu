use std::ops::Index;

// use super::errors::ServerError;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
use futures_util::{stream, StreamExt};
// use hyper::client::HttpConnector;

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

async fn handle_connection(req: Request<Body>) -> Result<Response<Body>> {
    let path = req.uri().path();

    match path.find(".flv") {
        Some(index) if index > 0 => {
            println!("{}: {}", index, path);
            let (left, _) = path.split_at(index);
            println!("11{}: {}", index, left);
            let mut rv = left.split("/");
            for s in rv{
                println!("22{}: {}", index, s);
            }
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(OK.into())
                .unwrap())
        }

        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()),
    }

    // if let Some(index) = path.find(".flv") && (index >0){

    // }

    // match (req.method(), req.uri().path()) {
    //     (&Method::GET, "/test.html") => api_get_response().await,

    //     _ => {
    //         println!("{}:{}", req.method(), req.uri().path());
    //         // Return 404 not found response.

    //     }
    // }
}

pub async fn run() -> Result<()> {
    let addr = "0.0.0.0:13370".parse().unwrap();

    let new_service = make_service_fn(move |_| async {
        Ok::<_, GenericError>(service_fn(move |req| handle_connection(req)))
    });

    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    server.await?;

    Ok(())
}
