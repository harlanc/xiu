use rtmp::remuxer::RtmpRemuxer;

use {
    super::api,
    super::config::Config,
    //https://rustcc.cn/article?id=6dcbf032-0483-4980-8bfe-c64a7dfb33c7
    anyhow::Result,
    hls::remuxer::HlsRemuxer,
    hls::server as hls_server,
    httpflv::server as httpflv_server,
    rtmp::{
        relay::{pull_client::PullClient, push_client::PushClient},
        rtmp::RtmpServer,
    },
    streamhub::{notify::Notifier, StreamsHub},
    tokio,
    xrtsp::rtsp::RtspServer,
    xwebrtc::webrtc::WebRTCServer,
};

pub struct Service {
    cfg: Config,
}

impl Service {
    pub fn new(cfg: Config) -> Self {
        Service { cfg }
    }

    pub async fn run(&mut self) -> Result<()> {
        let notifier = if let Some(httpnotifier) = &self.cfg.httpnotify {
            if !httpnotifier.enabled {
                None
            } else {
                Some(Notifier::new(
                    httpnotifier.on_publish.clone(),
                    httpnotifier.on_unpublish.clone(),
                    httpnotifier.on_play.clone(),
                    httpnotifier.on_stop.clone(),
                ))
            }
        } else {
            None
        };

        let mut stream_hub = StreamsHub::new(notifier);

        self.start_httpflv(&mut stream_hub).await?;
        self.start_hls(&mut stream_hub).await?;
        self.start_rtmp(&mut stream_hub).await?;
        self.start_rtsp(&mut stream_hub).await?;
        self.start_webrtc(&mut stream_hub).await?;
        self.start_http_api_server(&mut stream_hub).await?;
        self.start_rtmp_remuxer(&mut stream_hub).await?;

        tokio::spawn(async move {
            stream_hub.run().await;
            log::info!("stream hub end...");
        });
        Ok(())
    }

    async fn start_http_api_server(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let producer = stream_hub.get_hub_event_sender();

        let http_api_port = if let Some(httpapi) = &self.cfg.httpapi {
            httpapi.port
        } else {
            8000
        };

        tokio::spawn(async move {
            api::run(producer, http_api_port).await;
        });
        Ok(())
    }

