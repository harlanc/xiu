use std::time::Duration;
use tokio::time::sleep;
use {
    super::gb28181::GB28181Server,
    anyhow::Result,
    axum::{
        routing::{get, post},
        Json, Router,
    },
    serde::Deserialize,
    std::sync::Arc,
    streamhub::{define, define::StreamHubEventSender, utils::Uuid, StreamsHub},
    {
        tokio,
        tokio::sync::{mpsc, oneshot},
    },
};

use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Deserialize)]
struct RequestFormData {
    secret: String,
    schema: Option<String>,
    stream_name: Option<String>,
    re_use_port: Option<String>,
    port: Option<u16>,
    tcp_mode: Option<String>,
    need_dump: Option<bool>, //vhost : String,
}

#[derive(Serialize)]
struct HttpResponse {
    code: i16,
    changed: Option<usize>,
    stream_id: Option<String>,
    schema: Option<String>,
}

#[derive(Clone)]
struct ApiService {
    channel_event_producer: StreamHubEventSender,
    secret: String,
}

impl ApiService {
    async fn root(&self) -> String {
        String::from(
            "Usage of xiu http api:
                ./get_stream_status(get)  get audio and video stream statistic information.
                ./kick_off_client(post) kick off client by publish/subscribe id.\n",
        )
    }

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

        return Ok(serde_json::to_string(&response_body)?);
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
        // let response_body = if request_data.secret == self.secret {
        //     serde_json::json!({
        //     "code": 0,
        //         })
        // } else {
        //     serde_json::json!({
        //         "code": -1,
        //     })
        // };

