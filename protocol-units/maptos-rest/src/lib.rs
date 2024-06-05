use poem::{handler, IntoResponse, Response, Route};

pub struct MaptosConfig {
	pub url: String,
	// other fields for configs, log verbosite etc.
}

impl MaptosConfig {
	pub const MAPTOS_REST_ENV_VAR: &'static str = "MAPTOS_REST_URL";

	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let maptos_rest =
			std::env::var(Self::MAPTOS_REST_ENV_VAR).unwrap_or("0.0.0.0:30832".to_string());
		Ok(Self { url: maptos_rest })
	}
}

#[handler]
async fn health() -> Response {
	"healthy".into_response()
}

#[handler]
async fn state_root_hash() -> Response {
	"the_state_root_hash".into_response()
}

pub fn create_routes() -> Route {
	Route::new()
		.at("/health", poem::get(health))
		.at("/state_root_hash", poem::get(state_root_hash))
}
