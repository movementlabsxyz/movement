use crate::{
    JsonRpcRequest,
    ToJsonRpc,
};
use actix_web::web;
use serde_json::json;
use std::collections::HashMap;
use actix_router::{Path, ResourceDef};

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

pub struct WebArgsTuple(pub web::Path<String>, pub web::Json<serde_json::Value>, pub web::Query<serde_json::Map<String, serde_json::Value>>);

impl From<WebArgsTuple> for WebArgs {
    fn from(tuple: WebArgsTuple) -> Self {
        WebArgs::new(tuple.0, tuple.1, tuple.2)
    }
}

impl ToJsonRpc<WebArgs> for ActixWeb {

    async fn request_to_method(&self, request : &WebArgs) -> Result<String, anyhow::Error> {
        Ok(request.path.clone().replace("/", "."))
    }

    async fn to_json_rpc(&self, request: WebArgs) -> Result<JsonRpcRequest, anyhow::Error> {
        let path_as_method = self.request_to_method(&request).await?;
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


pub mod test_web_args {
    use super::{
        ActixWeb,
        WebArgs,
        ToJsonRpc,
    };
    use actix_web::web;
    use serde_json::json;

    #[tokio::test]
    async fn test_to_json_rpc() -> Result<(), anyhow::Error>{

        let actix_web = ActixWeb;
        let request = WebArgs::new(
            web::Path::from("test".to_string()), 
            web::Json(json!({"test": "test"})),
            web::Query::from_query("test=test").unwrap()
        );   
        let rpc_request = actix_web.to_json_rpc(request).await?;
        assert_eq!(rpc_request.method, "test");
        assert_eq!(rpc_request.params, json!({"body": {"test": "test"}, "query": {"test": "test"}}));

        Ok(())

    }

}

#[derive(Clone)]
pub struct PathExtractor {
    pub actix_web: ActixWeb,
    pub matching : Vec<String>,
}

impl PathExtractor {
    pub fn new() -> Self {

        PathExtractor {
            actix_web: ActixWeb,
            matching: vec![],
        }
    }

    pub fn matching(&mut self, pattern: &str) -> Result<(), anyhow::Error> {
        self.matching.push(pattern.to_string());
        Ok(())
    }

    pub fn match_and_extract(&self, original_path: &str) -> Result<(HashMap<String, String>, String), anyhow::Error> {
        for pattern in &self.matching {

            let resource = ResourceDef::new(pattern.as_str());
            let mut path = Path::new(original_path);
            let matches = resource.capture_match_info(&mut path);

            if matches {
                let mut path_params = HashMap::new();
                let mut new_path = original_path.to_string();
                
                for (name, value) in path.iter() {
                  
                    path_params.insert(name.to_string(), value.to_string());
                    new_path = new_path.replace(&value, name);

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

pub mod test_path_extractor {

    use super::{
        PathExtractor,
        WebArgs,
        ToJsonRpc,
        ActixWeb,
    };
    use actix_web::web;
    use serde_json::json;

    #[test]
    fn test_match_and_extract() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathExtractor::new();
        path_extractor.matching(r"test/{id}")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id");

        Ok(())
    }

    #[test]
    fn test_match_multiple_patterns() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathExtractor::new();
        path_extractor.matching(r"test/{id}")?;
        path_extractor.matching(r"test/{id}/test")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1/test")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id/test");

        Ok(())
    }

    #[test]
    fn test_multiple_patterns_matches_first() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathExtractor::new();
        path_extractor.matching(r"test/{id}")?;
        path_extractor.matching(r"test/{id}/test")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id");

        Ok(())
    }

    #[test]
    fn test_multiple_segments() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathExtractor::new();
        path_extractor.matching(r"test/{id}/test/{id2}")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1/test/2")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(path_params.get("id2").unwrap(), "2");
        assert_eq!(new_path, "test/id/test/id2");

        Ok(())
    }

}
