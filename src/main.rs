mod proxy;

use std::convert::Infallible;
use log::{info, error};
use simple_logger::{SimpleLogger};
use hyper::{Server, Response, Body, Request};
use hyper::service::{make_service_fn, service_fn};
use web3::{transports, Web3};
use crate::proxy::RouterTrait;

static WEB3_PROVIDER: &str = "http://10.0.1.18:8545";

async fn handle_request(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let transport = transports::Http::new(WEB3_PROVIDER).expect("Could not get HTTP provider");
    let web3 = Web3::new(transport);
    let response = proxy::Router { web3 }.route(request).await;

    response
}

// TODO: websocket support for clients
// TODO: use IPC for connection to web3 provider
#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("web3_proxy", log::LevelFilter::Trace)
        .with_module_level("web3_proxy::proxy::router", log::LevelFilter::Trace)
        .init().unwrap();

    info!("Starting web3-proxy");

    info!("Using web3 HTTP provider: {}", WEB3_PROVIDER);

    let make_service = make_service_fn(|_conn| {
        async { Ok::<_, Infallible>(service_fn(handle_request))}
    });

    let addr = ([0, 0, 0, 0], 3000).into();
    let server = Server::bind(&addr).serve(make_service);

    info!("Listening on: {}", addr);

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }
}
