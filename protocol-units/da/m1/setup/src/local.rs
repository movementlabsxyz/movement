use anyhow::{Result, anyhow};
use commander::run_command;
use dot_movement::DotMovement;
use tokio::fs;
use tokio::process::Command;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
    
    pub fn new() -> Self {
        Self
    }

    fn random_hex(bytes: usize) -> String {
        let mut rng = thread_rng();
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
        dot_movement: &DotMovement,
        config : m1_da_light_node_util::Config,
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
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-app")
        );

        // update the node path with the chain id
        config.celestia_node_path.replace(
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-node")
        );

    }

    async fn setup_celestia(
        &self, 
        dot_movement_path: DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<m1_da_light_node_util::Config, anyhow::Error> {

        let dot_movement_path = dot_movement_path.get_path();
        config.celestia_chain_id.get_or_insert(Self::random_chain_id());
        let celestia_chain_id = config.try_celestia_chain_id()?;
        config.celestia_namespace.get_or_insert(Self::random_namespace());
        config.celestia_app_path.replace(
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-app")
        );
        config.celestia_node_path.replace(
            dot_movement_path.join("celestia").join(celestia_chain_id.clone()).join(".celestia-node")
        );

        if self.check_folder_exists(&celestia_app_path).await? {
            println!("The folder {} exists. Deleting it...", celestia_app_path);
            self.delete_folder(&celestia_app_path).await?;
            println!("Folder deleted.");
        } else {
            println!("The folder {} does not exist.", celestia_app_path);
        }

        let coins = "1000000000000000utia";
        run_command("celestia-appd", &["init", &celestia_chain_id, "--chain-id", &celestia_chain_id, "--home", &celestia_app_path]).await?;
        run_command("celestia-appd", &["keys", "add", "validator", "--keyring-backend=test", "--home", &celestia_app_path]).await?;
        let validator_address = self.get_validator_address(&celestia_app_path).await?;
        run_command("celestia-appd", &["add-genesis-account", &validator_address, coins, "--home", &celestia_app_path]).await?;
        run_command("celestia-appd", &["gentx", "validator", "5000000000utia", "--keyring-backend=test", "--chain-id", &celestia_chain_id, "--home", &celestia_app_path]).await?;
        run_command("celestia-appd", &["collect-gentxs", "--home", &celestia_app_path]).await?;

        self.update_config(&celestia_app_path).await?;

        self.copy_keys(&celestia_app_path, &celestia_node_path).await?;

        Ok(())
    }

    async fn check_folder_exists(&self, path: &str) -> Result<bool, anyhow::Error> {
        match fs::metadata(path).await {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(_) => Ok(false),
        }
    }

    async fn delete_folder(&self, path: &str) -> Result<(), anyhow::Error> {
        fs::remove_dir_all(path).await?;
        Ok(())
    }

    async fn get_validator_address(&self, home: &str) -> Result<String, anyhow::Error> {
        let output = Command::new("celestia-appd")
            .args(&["keys", "show", "validator", "-a", "--keyring-backend=test", "--home", home])
            .output()
            .await?;
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)?.trim().to_string())
        } else {
            Err(anyhow!("Failed to get validator address"))
        }
    }

    async fn update_config(&self, home: &str) -> Result<(), anyhow::Error> {
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

    async fn copy_keys(&self, app_path: &str, node_path: &str) -> Result<(), anyhow::Error> {
        let keyring_source = format!("{}/keyring-test/", app_path);
        let keyring_dest = format!("{}/keys/keyring-test/", node_path);

        fs::create_dir_all(&format!("{}/keys", node_path)).await?;
        self.copy_recursive(&keyring_source, &keyring_dest).await?;

        Ok(())
    }

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
        info!("Setting up M1 DA Light Node.");
        self.setup_celestia(
            dot_movement,
            config,
        ).await?;
        // Placeholder for returning the actual configuration.
        Ok(m1_da_light_node_util::Config::default())
    }
}
