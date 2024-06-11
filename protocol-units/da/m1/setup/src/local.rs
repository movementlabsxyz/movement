use commander::run_command;
use dot_movement::DotMovement;
use tokio::fs;
use tracing::info;
use celestia_types::nmt::Namespace;
use crate::M1DaLightNodeSetupOperations;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
    
    pub fn new() -> Self {
        Self
    }

    fn random_hex(bytes: usize) -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..bytes).map(|_| rng.gen()).collect();
        hex::encode(random_bytes)
    }

    fn random_chain_id() -> String {
        Self::random_hex(10)
    }

    fn random_namespace() -> Namespace {
        let namespace_bytes = Self::random_hex(10);
        Namespace::new_v0(&hex::decode(namespace_bytes).unwrap()).unwrap()
    }

    async fn initialize_celestia_config(
        &self, 
        dot_movement: DotMovement,
        mut config : m1_da_light_node_util::Config,
    ) -> Result<m1_da_light_node_util::Config, anyhow::Error>  {

        // use the dot movement path to set up the celestia app and node paths
        let dot_movement_path = dot_movement.get_path();

        // if the celestia chain id is not set, generate a random operations
        config.celestia_chain_id.get_or_insert(Self::random_chain_id());
        let celestia_chain_id = config.try_celestia_chain_id()?;

        // if the celestia namespace is not set, generate a random namespace
        config.celestia_namespace.get_or_insert(Self::random_namespace());

        // update the app path with the chain id
        config.celestia_app_path.replace(
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-app").to_str().ok_or(
                anyhow::anyhow!("Failed to convert path to string.")
            )?.to_string()
        );

        // update the node path with the chain id
        config.celestia_node_path.replace(
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-node").to_str().ok_or(
                anyhow::anyhow!("Failed to convert path to string.")
            )?.to_string()
        );

        Ok(config)

    }

    async fn setup_celestia(
        &self, 
        dot_movement: DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<m1_da_light_node_util::Config, anyhow::Error> {

        let mut config = self.initialize_celestia_config(dot_movement, config).await?;

        // unpack some of the config values
        let celestia_chain_id = config.try_celestia_chain_id()?.to_string().clone();
        let celestia_app_path = config.try_celestia_app_path()?.to_string().clone();
        let celestia_node_path = config.try_celestia_node_path()?.to_string().clone();

        // initialize the celestia app
        info!("Initializing the Celestia App.");
        run_command("celestia-appd", &["init", &celestia_chain_id, "--chain-id", &celestia_chain_id, "--home", &celestia_app_path]).await?;

        // add the validator key
        info!("Adding the validator key.");
        run_command("celestia-appd", &["keys", "add", "validator", "--keyring-backend=test", "--home", &celestia_app_path]).await?;

        // get the validator address
        info!("Getting the validator address.");
        let validator_address = run_command(
            "celestia-appd",
            &[ "keys", "show", "validator", "-a", "--keyring-backend=test", "--home", &celestia_app_path],
        ).await?.trim().to_string();
        config.celestia_validator_address.replace(validator_address.clone());

        // add the genesis account
        info!("Adding the genesis account.");
        let coins = "1000000000000000utia";
        run_command("celestia-appd", &["add-genesis-account", &validator_address, coins, "--home", &celestia_app_path]).await?;

        // create the genesis transaction
        info!("Creating the genesis transaction.");
        run_command("celestia-appd", &["gentx", "validator", "5000000000utia", "--keyring-backend=test", "--chain-id", &celestia_chain_id, "--home", &celestia_app_path]).await?;

        // collect the genesis transactions
        info!("Collecting the genesis transactions.");
        run_command("celestia-appd", &["collect-gentxs", "--home", &celestia_app_path]).await?;

        // updating the celestia node config
        info!("Updating the Celestia Node config.");
        self.update_celestia_node_config(&celestia_app_path).await?;

        info!("Copying keys from Celestia App to Celestia Node.");
        self.copy_keys(&celestia_app_path, &celestia_node_path).await?;

        Ok(config)
    }


    /// Updates the Celestia Node config
    async fn update_celestia_node_config(&self, home: &str) -> Result<(), anyhow::Error> {
        let config_path = format!("{}/config/config.toml", home);
        let sed_commands = [
            ("s#\"tcp://127.0.0.1:26657\"#\"tcp://0.0.0.0:26657\"#g", &config_path),
            ("s/^timeout_commit\\s*=.*/timeout_commit = \"2s\"/g", &config_path),
            ("s/^timeout_propose\\s*=.*/timeout_propose = \"2s\"/g", &config_path),
        ];

        for (command, path) in &sed_commands {
            run_command("sed", &["-i.bak", command, path]).await?;
        }

        Ok(())
    }

    /// Copies keys from Celestia App to Celestia Node
    async fn copy_keys(&self, app_path: &str, node_path: &str) -> Result<(), anyhow::Error> {
        let keyring_source = format!("{}/keyring-test/", app_path);
        let keyring_dest = format!("{}/keys/keyring-test/", node_path);

        fs::create_dir_all(&format!("{}/keys", node_path)).await?;
        self.copy_recursive(&keyring_source, &keyring_dest).await?;

        Ok(())
    }

    /// Recursively copies files from one directory to another
    #[async_recursion::async_recursion]
    async fn copy_recursive(&self, from: &str, to: &str) -> Result<(), anyhow::Error> {
        fs::create_dir_all(to).await?;
        let mut dir = fs::read_dir(from).await?;
        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            let dest_path = format!("{}/{}", to, entry.file_name().to_string_lossy());
            if entry_path.is_dir() {
                self.copy_recursive(&entry_path.to_string_lossy(), &dest_path).await?;
            } else {
                fs::copy(&entry_path, &dest_path).await?;
            }
        }
        Ok(())
    }

}

impl M1DaLightNodeSetupOperations for Local {
    async fn setup(
        &self,
        dot_movement : DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<m1_da_light_node_util::Config, anyhow::Error> {

        info!("Setting up Celestia for M1 DA Light Node.");
        let config = self.setup_celestia(
            dot_movement,
            config,
        ).await?;

        // Placeholder for returning the actual configuration.
        Ok(config)
    }
}
