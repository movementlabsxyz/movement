use dot_movement::DotMovement;
use serde_json::Value;

pub async fn migrate_v0_4_0(dot_movement: DotMovement) -> Result<(), anyhow::Error> {
	let mut value = dot_movement.try_load_value().await?;

	//verify the da-sequencer conf exist.
	let da_conf = value.get("maptos_config").and_then(|conf| conf.get("da_sequencer"));
	//add default values
	if da_conf.is_none() {
		tracing::info!("No Da-sequencer config, create a new one.");
		let da_config = maptos_execution_util::config::da_sequencer::Config::default();
		if let Some(maptos_conf) =
			value.get_mut("maptos_config").and_then(|val| val.as_object_mut())
		{
			maptos_conf.insert(
				"da_sequencer".to_string(),
				serde_json::to_value(da_config).unwrap_or_default(),
			);
		}
	}

	let da_conf = value
		.get_mut("maptos_config")
		.and_then(|conf| conf.get_mut("da_sequencer"))
		.and_then(|val| val.as_object_mut());

	//set DA sequencer connection url if not local
	let local = std::env::var_os("MAYBE_RUN_LOCAL").unwrap_or("false".into());
	if local == "false" {
		let new_url = match std::env::var_os("MAPTOS_DA_SEQUENCER_CONNECTION_URL") {
			Some(url) => url.to_string_lossy().into_owned(),
			None => "http://movement-da-sequencer:30730/".to_string(),
		};
		tracing::info!("updating da-sequecner connection url with:{new_url}");
		if let Some(conn) = value.pointer_mut("/maptos_config/da_sequencer/connection_url") {
			*conn = Value::String(new_url);
		}
	}

	// write the migrated value
	dot_movement.try_overwrite_config_to_json(&value)?;

	tracing::info!("Migration done.");
	Ok(())
}
