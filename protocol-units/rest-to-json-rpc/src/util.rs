
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
    CONNECT,
    TRACE,
}

impl From<&str> for HttpMethod {
    fn from(s : &str) -> Self {
        match s {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            "DELETE" => HttpMethod::DELETE,
            "PATCH" => HttpMethod::PATCH,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            "CONNECT" => HttpMethod::CONNECT,
            "TRACE" => HttpMethod::TRACE,
            _ => HttpMethod::GET
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequestStandard {
    pub path : String,
    pub http_method : HttpMethod,
    pub http_headers : HashMap<String, String>,
    pub path_params : HashMap<String, String>,
    pub body : serde_json::Value,
    pub query_params : serde_json::Map<String, serde_json::Value>
}

impl JsonRpcRequestStandard {
    pub fn new() -> Self {
        JsonRpcRequestStandard {
            path: "".to_string(),
            http_method: HttpMethod::GET,
            http_headers: HashMap::new(),
            path_params: HashMap::new(),
            body: serde_json::Value::Null,
            query_params: serde_json::Map::new()
        }
    }

    pub fn set_path_param(&mut self, key : String, value : String) {
        self.path_params.insert(key, value);
    }

    pub fn set_http_header(&mut self, key : String, value : String) {
        self.http_headers.insert(key, value);
    }

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

impl From<JsonRpcRequestStandard> for JsonRpcRequest {
    fn from(standard : JsonRpcRequestStandard) -> Self {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: standard.path.replace("/", "."), // ? This is a naive way to convert a path to a method name
            params: serde_json::to_value(standard).unwrap(),
            id: json!(1), // You can customize this as needed
        }
    }
}


/*impl From<JsonRpcRequest> for JsonRpcRequestStandard {
    fn from(request : JsonRpcRequest) -> Self {
        let params = request.params.as_object().unwrap();
        JsonRpcRequestStandard {
            path: request.method.replace(".", "/"),
            http_headers: params.get("http_headers").unwrap().as_object().unwrap().clone(),
            path_params: params.get("path_params").unwrap().as_object().unwrap().clone(),
            body: params.get("body").unwrap().clone(),
            query_params: params.get("query").unwrap().clone(),
        }
    }
}*/


#[async_trait::async_trait] // if we don't have this we can't use Box<dyn Forwarder>
pub trait Forwarder<T> {

    async fn forward(&self, json_rpc_request : JsonRpcRequest) -> Result<T, anyhow::Error>;

}


#[async_trait::async_trait] // if we don't have this we can't use Box<dyn Forwarder>
pub trait Middleware<T>  {
    async fn apply(&self, request : T) -> Result<T, anyhow::Error>;
}



pub trait Proxy {

    async fn serve(&self) -> Result<(), anyhow::Error>;

}