use util::{
    service::Service,
    util::util::patterns::constructor::ConstructorOperations,
    util::util::version
};
use artifacts::known_artifacts::m1::m1_with_submodules;

#[derive(Debug, Clone)]
pub struct Config {
    pub evm_sender : String,
    pub faucet_sender : String,
    pub node_url : String,
}

impl Config {

    pub const DEFAULT_EVM_SENDER : &str = "0xf238ff22567c56bdaa18105f229ac0dacc2d9f73dfc5bf08a2a2a4a0fac4d221";
    pub const DEFAULT_FAUCET_SENDER : &str = "0xf238ff22567c56bdaa18105f229ac0dacc2d9f73dfc5bf08a2a2a4a0fac4d221";
    pub const DEFAULT_NODE_URL : &str = "http://testnet.m1.movementlabs.xyz";

}

impl Default for Config {

    fn default() -> Self {

        Self {
            evm_sender : Self::DEFAULT_EVM_SENDER.to_string(),
            faucet_sender : Self::DEFAULT_FAUCET_SENDER.to_string(),
            node_url : Self::DEFAULT_NODE_URL.to_string(),
        }

    }

}

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Service;
    type Config = Config;

    fn default() -> Self::Artifact {

       Self::default_with_version(&version::Version::Latest)

    }

    fn default_with_version(version : &util::util::util::Version) -> Self::Artifact {
        
        Self::from_config(
            version, 
            &Self::Config::default()
        )

    }

    fn from_config(version : &util::util::util::Version, config : &Self::Config) -> Self::Artifact {
        

        Service::foreground(
            "proxy".to_string(), 
            format!(
            r#"
                export EVM_SENDER={}
                export FAUCET_SENDER={}
                export NODE_URL={}
                cd $MOVEMENT_DIR/src/m1-with-submodules/m1/infrastructure/evm-rpc
                npm install
                npm run start
                "#, 
                config.evm_sender, 
                config.faucet_sender, 
                config.node_url
            ), 
            vec![
                m1_with_submodules::Constructor::default_with_version(
                    version
                ).into()
            ]
        )

    }

}
