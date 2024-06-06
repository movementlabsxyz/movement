use anyhow::Error;
use poem::listener::TcpListener;
use poem::{get, handler, web::Path, IntoResponse, Response, Route, Server};
use std::env;
use tracing::info;

pub struct MaptosRest {
    pub url: String,
    // More fields to be added here, log verboisty, etc.
}

impl MaptosRest {
    pub const MAPTOS_REST_ENV_VAR: &'static str = "MAPTOS_REST_URL";

    pub fn try_from_env() -> Result<Self, Error> {
        let maptos_rest =
            env::var(Self::MAPTOS_REST_ENV_VAR).unwrap_or_else(|_| "0.0.0.0:30832".to_string());
        Ok(Self { url: maptos_rest })
    }

    pub async fn run_service(&self) -> Result<(), Error> {
        info!("Starting maptos rest service at {}", self.url);
        let maptos_rest = create_routes();
        Server::new(TcpListener::bind(&self.url)).run(maptos_rest).await?;
        Ok(())
    }
}

#[handler]
async fn state_root_hash(Path(blockheight): Path<u64>) -> Response {
    // Use the blockheight value here
    format!("The state root hash for blockheight: {}", blockheight).into_response()
}

#[handler]
async fn health() -> Response {
    "OK".into_response()
}

pub fn create_routes() -> Route {
    Route::new()
        .at("/health", get(health))
        .at("/movement/v1/state-root-hash/:blockheight", get(state_root_hash))
}
