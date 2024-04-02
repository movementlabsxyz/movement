use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: serde_json::Value,
}


pub trait ToJsonRpc<T> {

    // ? This currently does not need to be part of the trait. It's just this way for organization.
    /// Converts a request to a method name
    fn path_to_method(&self, request : &T) -> Result<String, anyhow::Error>;

    /// Converts a request to a JsonRpcRequest
    fn to_json_rpc(&self, request : T) -> Result<JsonRpcRequest, anyhow::Error>;

}

pub mod actix_web {
    use super::{
        JsonRpcRequest,
        ToJsonRpc,
    };
    use actix_web::web;
    use serde_json::json;


    pub struct ActixWeb;

    pub struct WebArgs(web::Path<String>, web::Json<serde_json::Value>, web::Query<serde_json::Map<String, serde_json::Value>>);

    impl ToJsonRpc<WebArgs> for ActixWeb {

        fn path_to_method(&self, path: &WebArgs) -> Result<String, anyhow::Error> {
            Ok(path.0.clone().replace("/", "."))
        }

        fn to_json_rpc(&self, request: WebArgs) -> Result<JsonRpcRequest, anyhow::Error> {
            let path_as_method = self.path_to_method(&request)?;
            let params = json!({
                "body": request.1.into_inner(),
                "query": request.2.into_inner(),
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
            let request = WebArgs(
                web::Path::from("test".to_string()), 
                web::Json(json!({"test": "test"})),
                web::Query::from_query("test=test").unwrap()
        );   
            let rpc_request = actix_web.to_json_rpc(request).unwrap();
            assert_eq!(rpc_request.method, "test");
            assert_eq!(rpc_request.params, json!({"body": {"test": "test"}, "query": {"test": "test"}}));

        }

    }

}