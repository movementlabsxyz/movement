use ed25519_dalek::SigningKey;
use godfig::env_default;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::info;

pub const DASEQUENCER_CONF_FOLDER: &str = "da-sequencer";

fn default_signing_key() -> SigningKey {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);

        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();

        info!(
                "Using batch signing public key: {}",
                hex::encode(verifying_key.to_bytes())
        );

        signing_key
}

pub fn get_config_path(dot_movement: &dot_movement::DotMovement) -> std::path::PathBuf {
        let mut pathbuff = std::path::PathBuf::from(dot_movement.get_path());
        pathbuff.push(DASEQUENCER_CONF_FOLDER);
        pathbuff
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaSequencerConfig {
        #[serde(default = "default_movement_da_sequencer_listen_address")]
        pub movement_da_sequencer_listen_address: SocketAddr,

        #[serde(skip_deserializing, skip_serializing, default = "default_signing_key")]
        pub signing_key: SigningKey,
}

env_default!(
        default_movement_da_sequencer_listen_address,
        "MOVEMENT_DA_SEQUENCER_LISTEN_ADDRESS",
        SocketAddr,
        "0.0.0.0:30730"
                .parse::<SocketAddr>()
                .expect("Bad da sequencer listener address.")
);

impl Default for DaSequencerConfig {
        fn default() -> Self {
                Self {
                        movement_da_sequencer_listen_address: default_movement_da_sequencer_listen_address(),
                        signing_key: default_signing_key(),
                }
        }
}
