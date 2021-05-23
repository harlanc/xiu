// use super::errors::ServerError;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Client, Method, Request, Response, Server, StatusCode};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

static INDEX: &[u8] = b"<a href=\"test.html\">test.html</a>";
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static NOTFOUND: &[u8] = b"Not Found";
static POST_DATA: &str = r#"{"original": "data"}"#;
static URL: &str = "http://127.0.0.1:1337/json_api";

pub struct HttpFlvServer {
    port: u32,
}

impl HttpFlvServer {
    pub fn new(port: u32) -> Self {
        Self { port: port }
    }

    async fn api_get_response(&mut self) -> Result<Response<Body>, Error> {
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

    async fn route(&'static mut self, req: Request<Body>) -> Result<Response<Body>, Error> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/json_api") => self.api_get_response().await,
            _ => {
                // Return 404 not found response.
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(NOTFOUND.into())
                    .unwrap())
            }
        }
    }

    pub async fn run(&'static mut self) {
        let new_service = make_service_fn(move |_| {
            // Move a clone of `client` into the `service_fn`.

            async {
                Ok::<_, GenericError>(service_fn(move | req| {
                    // Clone again to ensure that client outlives this closure.
                    self.route(req)
                }))
            }
        });

        let addr = "127.0.0.1:1337".parse().unwrap();
        let server = Server::bind(&addr).serve(new_service);
        println!("Listening on http://{}", addr);
        let _ = server.await;
    }
}
