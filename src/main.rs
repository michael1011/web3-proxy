mod proxy;
mod service;

use hyper::Server;
use log::{debug, error, info, trace};
use simple_logger::SimpleLogger;
use structopt::StructOpt;
use web3::{transports, Web3};

#[derive(StructOpt, Debug)]
#[structopt(name = "web3-proxy")]
struct Args {
    #[structopt(short, long, default_value = "3000")]
    port: u16,

    #[structopt(short, long, default_value = "http://127.0.0.1:8545")]
    web3_endpoint: String,
}

// TODO: websocket support for clients
// TODO: use IPC for connection to web3 provider
#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("web3_proxy", log::LevelFilter::Trace)
        .with_module_level("web3_proxy::proxy::router", log::LevelFilter::Trace)
        .init()
        .unwrap();

    let args = Args::from_args();

    info!("Starting web3-proxy");

    info!("Using web3 HTTP provider: {}", args.web3_endpoint);

    let transport =
        transports::Http::new(&args.web3_endpoint).expect("Could not get HTTP provider");
    let web3 = Web3::new(transport);

    trace!("Sanity checking web3 HTTP provider");

    match web3.clone().web3().client_version().await {
        Ok(client_version) => {
            debug!("Connected to web3 provider: {}", client_version);
        }
        Err(error) => {
            error!("Could not connect to web3 provider: {}", error);
            // EX_UNAVAILABLE
            // https://www.freebsd.org/cgi/man.cgi?query=sysexits&apropos=0&sektion=0&manpath=FreeBSD+11.2-stable&arch=default&format=html
            std::process::exit(69);
        }
    }

    let router = proxy::Router { web3 };

    let addr = ([0, 0, 0, 0], args.port).into();
    let server = Server::bind(&addr).serve(service::MakeSvc { router });

    info!("Listening on: {}", addr);

    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }
}
