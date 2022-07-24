use std::sync::Arc;
use {
    super::hls_event_manager::HlsEventManager,
    super::hls_request_handler::MakeHlsHandler,
    hyper::{
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server, StatusCode,
    },
};

pub async fn run(port: u32, hls_event_manager: HlsEventManager) -> Result<(), hyper::Error> {
    let listen_address = format!("0.0.0.0:{}", port);
    let sock_addr = listen_address.parse().unwrap();

    // let new_service = make_service_fn(move |_| {
    //     async move {
    //         Ok::<_, GenericError>(service_fn(move |req| {
    //             handle_connection(req, Arc::clone(&t)).await
    //         }))
    //     }
    // });

    let t = Arc::clone(&hls_event_manager.stream_to_producer);

    let server = Server::bind(&sock_addr).serve(MakeHlsHandler { stp_map: t });
    log::info!("Hls server listening on http://{}", sock_addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
