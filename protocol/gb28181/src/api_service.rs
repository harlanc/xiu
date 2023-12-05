use tokio::sync::Mutex;

use {
    super::gb28181::GB28181Server,
    anyhow::Result,
    axum::{routing::post, Json, Router},
    serde::Deserialize,
    std::sync::Arc,
    streamhub::define::StreamHubEventSender,
    tokio,
};

use axum::http::StatusCode;
use serde::Serialize;

#[derive(Deserialize)]
pub struct RequestFormData {
    secret: String,
    pub schema: Option<String>,
    stream_id: Option<String>,
    pub re_use_port: Option<String>,
    port: Option<u16>,
    pub tcp_mode: Option<String>,
    need_dump: Option<bool>, //vhost : String,
}

#[derive(Serialize)]
struct HttpResponse {
    code: i16,
    changed: Option<usize>,
    stream_id: Option<String>,
    schema: Option<String>,
}

struct ApiService {
    secret: String,
    gb_server: GB28181Server,
}

impl ApiService {
    async fn get_server_config(
        &self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Result<String> {
        let response_body = if request_data.secret == self.secret {
            HttpResponse {
                code: 0,
                changed: None,
                stream_id: None,
                schema: None,
            }
        } else {
            HttpResponse {
                code: -1,
                changed: None,
                stream_id: None,
                schema: None,
            }
        };

        Ok(serde_json::to_string(&response_body)?)
    }

    async fn set_server_config(
        &self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Json<serde_json::Value> {
        let response_body = if request_data.secret == self.secret {
            serde_json::json!({
                "code": 0,
                "changed": 0,
            })
        } else {
            serde_json::json!({
                "code": -1,
            })
        };

        Json(response_body)
    }

    async fn get_media_list(
        &self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> (StatusCode, Json<HttpResponse>) {
        let response_body = if request_data.secret == self.secret {
            HttpResponse {
                code: 0,
                changed: None,
                stream_id: None,
                schema: None,
            }
        } else {
            HttpResponse {
                code: -1,
                changed: None,
                stream_id: None,
                schema: None,
            }
        };

        (StatusCode::OK, Json(response_body))
    }

    async fn get_rtp_info(
        &self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Json<serde_json::Value> {
        let response_body = if request_data.secret == self.secret {
            serde_json::json!({
            "code": 0,
            "exist":false
                })
        } else {
            serde_json::json!({
                "code": -1,
            })
        };

        Json(response_body)
    }

    async fn open_rtp_server(
        &mut self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Json<serde_json::Value> {
        log::info!("open rtp server");
        if request_data.secret != self.secret {
            return Json(serde_json::json!({
            "code": -1,
            "err_msg": "The secret is not correct"
            }));
        };

        if request_data.stream_id.is_none() {
            return Json(serde_json::json!({
            "code": -2,
            "err_msg": "The stream name should not be empty"
            }));
        }

        let local_port = if let Some(port) = request_data.port {
            port
        } else {
            0
        };

        let need_dump = if let Some(dump) = request_data.need_dump {
            dump
        } else {
            false
        };

        match self
            .gb_server
            .start_session(
                local_port,
                request_data.stream_id.clone().unwrap(),
                need_dump,
            )
            .await
        {
            Ok(port) => {
                let response_body = serde_json::json!({
                "code": 0,
                "port":port,
                });

                Json(response_body)
            }
            Err(err) => {
                let response_body = serde_json::json!({
                "code": -3,
                "err_msg":format!("{:?}",err)
                });

                Json(response_body)
            }
        }
    }

    async fn close_rtp_server(
        &mut self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Json<serde_json::Value> {
        let response_body = if request_data.secret == self.secret {
            serde_json::json!({
            "code": 0,
                })
        } else {
            serde_json::json!({
                "code": -1,
            })
        };

        self.gb_server
            .stop_session(request_data.stream_id.clone().unwrap())
            .await;

        Json(response_body)
    }
}

pub async fn run(producer: StreamHubEventSender, port: usize) {
    let api = Arc::new(Mutex::new(ApiService {
        secret: String::from("xiu"),
        gb_server: GB28181Server::new(producer),
    }));

    let api_0 = api.clone();
    let get_server_config = move |request_data: axum::extract::Form<RequestFormData>| async move {
        match api_0.lock().await.get_server_config(request_data).await {
            Ok(response) => response,
            Err(_) => "error".to_owned(),
        }
    };

    let api_1 = api.clone();
    let set_server_config = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_1.lock().await.set_server_config(request_data).await
    };

    let api_2 = api.clone();
    let get_media_list = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_2.lock().await.get_media_list(request_data).await
    };

    let api_3 = api.clone();
    let get_rtp_info = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_3.lock().await.get_rtp_info(request_data).await
    };

    let api_4 = api.clone();
    let open_rtp_server = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_4.lock().await.open_rtp_server(request_data).await
    };

    let api_5 = api.clone();
    let close_rtp_server = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_5.lock().await.close_rtp_server(request_data).await
    };

    let app = Router::new()
        .route("/index/api/getServerConfig", post(get_server_config))
        .route("/index/api/setServerConfig", post(set_server_config))
        .route("/index/api/getMediaList", post(get_media_list))
        .route("/index/api/getRtpInfo", post(get_rtp_info))
        .route("/index/api/openRtpServer", post(open_rtp_server))
        .route("/index/api/clostRtpServer", post(close_rtp_server));

    log::info!("GB28181 api server listening on http://:{}", port);
    axum::Server::bind(&([127, 0, 0, 1], port as u16).into())
        .serve(app.into_make_service())
        .await
        .unwrap();

    log::info!("GB28181 api server end...");
}