        // Json(response_body)

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
        &self,
        request_data: axum::extract::Form<RequestFormData>,
    ) -> Json<serde_json::Value> {
        log::info!("open rtp server 0");
        if request_data.secret != self.secret {
            return Json(serde_json::json!({
            "code": -1,
            "err_message": "The secret is not correct"
            }));
        };

        if request_data.stream_name.is_none() {
            return Json(serde_json::json!({
            "code": -2,
            "err_message": "The stream name should not be empty"
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

        let mut rtp_server = GB28181Server::new(
            local_port,
            self.channel_event_producer.clone(),
            request_data.stream_name.clone().unwrap(),
            need_dump,
        );
        tokio::spawn(async move {
            if let Err(err) = rtp_server.run().await {
                log::error!("rtp server error: {}\n", err);
            }
        });

        let response_body = serde_json::json!({
        "code": 0,
        "port":30000
        });

        Json(response_body)
    }

    async fn close_rtp_server(
        &self,
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

        Json(response_body)
    }
}

pub async fn run(producer: StreamHubEventSender, port: usize) {
    // let mut rtp_server = GB28181Server::new(30000, producer.clone());
    // tokio::spawn(async move {
    //     if let Err(err) = rtp_server.run().await {
    //         log::error!("rtp server error: {}\n", err);
    //     }
    // });

    tokio::spawn(async move {
        let request_client = reqwest::Client::new();

        let start_url = "http://192.168.0.104:8081/index/hook/on_server_started";

        let payload = serde_json::json!({
            "api.apiDebug" : "1",
	"api.defaultSnap" : "./www/logo.png",
	"api.secret" : "10000",
	"api.snapRoot" : "./www/snap/",
	"cluster.origin_url" : "",
	"cluster.retry_count" : "3",
	"cluster.timeout_sec" : "15",
	"ffmpeg.bin" : "/usr/bin/ffmpeg",
	"ffmpeg.cmd" : "%s -re -i %s -c:a aac -strict -2 -ar 44100 -ab 48k -c:v libx264 -f flv %s",
	"ffmpeg.log" : "./ffmpeg/ffmpeg.log",
	"ffmpeg.restart_sec" : "0",
	"ffmpeg.snap" : "%s -rtsp_transport tcp -i %s -y -f mjpeg -t 0.001 %s",
	"general.check_nvidia_dev" : "1",
	"general.enableVhost" : "0",
	"general.enable_ffmpeg_log" : "0",
	"general.flowThreshold" : "1024",
	"general.maxStreamWaitMS" : "15000",
	"general.mediaServerId" : "xiu",
	"general.mergeWriteMS" : "0",
	"general.resetWhenRePlay" : "1",
	"general.streamNoneReaderDelayMS" : "20000",
	"general.unready_frame_cache" : "100",
	"general.wait_add_track_ms" : "3000",
	"general.wait_track_ready_ms" : "10000",
	"hls.broadcastRecordTs" : "0",
	"hls.deleteDelaySec" : "10",
	"hls.fileBufSize" : "65536",
	"hls.segDur" : "2",
	"hls.segKeep" : "0",
	"hls.segNum" : "3",
	"hls.segRetain" : "5",
	"hook.alive_interval" : "10.0",
	"hook.enable" : "1",
	"hook.on_flow_report" : "",
	"hook.on_http_access" : "",
	"hook.on_play" : "http://192.168.0.104:8081/index/hook/on_play",
	"hook.on_publish" : "http://192.168.0.104:8081/index/hook/on_publish",
	"hook.on_record_mp4" : "http://127.0.0.1:18081/api/record/on_record_mp4",
	"hook.on_record_ts" : "",
	"hook.on_rtp_server_timeout" : "http://192.168.0.104:8081/index/hook/on_rtp_server_timeout",
	"hook.on_rtsp_auth" : "",
	"hook.on_rtsp_realm" : "",
	"hook.on_send_rtp_stopped" : "http://192.168.0.104:8081/index/hook/on_send_rtp_stopped",
	"hook.on_server_exited" : "https://127.0.0.1/index/hook/on_server_exited",
	"hook.on_server_keepalive" : "http://192.168.0.104:8081/index/hook/on_server_keepalive",
	"hook.on_server_started" : "http://192.168.0.104:8081/index/hook/on_server_started",
	"hook.on_shell_login" : "",
	"hook.on_stream_changed" : "http://192.168.0.104:8081/index/hook/on_stream_changed",
	"hook.on_stream_none_reader" : "http://192.168.0.104:8081/index/hook/on_stream_none_reader",
	"hook.on_stream_not_found" : "http://192.168.0.104:8081/index/hook/on_stream_not_found",
	"hook.retry" : "1",
	"hook.retry_delay" : "3.0",
	"hook.stream_changed_schemas" : "rtsp/rtmp/fmp4/ts/hls/hls.fmp4",
	"hook.timeoutSec" : "20",
	"hook_index" : 0,
	"http.allow_cross_domains" : "1",
	"http.allow_ip_range" : "::1,127.0.0.1,172.16.0.0-172.31.255.255,192.168.0.0-192.168.255.255,10.0.0.0-10.255.255.255",
	"http.charSet" : "utf-8",
	"http.dirMenu" : "1",
	"http.forbidCacheSuffix" : "",
	"http.forwarded_ip_header" : "",
	"http.keepAliveSecond" : "30",
	"http.maxReqSize" : "40960",
	"http.notFound" : "",
	"http.port" : "8080",
	"http.rootPath" : "./www",
	"http.sendBufSize" : "65536",
	"http.sslport" : "1443",
	"http.virtualPath" : "",
	"mediaServerId" : "xiu",
	"multicast.addrMax" : "239.255.255.255",
	"multicast.addrMin" : "239.0.0.0",
	"multicast.udpTTL" : "64",
	"protocol.add_mute_audio" : "1",
	"protocol.auto_close" : "0",
	"protocol.continue_push_ms" : "3000",
	"protocol.enable_audio" : "1",
	"protocol.enable_fmp4" : "1",
	"protocol.enable_hls" : "1",
	"protocol.enable_hls_fmp4" : "0",
	"protocol.enable_mp4" : "0",
	"protocol.enable_rtmp" : "1",
	"protocol.enable_rtsp" : "1",
	"protocol.enable_ts" : "1",
	"protocol.fmp4_demand" : "0",
	"protocol.hls_demand" : "0",
	"protocol.hls_save_path" : "./www",
	"protocol.modify_stamp" : "2",
	"protocol.mp4_as_player" : "0",
	"protocol.mp4_max_second" : "3600",
	"protocol.mp4_save_path" : "./www",
	"protocol.rtmp_demand" : "0",
	"protocol.rtsp_demand" : "0",
	"protocol.ts_demand" : "0",
	"record.appName" : "record",
	"record.fastStart" : "0",
	"record.fileBufSize" : "65536",
	"record.fileRepeat" : "0",
	"record.sampleMS" : "500",
	"rtc.externIP" : "",
	"rtc.port" : "8000",
	"rtc.preferredCodecA" : "PCMU,PCMA,opus,mpeg4-generic",
	"rtc.preferredCodecV" : "H264,H265,AV1,VP9,VP8",
	"rtc.rembBitRate" : "0",
	"rtc.tcpPort" : "8000",
	"rtc.timeoutSec" : "15",
	"rtmp.handshakeSecond" : "15",
	"rtmp.keepAliveSecond" : "15",
	"rtmp.port" : "1935",
	"rtmp.sslport" : "0",
	"rtp.audioMtuSize" : "600",
	"rtp.h264_stap_a" : "1",
	"rtp.lowLatency" : "0",
	"rtp.rtpMaxSize" : "10",
	"rtp.videoMtuSize" : "1400",
	"rtp_proxy.dumpDir" : "",
	"rtp_proxy.gop_cache" : "1",
	"rtp_proxy.h264_pt" : "98",
	"rtp_proxy.h265_pt" : "99",
	"rtp_proxy.opus_pt" : "100",
	"rtp_proxy.port" : "10000",
	"rtp_proxy.port_range" : "30000-35000",
	"rtp_proxy.ps_pt" : "96",
	"rtp_proxy.timeoutSec" : "15",
	"rtsp.authBasic" : "0",
	"rtsp.directProxy" : "1",
	"rtsp.handshakeSecond" : "15",
	"rtsp.keepAliveSecond" : "15",
	"rtsp.lowLatency" : "0",
	"rtsp.port" : "554",
	"rtsp.rtpTransportType" : "-1",
	"rtsp.sslport" : "0",
	"shell.maxReqSize" : "1024",
	"shell.port" : "0",
	"srt.latencyMul" : "4",
	"srt.pktBufSize" : "8192",
	"srt.port" : "9000",
	"srt.timeoutSec" : "5"
        })
        .to_string();

        // Send POST request
        let response = request_client
            .post(start_url)
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await;

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    println!("POST request successful!");
                } else {
                    println!("POST request failed with status code: {}", res.status());
                }
            }
            Err(err) => {
                eprintln!("POST request error: {}", err);
            }
        }

        sleep(Duration::from_secs(10)).await;
        let url = "http://192.168.0.104:8081/index/hook/on_server_keepalive";
        let mut hook_indx = 0;

        loop {
            hook_indx += 1;

            let payload = serde_json::json!({
                "mediaServerId": "xiu",
                "hook_index":hook_indx
            })
            .to_string();

            // Send POST request
            let response = request_client
                .post(url)
                .header("Content-Type", "application/json")
                .body(payload)
                .send()
                .await;

            match response {
                Ok(res) => {
                    if res.status().is_success() {
                        println!("POST request successful!");
                    } else {
                        println!("POST request failed with status code: {}", res.status());
                    }
                }
                Err(err) => {
                    eprintln!("POST request error: {}", err);
                }
            }

            // Delay for 10 seconds
            sleep(Duration::from_secs(10)).await;
        }
    });

    log::info!("====================");

    let api = Arc::new(ApiService {
        channel_event_producer: producer,
        secret: String::from("xiu"),
    });

    let api_root = api.clone();
    let root = move || async move { api_root.root().await };

    let api_0 = api.clone();
    let get_server_config = move |request_data: axum::extract::Form<RequestFormData>| async move {
        match api_0.get_server_config(request_data).await {
            Ok(response) => response,
            Err(_) => "error".to_owned(),
        }
    };

    let api_1 = api.clone();
    let set_server_config = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_1.set_server_config(request_data).await
    };

    let api_2 = api.clone();
    let get_media_list = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_2.get_media_list(request_data).await
    };

    let api_3 = api.clone();
    let get_rtp_info = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_3.get_rtp_info(request_data).await
    };

    let api_4 = api.clone();
    let open_rtp_server = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_4.open_rtp_server(request_data).await
    };

    let api_5 = api.clone();
    let close_rtp_server = move |request_data: axum::extract::Form<RequestFormData>| async move {
        api_5.close_rtp_server(request_data).await
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/index/api/getServerConfig", post(get_server_config))
        .route("/index/api/setServerConfig", post(set_server_config))
        .route("/index/api/getMediaList", post(get_media_list))
        .route("/index/api/getRtpInfo", post(get_rtp_info))
        .route("/index/api/openRtpServer", post(open_rtp_server))
        .route("/index/api/clostRtpServer", post(close_rtp_server));

    log::info!("Http api server listening on http://:{}", port);
    axum::Server::bind(&([192, 168, 0, 104], port as u16).into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
