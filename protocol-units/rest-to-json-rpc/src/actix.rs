use crate::{
    JsonRpcRequest,
    ToJsonRpc,
};
use actix_web::web;
use serde_json::json;
use regex::Regex;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ActixWeb;

pub struct WebArgs {
    pub path : web::Path<String>, 
    pub body : web::Json<serde_json::Value>,
    pub query_params : web::Query<serde_json::Map<String, serde_json::Value>>,
    pub path_params : HashMap<String, String>,
}

impl WebArgs {
    pub fn new(path: web::Path<String>, body: web::Json<serde_json::Value>, query_params: web::Query<serde_json::Map<String, serde_json::Value>>) -> Self {
        WebArgs {
            path,
            body,
            query_params,
            path_params: HashMap::new(),
        }
    }
}

impl ToJsonRpc<WebArgs> for ActixWeb {

    fn request_to_method(&self, request : &WebArgs) -> Result<String, anyhow::Error> {
        Ok(request.path.clone().replace("/", "."))
    }

    fn to_json_rpc(&self, request: WebArgs) -> Result<JsonRpcRequest, anyhow::Error> {
        let path_as_method = self.request_to_method(&request)?;
        let params = json!({
            "body": request.body.into_inner(),
            "query": request.path.into_inner(),
        });

        let rpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: path_as_method,
            params,
            id: json!(1), // You can customize this as needed
        };

        Ok(rpc_request)
    }
}


pub mod test {
    use super::{
        ActixWeb,
        WebArgs,
        ToJsonRpc,
    };
    use actix_web::web;
    use serde_json::json;

    #[test]
    fn test_to_json_rpc() {

        let actix_web = ActixWeb;
        let request = WebArgs::new(
            web::Path::from("test".to_string()), 
            web::Json(json!({"test": "test"})),
            web::Query::from_query("test=test").unwrap()
        );   
        let rpc_request = actix_web.to_json_rpc(request).unwrap();
        assert_eq!(rpc_request.method, "test");
        assert_eq!(rpc_request.params, json!({"body": {"test": "test"}, "query": {"test": "test"}}));

    }

}

#[derive(Clone)]
pub struct WebArgsExtractor {
    pub actix_web: ActixWeb,
    pub matching : Vec<Regex>,
}

impl WebArgsExtractor {
    pub fn new() -> Self {

        WebArgsExtractor {
            actix_web: ActixWeb,
            matching: vec![],
        }
    }

    pub fn match_and_extract(&self, original_path: &str) -> Result<(HashMap<String, String>, String), anyhow::Error> {
        for pattern in &self.matching {
            if let Some(caps) = pattern.captures(original_path) {
                let mut path_params = HashMap::new();
                let mut new_path = original_path.to_string();
                
                for name in pattern.capture_names().flatten() {
                    if let Some(matched) = caps.name(name) {
                        path_params.insert(name.to_string(), matched.as_str().to_string());

                        // Replace the matched segment with nothing in the new path
                        new_path = new_path.replacen(matched.as_str(), "", 1);
                    }
                }

                // Cleanup any residual slashes from the path
                new_path = new_path.trim_matches('/').replace("//", "/");
                return Ok((path_params, new_path));
            }
        }
        
        // If no patterns matched, return the original path without modifications
        Ok((HashMap::new(), original_path.to_string()))
    }

    pub fn extract(&self, mut request: WebArgs) -> Result<WebArgs, anyhow::Error> {
        
        let (path_params, new_path) = self.match_and_extract(&request.path)?;

        request.path = web::Path::from(new_path);
        request.path_params = path_params;

        Ok(request)

    }

}