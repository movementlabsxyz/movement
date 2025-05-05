use dot_movement::DotMovement;

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
		// write the migrated value
		dot_movement.try_overwrite_config_to_json(&value)?;
	}

	tracing::info!("Migration done.");
	Ok(())
}
