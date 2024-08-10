use {
    super::errors::RelayError,
    crate::{
        rtsp_transport::ProtocolType,
        session::{client_session::RtspClientSession, define::ClientSessionType},
    },
    std::{
        collections::HashMap,
        sync::{atomic::AtomicBool, Arc},
    },
    streamhub::{
        define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
        errors::{StreamHubError, StreamHubErrorValue},
        stream::StreamIdentifier,
    },
    tokio::sync::Mutex,
};

pub struct RtspPullClientManager {
    clients: HashMap<String, Arc<AtomicBool>>,
    client_event_consumer: BroadcastEventReceiver,
    channel_event_producer: StreamHubEventSender,
}

impl RtspPullClientManager {
    pub fn new(consumer: BroadcastEventReceiver, producer: StreamHubEventSender) -> Self {
        Self {
            clients: HashMap::new(),
            client_event_consumer: consumer,
            channel_event_producer: producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), RelayError> {
        log::info!("push client run...");

        loop {
            let val = self.client_event_consumer.recv().await?;

            match val {
                BroadcastEvent::Subscribe {
                    id,
                    identifier,
                    server_address,
                    result_sender,
                } => {
                    let sender = result_sender.unwrap();

                    if let StreamIdentifier::Rtsp { stream_path } = identifier {
                        if let Some(server_address) = server_address {
                            log::info!("publish stream_path: {}", stream_path.clone());

                            /* judge if the server address / stream path exists */
                            if self.clients.get_mut(&id).is_some() {
                                log::warn!("the client session with id:{} exists", id);

                                let err = Err(StreamHubError {
                                    value: StreamHubErrorValue::RtspClientSessionError(format!(
                                        "stream {} exists.",
                                        stream_path
                                    )),
                                });
                                if let Err(send_err) = sender.send(err).await {
                                    log::error!("sender error: {}", send_err);
                                }
                                continue;
                            }

                            /* new and run a client, save the client handler for exit */
                            match RtspClientSession::new(
                                server_address.clone(),
                                stream_path.clone(),
                                ProtocolType::TCP,
                                self.channel_event_producer.clone(),
                                ClientSessionType::Pull,
                            )
                            .await
                            {
                                Ok(client_session) => {
                                    self.clients.insert(id, client_session.is_running.clone());
                                    let arc_client_session = Arc::new(Mutex::new(client_session));

                                    tokio::spawn(async move {
                                        if let Err(err) =
                                            arc_client_session.lock().await.run().await
                                        {
                                            log::error!(
                                                "client_session as push client run error: {}",
                                                err
                                            );

                                            //let err = Err(StreamHubError {
                                            //    value: StreamHubErrorValue::RtspClientSessionError(
                                            //        err.to_string(),
                                            //    ),
                                            //});
                                            //if let Err(send_err) = sender.send(err).await {
                                            //    log::error!("sender error: {}", send_err);
                                            //}
                                        }
                                    });

                                    if let Err(send_err) = sender.send(Ok(())).await {
                                        log::error!("sender error: {}", send_err);
                                    }
                                }
                                Err(err) => {
                                    log::error!("new client session err: {}", err);

                                    let err = Err(StreamHubError {
                                        value: StreamHubErrorValue::RtspClientSessionError(
                                            err.to_string(),
                                        ),
                                    });
                                    if let Err(send_err) = sender.send(err).await {
                                        log::error!("sender error: {}", send_err);
                                    }
                                    continue;
                                }
                            }
                        } else {
                            log::error!(
                                "The Rtsp subscribe parameters does not contain server address: {}",
                                stream_path
                            );

                            let err = Err(StreamHubError {
                                value: StreamHubErrorValue::RtspClientSessionError(String::from(
                                    "The Rtsp subscribe parameters does not contain server address",
                                )),
                            });
                            if let Err(send_err) = sender.send(err).await {
                                log::error!("sender error: {}", send_err);
                            }
                            continue;
                        }
                    }
                }

                BroadcastEvent::UnSubscribe { id, result_sender } => {
                    let sender = result_sender.unwrap();
                    /* judge if the server address / stream path exists */
                    if let Some(client) = self.clients.get_mut(&id) {
                        client.store(false, std::sync::atomic::Ordering::Release);
                        self.clients.remove(&id);
                    } else {
                        log::warn!("the client session with id:{} not exists", id);

                        let err = Err(StreamHubError {
                            value: StreamHubErrorValue::RtspClientSessionError(String::from(
                                "the client session not exists",
                            )),
                        });
                        if let Err(send_err) = sender.send(err).await {
                            log::error!("sender error: {}", send_err);
                        }
                    }
                }

                _ => {
                    log::info!("push client receive other events");
                }
            }
        }
    }
}
