use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: serde_json::Value,
}

async fn handle_request(info: web::Path<String>, body: web::Json<serde_json::Value>, query: web::Query<serde_json::Map<String, serde_json::Value>>) -> impl Responder {
    let path_as_method = info.into_inner().replace("/", ".");
    let params = json!({
        "body": body.into_inner(),
        "query": query.into_inner(),
    });

    let rpc_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: path_as_method,
        params,
        id: json!(1), // You can customize this as needed
    };

    HttpResponse::Ok().json(rpc_request)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/{path:.*}", web::post().to(handle_request))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
