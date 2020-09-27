use crate::proxy::hyper_helpers::{
    build_error_response, build_web3_response, build_wrong_argument_response, parse_json_request,
};
use crate::proxy::input_checker;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::{debug, error, trace, warn};
use web3::{transports, Transport, Web3};

// TODO: essentially all errors should be in the default web3 format
pub async fn route_request(
    web3: Web3<transports::Http>,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    debug!(
        "Got {} request for: {}",
        req.method().as_str(),
        req.uri().path()
    );

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => Ok(proxy_web3(web3, req).await),
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

            if !json["id"].is_u64() {
                let (response, error) = build_wrong_argument_response("method", "number");
                log_rejected(method, error);
                return response;
            }

            let request_id = json["id"].as_u64().unwrap();

            match method {
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
                    debug!(
                        "Got response from web3 provider: {}",
                        web3_response.to_string()
                    );
                    build_web3_response(request_id, web3_response)
                }
                Err(error) => {
                    error!("Request to web3 provider failed: {}", error.to_string());

                    match error.clone() {
                        web3::Error::Rpc(rpc_error) => {
                            return build_web3_response(request_id, serde_json::json!({
                                "code": rpc_error.code.code(),
                                "message": rpc_error.message,
                                "data": rpc_error.data,
                            }));
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

fn get_info(_req: Request<Body>) -> Response<Body> {
    Response::new(Body::from("This web3 provider is protected by web3-proxy"))
}

fn log_rejected(method: &str, error: String) {
    warn!("Rejected request to {}: {}", method, error);
}