    async fn start_rtmp(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let rtmp_cfg = &self.cfg.rtmp;

        if let Some(rtmp_cfg_value) = rtmp_cfg {
            if !rtmp_cfg_value.enabled {
                return Ok(());
            }

            let gop_num = if let Some(gop_num_val) = rtmp_cfg_value.gop_num {
                gop_num_val
            } else {
                1
            };

            let producer = stream_hub.get_hub_event_sender();

            /*static push */
            if let Some(push_cfg_values) = &rtmp_cfg_value.push {
                for push_value in push_cfg_values {
                    if !push_value.enabled {
                        continue;
                    }
                    log::info!("start rtmp push client..");
                    let address = format!(
                        "{ip}:{port}",
                        ip = push_value.address,
                        port = push_value.port
                    );

                    let mut push_client = PushClient::new(
                        address,
                        stream_hub.get_client_event_consumer(),
                        producer.clone(),
                    );
                    tokio::spawn(async move {
                        if let Err(err) = push_client.run().await {
                            log::error!("push client error {}\n", err);
                        }
                    });

                    stream_hub.set_rtmp_push_enabled(true);
                }
            }
            /*static pull*/
            if let Some(pull_cfg_value) = &rtmp_cfg_value.pull {
                if pull_cfg_value.enabled {
                    let address = format!(
                        "{ip}:{port}",
                        ip = pull_cfg_value.address,
                        port = pull_cfg_value.port
                    );
                    log::info!("start rtmp pull client from address: {}", address);
                    let mut pull_client = PullClient::new(
                        address,
                        stream_hub.get_client_event_consumer(),
                        producer.clone(),
                    );

                    tokio::spawn(async move {
                        if let Err(err) = pull_client.run().await {
                            log::error!("pull client error {}\n", err);
                        }
                    });

                    stream_hub.set_rtmp_pull_enabled(true);
                }
            }

            let listen_port = rtmp_cfg_value.port;
            let address = format!("0.0.0.0:{listen_port}");

            let mut rtmp_server = RtmpServer::new(address, producer, gop_num);
            tokio::spawn(async move {
                if let Err(err) = rtmp_server.run().await {
                    log::error!("rtmp server error: {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_rtmp_remuxer(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        //The remuxer now is used for rtsp2rtmp, so both rtsp/rtmp cfg need to be enabled.
        let mut rtsp_enabled = false;
        if let Some(rtsp_cfg_value) = &self.cfg.rtsp {
            if rtsp_cfg_value.enabled {
                rtsp_enabled = true;
            }
        }
        if !rtsp_enabled {
            return Ok(());
        }

        let mut rtmp_enabled: bool = false;
        if let Some(rtmp_cfg_value) = &self.cfg.rtmp {
            if rtmp_cfg_value.enabled {
                rtmp_enabled = true;
            }
        }
        if !rtmp_enabled {
            return Ok(());
        }

        let event_producer = stream_hub.get_hub_event_sender();
        let broadcast_event_receiver = stream_hub.get_client_event_consumer();
        let mut remuxer = RtmpRemuxer::new(broadcast_event_receiver, event_producer);
        stream_hub.set_rtmp_remuxer_enabled(true);

        tokio::spawn(async move {
            if let Err(err) = remuxer.run().await {
                log::error!("rtmp remuxer server error: {}\n", err);
            }
        });
        Ok(())
    }

    async fn start_rtsp(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let rtsp_cfg = &self.cfg.rtsp;

        if let Some(rtsp_cfg_value) = rtsp_cfg {
            if !rtsp_cfg_value.enabled {
                return Ok(());
            }

            let producer = stream_hub.get_hub_event_sender();

            let listen_port = rtsp_cfg_value.port;
            let address = format!("0.0.0.0:{listen_port}");

            let mut rtsp_server = RtspServer::new(address, producer);
            tokio::spawn(async move {
                if let Err(err) = rtsp_server.run().await {
                    log::error!("rtsp server error: {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_webrtc(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let webrtc_cfg = &self.cfg.webrtc;

        if let Some(webrtc_cfg_value) = webrtc_cfg {
            if !webrtc_cfg_value.enabled {
                return Ok(());
            }

            let producer = stream_hub.get_hub_event_sender();

            let listen_port = webrtc_cfg_value.port;
            let address = format!("0.0.0.0:{listen_port}");

            let mut webrtc_server = WebRTCServer::new(address, producer);
            tokio::spawn(async move {
                if let Err(err) = webrtc_server.run().await {
                    log::error!("webrtc server error: {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_httpflv(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let httpflv_cfg = &self.cfg.httpflv;

        if let Some(httpflv_cfg_value) = httpflv_cfg {
            if !httpflv_cfg_value.enabled {
                return Ok(());
            }
            let port = httpflv_cfg_value.port;
            let event_producer = stream_hub.get_hub_event_sender();

            tokio::spawn(async move {
                if let Err(err) = httpflv_server::run(event_producer, port).await {
                    log::error!("httpflv server error: {}\n", err);
                }
            });
        }

        Ok(())
    }

    async fn start_hls(&mut self, stream_hub: &mut StreamsHub) -> Result<()> {
        let hls_cfg = &self.cfg.hls;

        if let Some(hls_cfg_value) = hls_cfg {
            if !hls_cfg_value.enabled {
                return Ok(());
            }

            let event_producer = stream_hub.get_hub_event_sender();
            let cient_event_consumer = stream_hub.get_client_event_consumer();
            let mut hls_remuxer = HlsRemuxer::new(
                cient_event_consumer,
                event_producer,
                hls_cfg_value.need_record,
            );

            tokio::spawn(async move {
                if let Err(err) = hls_remuxer.run().await {
                    log::error!("rtmp event processor error: {}\n", err);
                }
            });

            let port = hls_cfg_value.port;

            tokio::spawn(async move {
                if let Err(err) = hls_server::run(port).await {
                    log::error!("hls server error: {}\n", err);
                }
            });
            stream_hub.set_hls_enabled(true);
        }

        Ok(())
    }
}
