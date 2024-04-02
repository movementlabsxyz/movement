use warp::Filter;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::env;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "warp=info");
    env_logger::init();

    // Define the route
    let proxy_route = warp::path!("proxy" / ..)
        .and(warp::any().map(move || {
            // Define your JSON-RPC backend URL here
            env::var("JSON_RPC_BACKEND").unwrap_or_else(|_| "http://localhost:8080".into())
        }))
        .and(warp::body::json())
        .and_then(handle_proxy_request);

    warp::serve(proxy_route)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

async fn handle_proxy_request(backend_url: String, body: serde_json::Value) -> Result<impl warp::Reply, Infallible> {
    let client = reqwest::Client::new();
    // Assuming the JSON-RPC backend expects a POST request
    let res = client.post(&backend_url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    Ok(warp::reply::json(&res))
}

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: serde_json::Value,
}
