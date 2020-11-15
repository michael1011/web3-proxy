mod proxy;
mod service;

use hyper::Server;
use log::{debug, error, info};
use simple_logger::SimpleLogger;
use structopt::StructOpt;
use web3::{transports, Web3};

/// This is a proxy for the HTTP server of your Ethereum node
///
/// It will prevent malicious parties from abusing your generous service to the Ethereum world
/// by blocking requests that would put an unreasonably high load on your node
#[derive(StructOpt, Debug)]
#[structopt(name = "web3-proxy")]
struct Args {
    /// Verbosity of the daemon (No flag - INFO, -v - DEBUG, -vv - TRACE)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    /// The port to which the proxy should listen
    #[structopt(short, long, default_value = "3000")]
    port: u16,

    /// The HTTP endpoint that should be proxied
    #[structopt(short, long, default_value = "http://127.0.0.1:8545")]
    web3_endpoint: String,
}

// TODO: websocket support for clients
// TODO: use IPC for connection to web3 provider
#[tokio::main]
async fn main() {
    let args = Args::from_args();

    let log_level = parse_log_level(args.verbose);

    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("web3_proxy", log_level)
        .with_module_level("web3_proxy::proxy::router", log_level)
        .init()
        .unwrap();

    info!("Parsed CLI arguments: {:#?}", args);

    debug!("Starting web3-proxy");

    info!("Using web3 HTTP provider: {}", args.web3_endpoint);

    let transport =
        transports::Http::new(&args.web3_endpoint).expect("Could not get HTTP provider");
    let web3 = Web3::new(transport);

    debug!("Sanity checking web3 HTTP provider");

    match web3.clone().web3().client_version().await {
        Ok(client_version) => {
            info!("Connected to web3 provider: {}", client_version);
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

fn parse_log_level(level: u8) -> log::LevelFilter {
    match level {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        2 => log::LevelFilter::Trace,
        _ => {
            println!("Could not parse log level: {}", level);
            // EX_CONFIG
            // https://www.freebsd.org/cgi/man.cgi?query=sysexits&apropos=0&sektion=0&manpath=FreeBSD+11.2-stable&arch=default&format=html
            std::process::exit(78);
        }
    }
}
