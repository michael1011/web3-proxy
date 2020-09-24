use hyper::{StatusCode, Body, Response};
use crate::proxy::hyper_helpers::{build_wrong_argument_response, build_error_response};
use web3::{Web3, transports};
use web3::types::{U64};
use std::ops::Add;

static MAX_BLOCK_TO_QUERY: u64 = 1000;

pub async fn check_get_logs(web3: Web3<transports::Http>, params: &serde_json::Value) -> Option<(Response<Body>, String)> {
    if !params.is_array() {
        return Option::from(build_wrong_argument_response("params", "array"));
    }

    let params_array = params.as_array().unwrap();

    if !params_array[0].is_object() {
        return Option::from(build_error_response(StatusCode::BAD_REQUEST, &"\"params\" is not formatted correctly".to_string()));
    }

    let params = params_array[0].as_object().unwrap();

    // If there is a block hash, the web3 provider will only search for the logs in that block
    if params.contains_key("blockHash") {
        return Option::None;
    }

    let latest_block_number_result = web3.eth().block_number().await;

    match latest_block_number_result {
        Ok(latest_block_number) => {
            let from_block = parse_json_block_number(params.get("fromBlock"), latest_block_number).as_u64();
            let to_block = parse_json_block_number(params.get("toBlock"), latest_block_number).as_u64();

            if to_block - from_block > MAX_BLOCK_TO_QUERY {
                return Option::from(build_error_response(StatusCode::BAD_REQUEST, &format!("only up to {} blocks can be queried", MAX_BLOCK_TO_QUERY)));
            }

            Option::None
        }
        Err(error) => Option::from(build_error_response(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()))
    }
}

fn parse_json_block_number(value: Option<&serde_json::Value>, latest_block: U64) -> U64 {
    if value.is_some() {
        let unwrapped = value.unwrap();

        if unwrapped.is_u64() {
            return U64::from(unwrapped.as_u64().unwrap())
        } else if unwrapped.is_string() {
            match unwrapped.as_str().unwrap() {
                "earliest" | "pending" => return latest_block.add(1),
                _ => {}
            }
        }
    }

    latest_block
}
