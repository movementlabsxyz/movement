use util::{
    service::Service,
    util::util::patterns::constructor::ConstructorOperations,
    util::util::version
};
use artifacts::known_artifacts::m1::m1_with_submodules;

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url : String,
    pub subnet_id : String,
}

impl Config {

    pub const BASE_URL : &str = "https://subnet.testnet.m1.movementlabs.xyz/v1";
    pub const SUBNET_ID : &str = "2vUTKYZBbLtXnfCL2RF5XEChZf1wxVYQqxZQQCShMmseSKSiee";

}

impl Default for Config {

    fn default() -> Self {

        Self {
            base_url : Self::BASE_URL.to_string(),
            subnet_id : Self::SUBNET_ID.to_string(),
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
                export BASE_URL={}
                export SUBNET_ID={}
                cd $MOVEMENT_DIR/src/m1-with-submodules/m1/infrastructure/subnet-proxy
                npm install
                npm run start
                "#, 
                config.base_url, 
                config.subnet_id
            ), 
            vec![
                m1_with_submodules::Constructor::default_with_version(
                    version
                ).into()
            ]
        )

    }

}
