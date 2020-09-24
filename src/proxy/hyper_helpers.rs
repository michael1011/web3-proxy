use hyper::{Request, Body, Response, StatusCode, header};

pub async fn parse_json_request(req: Request<Body>) -> Result<serde_json::Value, (Response<Body>, String)> {
    match hyper::body::to_bytes(req.into_body()).await {
        Ok(body_bytes) => {
            match String::from_utf8(body_bytes.to_vec()) {
                Ok(body_string) => {
                    match serde_json::from_slice::<serde_json::Value>(body_string.as_bytes()) {
                        Ok(json) => Ok(json),
                        Err(error) => Err(build_error_response(StatusCode::BAD_REQUEST, &error.to_string()))
                    }
                },
                Err(error) => Err(build_error_response(StatusCode::BAD_REQUEST, &error.to_string()))
            }
        },
        Err(error) => Err(build_error_response(StatusCode::BAD_REQUEST, &error.to_string())),
    }
}

pub fn build_web3_response(request_id: u64, data: serde_json::Value) -> Response<Body> {
    let response_body = serde_json::json!({
        "jsonrpc": "2.0",
        "result": data,
        "id": request_id,
    }).to_string();

    build_json_response(response_body)
}

pub fn build_json_response(data: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(data))
        .unwrap()
}

pub fn build_wrong_argument_response(argument: &str, expected_type: &str) -> (Response<Body>, String) {
    build_error_response(StatusCode::BAD_REQUEST, &format!("\"{}\" is not a {}", argument, expected_type))
}

pub fn build_error_response(status_code: StatusCode, error: &String) -> (Response<Body>, String) {
    (
        Response::builder()
        .status(status_code)
        .body(Body::from(error.to_string()))
        .unwrap(),
        error.to_string(),
    )
}
