use crate::{
    Forwarder, JsonRpcRequestStandard, Middleware, Proxy, HttpMethod
};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use actix_router::{Path, ResourceDef};
use serde_json::Value;

#[derive(Clone)]
pub struct PathMatchAndExtract {
    pub matching : Vec<String>,
}

impl PathMatchAndExtract {
    pub fn new() -> Self {

        PathMatchAndExtract {
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

    pub fn extract(&self, mut request: JsonRpcRequestStandard) -> Result<JsonRpcRequestStandard, anyhow::Error> {
        
        let (path_params, new_path) = self.match_and_extract(&request.path)?;

        request.path = new_path;
        request.path_params = path_params;

        Ok(request)

    }

}

#[async_trait::async_trait]
impl Middleware<JsonRpcRequestStandard> for PathMatchAndExtract {

    async fn apply(&self, request: JsonRpcRequestStandard) -> Result<JsonRpcRequestStandard, anyhow::Error> {
        self.extract(request)
    }

}

pub mod test_path_extractor {

    use super::PathMatchAndExtract;

    #[test]
    fn test_match_and_extract() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathMatchAndExtract::new();
        path_extractor.matching(r"test/{id}")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id");

        Ok(())
    }

    #[test]
    fn test_match_multiple_patterns() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathMatchAndExtract::new();
        path_extractor.matching(r"test/{id}")?;
        path_extractor.matching(r"test/{id}/test")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1/test")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id/test");

        Ok(())
    }

    #[test]
    fn test_multiple_patterns_matches_first() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathMatchAndExtract::new();
        path_extractor.matching(r"test/{id}")?;
        path_extractor.matching(r"test/{id}/test")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(new_path, "test/id");

        Ok(())
    }

    #[test]
    fn test_multiple_segments() -> Result<(), anyhow::Error> {
        let mut path_extractor = PathMatchAndExtract::new();
        path_extractor.matching(r"test/{id}/test/{id2}")?;
        let (path_params, new_path) = path_extractor.match_and_extract("test/1/test/2")?;
        assert_eq!(path_params.get("id").unwrap(), "1");
        assert_eq!(path_params.get("id2").unwrap(), "2");
        assert_eq!(new_path, "test/id/test/id2");

        Ok(())
    }

}

use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Actix {
    pub forwarder : Arc<Box<dyn Forwarder<reqwest::Response> + Send + Sync>>,
    pub middleware : Arc<RwLock<Vec<Box<dyn Middleware<JsonRpcRequestStandard> + Send + Sync>>>>,
}

impl Actix {

    pub fn new(forwarder : Box<dyn Forwarder<reqwest::Response> + Send + Sync>) -> Self {
        Actix {
            forwarder : Arc::new(forwarder),
            middleware: Arc::new(
                RwLock::new(
                    vec![]
                )
            ),
        }
    }

    pub fn forwarder(&mut self, forwarder : Box<dyn Forwarder<reqwest::Response> + Send + Sync>) {
        self.forwarder = Arc::new(forwarder);
    }

    pub fn middleware(&mut self, middleware: Box<dyn Middleware<JsonRpcRequestStandard> + Send + Sync>) {
        let mut middlewares = self.middleware.write().unwrap();
        middlewares.push(middleware);
    }

    pub async fn handle_request(
        &self,
        req: HttpRequest, // Include HttpRequest to access headers
        info: web::Path<String>, 
        body: web::Json<Value>, 
        query: web::Query<serde_json::Map<String, Value>>
    ) -> Result<impl Responder, anyhow::Error>  {
        
        let http_method = HttpMethod::from(req.method().as_str());

        let mut http_headers = HashMap::new();
        for (key, value) in req.headers().iter() {
            http_headers.insert(key.to_string(), value.to_str()?.to_string());
        }

        let mut standard_request = JsonRpcRequestStandard {
            http_headers: http_headers,
            http_method,
            path: info.into_inner(),
            body: body.into_inner(),
            query_params: query.into_inner(),
            path_params: HashMap::new(),
        };

        let middlewares = self.middleware.read().map_err(
            |e| anyhow::anyhow!("Error reading middlewares: {:?}", e)
        )?;
        for middleware in middlewares.iter() {
            standard_request = middleware.apply(standard_request).await?;
        }

        let response = self.forwarder.forward(standard_request.into()).await?;

        // extract the headers from the response and send the body along as text
        let mut response_builder = HttpResponse::Ok();
        for (key, value) in response.headers().iter() {
            response_builder.append_header(
                (key.as_str(), value.to_str()?)
            );
        }

        Ok(response_builder.body(response.text().await?))
        
    }

}

#[async_trait::async_trait]
impl Proxy for Actix {


    async fn serve(self) -> Result<(), anyhow::Error> {

        HttpServer::new(move || {
            App::new()
                .app_data(self.clone())
                .route("/{info:.*}", web::post().to(
                    |req, info, body, query, actix: web::Data<Actix>| {
                        // Use actix instance here
                        async move {
                            actix.handle_request(req, info, body, query).await.map_err(
                                |e| actix_web::error::ErrorInternalServerError(e)
                            )
                        }
                    }
                ))
        });

        Ok(())
    }

}

impl Actix {

    pub fn try_reqwest_from_env() -> Result<Self, anyhow::Error> {
        let url = std::env::var("PROXY_URL")?;
        let forwarder = Box::new(crate::reqwest::ReqwestForwarder {
            url: Arc::new(tokio::sync::RwLock::new(url)),
        });

        Ok(Actix::new(forwarder))
    }

}