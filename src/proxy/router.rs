use crate::proxy::hyper_helpers::{
    build_error_response, build_web3_response, build_wrong_argument_response, parse_json_request,
};
use crate::proxy::input_checker;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::{debug, trace, warn};
use web3::{transports, Transport, Web3};

// TODO: essentially all errors should be in the default web3 format
pub async fn route_request(
    web3: Web3<transports::Http>,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    trace!(
        "Got {} request for: {}",
        req.method().as_str(),
        req.uri().path()
    );

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => Ok(proxy_web3(web3, req).await),
        // This is required to support MetaMask connecting to the proxy
        (&Method::OPTIONS, "/") => Ok(proxy_options()),
        (&Method::GET, "/info") => Ok(get_info(req)),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 - not found"))
            .unwrap()),
    }
}

async fn proxy_web3(web3: Web3<transports::Http>, req: Request<Body>) -> Response<Body> {
    match parse_json_request(req).await {
        Ok(json) => {
            if !json["method"].is_string() {
                let (response, error) = build_wrong_argument_response("method", "string");
                log_rejected("unparseable method", error);
                return response;
            }

            let method = json["method"].as_str().unwrap();

            let request_id: String;

            if json["id"].is_string() {
                request_id = json["id"].as_str().unwrap().to_string();
            } else if json["id"].is_u64() {
                request_id = json["id"].as_u64().unwrap().to_string();
            } else {
                let (response, error) = build_wrong_argument_response("id", "number or string");
                log_rejected(method, error);
                return response;
            }

            match method {
                // eth_accounts lists the accounts registered on your node which is no one's business
                "eth_accounts" => {
                    log_rejected(method, "method not allowed".to_string());
                    return build_web3_response(request_id, serde_json::json!({
                        "code": jsonrpc_core::ErrorCode::MethodNotFound.code(),
                        "message": format!("the method {} does not exist/is not available", method),
                    }));
                }

                // eth_getLogs can put an unreasonably high load on your node by letting it search for events
                // since the genesis block or for a large range of blocks
                "eth_getLogs" => {
                    debug!(
                        "Sanity checking {} parameters: {}",
                        method,
                        json["params"].to_string()
                    );
                    let input_check =
                        input_checker::check_get_logs(web3.clone(), &json["params"]).await;

                    if input_check.is_some() {
                        let (response, error) = input_check.unwrap();
                        log_rejected(method, error);

                        return response;
                    }
                }
                _ => {}
            }

            let mut parameters: Vec<jsonrpc_core::Value> = Vec::new();

            if json["params"].is_array() {
                parameters = json["params"].as_array().unwrap().to_vec();
            } else {
                trace!("Not passing parameters to web3 provider, because none a non array type was provided");
            }

            debug!(
                "Sending {} request to web3 provider with arguments: {:?}",
                method, parameters
            );

            let web3_response_result = web3.transport().execute(method, parameters).await;

            match web3_response_result {
                Ok(web3_response) => {
                    trace!(
                        "Got response from web3 provider: {}",
                        web3_response.to_string()
                    );
                    build_web3_response(request_id, web3_response)
                }
                Err(error) => {
                    warn!("Request to web3 provider failed: {}", error.to_string());

                    match error.clone() {
                        web3::Error::Rpc(rpc_error) => {
                            return build_web3_response(
                                request_id,
                                serde_json::json!({
                                    "code": rpc_error.code.code(),
                                    "message": rpc_error.message,
                                    "data": rpc_error.data,
                                }),
                            );
                        }
                        _ => {}
                    }

                    build_error_response(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()).0
                }
            }
        }
        Err(response) => {
            debug!("Could not parse JSON: {}", response.1);
            response.0
        }
    }
}

fn proxy_options() -> Response<Body> {
    Response::builder()
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .body(Body::empty())
        .unwrap()
}

fn get_info(_req: Request<Body>) -> Response<Body> {
    Response::new(Body::from("This web3 provider is protected by web3-proxy"))
}

fn log_rejected(method: &str, error: String) {
    warn!("Rejected request to {}: {}", method, error);
}
