use crate::proxy::input_checker;
use crate::proxy::hyper_helpers::{parse_json_request, build_wrong_argument_response, build_error_response, build_web3_response};
use log::{warn, debug, trace, error};
use web3::{Web3, transports, Transport};
use hyper::{Request, Body, Response, StatusCode, Method};

// TODO: rpc error should return data and 200 status code
// TODO: essentially all errors should be in the default web3 format
pub async fn route_request(web3: Web3<transports::Http>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    trace!("Got {} request for: {}", req.method().as_str(), req.uri().path());

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/") => Ok(proxy_web3(web3, req).await),
        (&Method::GET, "/info") => Ok(get_info(req)),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 - not found"))
            .unwrap())
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
                    debug!("Sanity checking {} parameters: {}", method, json["params"].to_string());
                    let input_check = input_checker::check_get_logs(web3.clone(), &json["params"]).await;

                    if input_check.is_some() {
                        let (response, error) = input_check.unwrap();
                        log_rejected(method, error);

                        return response;
                    }
                },
                _ => {},
            }

            let mut parameters: Vec<jsonrpc_core::Value> = Vec::new();

            if json["params"].is_array() {
                parameters = json["params"].as_array().unwrap().to_vec();
            } else {
                trace!("Not passing parameters to web3 provider, because none a non array type was provided");
            }

            debug!("Sending {} request to web3 provider with arguments: {:?}", method, parameters);

            let web3_response_result = web3.transport().execute(method, parameters).await;

            match web3_response_result {
                Ok(web3_response) => {
                    debug!("Got response from web3 provider: {}", web3_response.to_string());
                    build_web3_response(request_id, web3_response)
                }
                Err(error) => {
                    let error_response = build_error_response(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string());
                    error!("Request to web3 provider failed: {}", error_response.1);

                    error_response.0
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
