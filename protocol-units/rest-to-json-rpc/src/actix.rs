use crate::{
    JsonRpcRequestStandard, 
    JsonRpcRequest,
    ToJsonRpc,
    Forwarder,
    Middleware,
    Proxy
};
use actix_web::{web, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder};
use serde_json::json;
use std::collections::HashMap;
use actix_router::{Path, ResourceDef};
use futures_util::{
    future::BoxFuture,
    FutureExt,
};

#[derive(Clone)]
pub struct ActixWeb;

impl FromRequest for JsonRpcRequestStandard {

    type Error = actix_web::Error;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    
    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {

        let headers = req.headers().clone();
        let path = req.path().to_owned();
        let match_info = req.match_info().clone();
        let query_string = req.query_string().to_owned();
        let bytes_future = web::Bytes::from_request(req, payload);

        async move {
            // Extracting the body as bytes
            let body_bytes = bytes_future.await.map_err(actix_web::Error::from)?;

            // Handling query parameters (assuming they are key=value pairs)
            let query_params = serde_urlencoded::from_str::<HashMap<String, String>>(&query_string)
                .map_err(actix_web::Error::from)?;

            let rpc_request = JsonRpcRequestStandard {
                path,
                http_headers: headers.iter().map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string())).collect(),
                path_params: match_info.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                body: body_bytes.to_vec().into(),
                query_params,
            };
    
            Ok(rpc_request)
        }
        .boxed()
    }

}

impl ToJsonRpc<HttpRequest> for ActixWeb {

    async fn request_to_method(&self, request : &HttpRequest) -> Result<String, anyhow::Error> {
        Ok(request.path().replace("/", ".").to_string())
    }

    async fn to_json_rpc_standard(&self, request: HttpRequest) -> Result<JsonRpcRequestStandard, anyhow::Error> {
        let path_as_method = self.request_to_method(&request).await?;
        let body = web::Json(serde_json::from_slice(&request.body().wait().unwrap()).unwrap());
        let query = web::Query::from_query(request.query_string()).unwrap();

        let rpc_request = JsonRpcRequestStandard {
            path : request.path().to_string(),
            http_headers: request.headers().iter().map(|(k, v)| (k.as_str().to_string(), v.to_str()?.to_string())).collect(),
            path_params: request.match_info().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body: body.into_inner(),
            query_params: query.into_inner(),
        };

        Ok(rpc_request)
    }


}



pub mod test_web_args {
    use super::{
        ActixWeb,
        ToJsonRpc,
    };
    use actix_web::{web, HttpRequest};
    use serde_json::json;

    #[tokio::test]
    async fn test_to_json_rpc() -> Result<(), anyhow::Error>{

        // need to figure out a new way to test this as HttpRequest can't be created directly. Maybe we'll use an extract struct.

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

    pub fn extract(&self, mut request: JsonRpcRequestStandard) -> Result<HttpRequest, anyhow::Error> {
        
        let (path_params, new_path) = self.match_and_extract(&request.path)?;

        request.path = new_path;
        request.path_params = path_params;

        Ok(request)

    }

}

#[async_trait::async_trait]
impl Middleware<JsonRpcRequestStandard> for PathExtractor {

    async fn apply(&self, request: JsonRpcRequestStandard) -> Result<JsonRpcRequestStandard, anyhow::Error> {
        self.extract(request)
    }

}

pub mod test_path_extractor {

    use super::{
        PathExtractor,
        JsonRpcRequestStandard,
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


pub struct Actix {
    pub forwarders : Box<dyn Forwarder<String> + Send + Sync>,
    pub middleware : Vec<Box<dyn Middleware<JsonRpcRequestStandard> + Send + Sync>>,
    pub actix_web : ActixWeb,
}

impl Actix {

    pub fn new() -> Self {
        Actix {
            forwarders: vec![],
            middleware: vec![],
            actix_web: ActixWeb,
        }
    }

    pub async fn add_middleware(&mut self, middleware: Box<dyn Middleware<JsonRpcRequestStandard> + Send + Sync>) {
        self.middleware.push(middleware);
    }

    pub async fn handle_request(&self, args: JsonRpcRequestStandard) -> impl Responder  {
        let mut request = args;
        for middleware in &self.middleware {
            request = middleware.apply(request).await?;
        }

        // send to all the forwarders in parallel
        let mut futures = vec![];
        for forwarder in &self.forwarders {
            futures.push(forwarder.forward(args.into()));
        }

        Ok(request)
    }

}

#[async_trait::async_trait]
impl Proxy<String> for Actix {

    async fn set_forwarder(&mut self, forwarder : Box<dyn Forwarder<String> + Send + Sync>) -> Result<(), anyhow::Error> {

        self.forwarders.push(forwarder);

        Ok(())
    }

    async fn serve(&self) -> Result<(), anyhow::Error> {



        Ok(())
    }

}